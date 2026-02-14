//! Property-based fuzz tests: provider JSON -> Universal -> provider JSON (exact match).
//!
//! Generates random provider payloads from OpenAPI specs, converts to Universal
//! and back, and asserts the output JSON exactly matches the input.
//!
//! Two test modes:
//!   `make run`   - fail on first error with verbose output (for debugging)
//!   `make stats` - run all cases and report aggregated summary (for triage)

use coverage_report::runner::diff_json;
use lingua::processing::adapter_for_format;
use lingua::serde_json::{self, Value};
use lingua::ProviderFormat;
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod schema_strategy;

const SNAPSHOT_SUITE_OPENAI: &str = "openai-roundtrip";
const SNAPSHOT_SUITE_ANTHROPIC: &str = "anthropic-roundtrip";
const SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM: &str = "chat-anthropic-two-arm";

fn is_openai_exact_roundtrip_scope(payload: &Value) -> bool {
    let Some(root) = payload.as_object() else {
        return false;
    };

    // Canonicalization differences currently outside strict-equality scope.
    if root.contains_key("max_tokens")
        || root.contains_key("tool_choice")
        || root.contains_key("response_format")
        || root.contains_key("reasoning_effort")
        || root.contains_key("tools")
    {
        return false;
    }

    if root
        .get("stop")
        .is_some_and(|v| matches!(v, Value::String(_)))
    {
        return false;
    }

    if root.get("stream").and_then(Value::as_bool) == Some(true) {
        return false;
    }

    let Some(messages) = root.get("messages").and_then(Value::as_array) else {
        return false;
    };

    for message in messages {
        let Some(msg) = message.as_object() else {
            return false;
        };

        // Message-level fields not yet preserved exactly.
        if msg.contains_key("name")
            || msg.contains_key("audio")
            || msg.contains_key("function_call")
            || msg.contains_key("refusal")
        {
            return false;
        }

        // `developer` collapses to universal `system`, so exact role roundtrip is out of scope.
        if msg.get("role").and_then(Value::as_str) == Some("developer") {
            return false;
        }

        // Assistant messages without content roundtrip back as empty content.
        if msg.get("role").and_then(Value::as_str) == Some("assistant")
            && !msg.contains_key("content")
            && !msg.contains_key("tool_calls")
        {
            return false;
        }
    }

    true
}

fn is_anthropic_exact_roundtrip_scope(payload: &Value) -> bool {
    let Some(root) = payload.as_object() else {
        return false;
    };

    // Canonicalized through universal representation in ways that are not exact.
    if root.contains_key("output_format")
        || root.contains_key("output_config")
        || root.contains_key("thinking")
        || root.contains_key("tool_choice")
        || root.contains_key("tools")
        || root.contains_key("system")
    {
        return false;
    }

    if root.get("stream").and_then(Value::as_bool) == Some(true) {
        return false;
    }

    // Anthropic only preserves metadata.user_id.
    if let Some(metadata) = root.get("metadata") {
        let Some(obj) = metadata.as_object() else {
            return false;
        };
        if obj.len() != 1 || !obj.contains_key("user_id") {
            return false;
        }
        if !obj.get("user_id").is_some_and(Value::is_string) {
            return false;
        }
    }

    let Some(messages) = root.get("messages").and_then(Value::as_array) else {
        return false;
    };

    for message in messages {
        let Some(msg) = message.as_object() else {
            return false;
        };
        if msg.get("role").and_then(Value::as_str).is_none() {
            return false;
        }
        let Some(content) = msg.get("content") else {
            return false;
        };
        match content {
            Value::String(_) => {}
            Value::Array(parts) => {
                // Restrict to text-only blocks for exact roundtrip mode.
                for part in parts {
                    let Some(part_obj) = part.as_object() else {
                        return false;
                    };
                    if part_obj.get("type").and_then(Value::as_str) != Some("text") {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }

    true
}

fn is_chat_to_anthropic_two_arm_scope(payload: &Value) -> bool {
    if !is_openai_exact_roundtrip_scope(payload) {
        return false;
    }

    let Some(root) = payload.as_object() else {
        return false;
    };

    // Parameters unsupported by Anthropic and not expected to roundtrip through it.
    if root.contains_key("frequency_penalty")
        || root.contains_key("presence_penalty")
        || root.contains_key("logprobs")
        || root.contains_key("top_logprobs")
        || root.contains_key("store")
        || root.contains_key("prediction")
        || root.contains_key("functions")
        || root.contains_key("audio")
        || root.contains_key("function_call")
    {
        return false;
    }

    let Some(messages) = root.get("messages").and_then(Value::as_array) else {
        return false;
    };

    for message in messages {
        let Some(msg) = message.as_object() else {
            return false;
        };
        // Keep initial arm focused on text-only, no tool-call payload shape changes.
        if msg.contains_key("tool_calls")
            || msg.get("role").and_then(Value::as_str) == Some("system")
        {
            return false;
        }
    }

    true
}

fn sanitize_chat_for_two_arm_payload(payload: Value) -> Value {
    let mut payload = payload;
    let Some(root) = payload.as_object_mut() else {
        return payload;
    };

    for key in [
        "frequency_penalty",
        "presence_penalty",
        "logprobs",
        "top_logprobs",
        "store",
        "prediction",
        "functions",
        "audio",
        "function_call",
        "logit_bias",
        "modalities",
        "n",
        "stream_options",
        "verbosity",
        "web_search_options",
        "seed",
        "parallel_tool_calls",
    ] {
        root.remove(key);
    }

    // Anthropic path injects max_tokens; seed an equivalent OpenAI field up front.
    if !root.contains_key("max_tokens") && !root.contains_key("max_completion_tokens") {
        root.insert("max_completion_tokens".into(), Value::Number(4096.into()));
    }

    if let Some(messages) = root.get_mut("messages").and_then(Value::as_array_mut) {
        messages.retain_mut(|message| {
            let Some(msg) = message.as_object_mut() else {
                return false;
            };
            for key in [
                "tool_calls",
                "audio",
                "function_call",
                "name",
                "refusal",
                "annotations",
            ] {
                msg.remove(key);
            }

            let role = msg.get("role").and_then(Value::as_str);
            if matches!(role, Some("system" | "developer")) {
                return false;
            }

            if !msg.contains_key("content") || msg.get("content").is_some_and(Value::is_null) {
                msg.insert("content".into(), Value::String(String::new()));
            }

            true
        });

        if messages.is_empty() {
            messages.push(serde_json::json!({
                "role": "user",
                "content": "hello",
            }));
        }
    }

    payload
}

fn sanitize_anthropic_roundtrip_payload(payload: Value) -> Value {
    let mut payload = payload;
    let Some(root) = payload.as_object_mut() else {
        return payload;
    };

    for key in [
        "output_format",
        "output_config",
        "thinking",
        "tool_choice",
        "tools",
        "system",
        "stream",
    ] {
        root.remove(key);
    }

    if let Some(metadata) = root.get_mut("metadata") {
        if let Some(obj) = metadata.as_object_mut() {
            if let Some(user_id) = obj.get_mut("user_id") {
                if user_id.is_null() {
                    *user_id = Value::String("user".to_string());
                }
            } else {
                obj.insert("user_id".into(), Value::String("user".to_string()));
            }
            let keep = obj
                .get("user_id")
                .cloned()
                .unwrap_or(Value::String("user".into()));
            obj.clear();
            obj.insert("user_id".into(), keep);
        } else {
            root.remove("metadata");
        }
    }

    if let Some(messages) = root.get_mut("messages").and_then(Value::as_array_mut) {
        messages.retain_mut(|message| {
            let Some(msg) = message.as_object_mut() else {
                return false;
            };
            if msg.get("role").and_then(Value::as_str).is_none() {
                msg.insert("role".into(), Value::String("user".to_string()));
            }

            if !msg.contains_key("content") || msg.get("content").is_some_and(Value::is_null) {
                msg.insert("content".into(), Value::String("hello".to_string()));
            }

            if let Some(parts) = msg.get_mut("content").and_then(Value::as_array_mut) {
                parts.retain_mut(|part| {
                    let Some(obj) = part.as_object_mut() else {
                        return false;
                    };
                    if obj.get("type").and_then(Value::as_str) != Some("text") {
                        return false;
                    }
                    obj.remove("cache_control");
                    obj.remove("citations");
                    if !obj.get("text").is_some_and(Value::is_string) {
                        obj.insert("text".into(), Value::String("hello".to_string()));
                    }
                    true
                });
                if parts.is_empty() {
                    msg.insert("content".into(), Value::String("hello".to_string()));
                }
            }
            true
        });

        if messages.is_empty() {
            messages.push(serde_json::json!({
                "role": "user",
                "content": "hello",
            }));
        }
    }

    payload
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory should exist")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn fuzz_snapshot_dir_for_suite(suite: &str) -> PathBuf {
    workspace_root().join(format!("payloads/fuzz-snapshots/{suite}"))
}

fn fnv1a_64(input: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn save_failing_snapshot_for_suite(
    suite: &str,
    provider: &str,
    kind: &str,
    payload: &Value,
    issues: &[String],
) -> Result<PathBuf, String> {
    let compact =
        serde_json::to_vec(payload).map_err(|e| format!("failed to serialize payload: {}", e))?;
    let hash = fnv1a_64(&compact);
    let case_id = format!("case-{hash:016x}");

    let dir = fuzz_snapshot_dir_for_suite(suite);
    fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "failed to create fuzz snapshot directory {}: {}",
            dir.display(),
            e
        )
    })?;

    let payload_path = dir.join(format!("{case_id}.request.json"));
    let meta_path = dir.join(format!("{case_id}.meta.json"));

    if !payload_path.exists() {
        let pretty = serde_json::to_string_pretty(payload)
            .map_err(|e| format!("failed to render payload json: {}", e))?
            + "\n";
        fs::write(&payload_path, pretty).map_err(|e| {
            format!(
                "failed to write payload snapshot {}: {}",
                payload_path.display(),
                e
            )
        })?;
    }

    let meta = serde_json::json!({
        "provider": provider,
        "kind": kind,
        "issues": issues,
    });
    let meta_pretty = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("failed to render meta json: {}", e))?
        + "\n";
    fs::write(&meta_path, meta_pretty).map_err(|e| {
        format!(
            "failed to write meta snapshot {}: {}",
            meta_path.display(),
            e
        )
    })?;

    Ok(payload_path)
}

fn snapshot_request_paths_for_suite(suite: &str) -> Vec<PathBuf> {
    let dir = fuzz_snapshot_dir_for_suite(suite);
    if !dir.exists() {
        return Vec::new();
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", dir.display(), e))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with(".request.json"))
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    entries.sort();
    entries
}

fn snapshot_case_id(path: &Path, suffix: &str) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    name.strip_suffix(suffix).map(str::to_string)
}

fn snapshot_meta_path(request_path: &Path) -> PathBuf {
    let name = request_path
        .file_name()
        .and_then(|s| s.to_str())
        .expect("request snapshot filename should be valid utf-8");
    request_path.with_file_name(name.replace(".request.json", ".meta.json"))
}

fn delete_snapshot_pair(request_path: &Path) {
    let _ = fs::remove_file(request_path);
    let _ = fs::remove_file(snapshot_meta_path(request_path));
}

fn canonical_issue_signature(issues: &[String]) -> String {
    let normalized: BTreeSet<String> = issues
        .iter()
        .map(|issue| normalize_issue_for_signature(issue))
        .collect();
    normalized.into_iter().collect::<Vec<_>>().join(" | ")
}

fn normalize_path_indices(path: &str) -> String {
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0usize;
    let mut out = String::new();
    while i < chars.len() {
        if chars[i] == '[' {
            let mut j = i + 1;
            let mut has_digits = false;
            while j < chars.len() && chars[j].is_ascii_digit() {
                has_digits = true;
                j += 1;
            }
            if has_digits && j < chars.len() && chars[j] == ']' {
                out.push_str("[*]");
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn normalize_issue_for_signature(issue: &str) -> String {
    let trimmed = issue.trim();
    let without_values = if trimmed.starts_with("changed: ") {
        if let Some(idx) = trimmed.find(" (") {
            &trimmed[..idx]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    if let Some(path) = without_values.strip_prefix("lost: ") {
        return format!("lost: {}", normalize_path_indices(path));
    }
    if let Some(path) = without_values.strip_prefix("added: ") {
        return format!("added: {}", normalize_path_indices(path));
    }
    if let Some(path) = without_values.strip_prefix("changed: ") {
        return format!("changed: {}", normalize_path_indices(path));
    }

    normalize_path_indices(without_values)
}

fn load_meta_issues(request_path: &Path) -> Option<Vec<String>> {
    let meta_path = snapshot_meta_path(request_path);
    let raw = fs::read_to_string(meta_path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let issues = value.get("issues")?.as_array()?;
    let parsed: Vec<String> = issues
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn prune_orphan_meta_files_for_suite(suite: &str) -> usize {
    let dir = fuzz_snapshot_dir_for_suite(suite);
    if !dir.exists() {
        return 0;
    }

    let mut request_cases = BTreeSet::new();
    let mut meta_paths = Vec::new();

    for entry in
        fs::read_dir(&dir).unwrap_or_else(|e| panic!("failed to read {}: {}", dir.display(), e))
    {
        let path = match entry {
            Ok(e) => e.path(),
            Err(_) => continue,
        };
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.ends_with(".request.json") {
            if let Some(case_id) = snapshot_case_id(&path, ".request.json") {
                request_cases.insert(case_id);
            }
        } else if name.ends_with(".meta.json") {
            meta_paths.push(path);
        }
    }

    let mut removed = 0usize;
    for meta_path in meta_paths {
        let Some(case_id) = snapshot_case_id(&meta_path, ".meta.json") else {
            continue;
        };
        if !request_cases.contains(&case_id) {
            if fs::remove_file(&meta_path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

// ============================================================================
// Roundtrip assertion
// ============================================================================

/// Provider JSON -> Universal -> Provider JSON.
/// Returns a list of diff descriptions, or empty if exact match.
/// Returns None if the payload was rejected by request_to_universal (skip).
fn assert_provider_roundtrip(format: ProviderFormat, payload: &Value) -> Option<Vec<String>> {
    if format == ProviderFormat::ChatCompletions && !is_openai_exact_roundtrip_scope(payload) {
        return None;
    }
    if format == ProviderFormat::Anthropic && !is_anthropic_exact_roundtrip_scope(payload) {
        return None;
    }

    let adapter = adapter_for_format(format)?;

    let universal = adapter.request_to_universal(payload.clone()).ok()?;
    let output = match adapter.request_from_universal(&universal) {
        Ok(o) => o,
        Err(e) => return Some(vec![format!("request_from_universal error: {}", e)]),
    };

    if *payload == output {
        return Some(vec![]);
    }

    let diff = diff_json(payload, &output);
    let mut issues = Vec::new();
    for f in &diff.lost_fields {
        issues.push(format!("lost: {}", f));
    }
    for f in &diff.added_fields {
        issues.push(format!("added: {}", f));
    }
    for (f, before, after) in &diff.changed_fields {
        issues.push(format!("changed: {} ({} -> {})", f, before, after));
    }
    Some(issues)
}

/// Verbose roundtrip check for debugging. Returns full error string on failure.
fn assert_provider_roundtrip_verbose(
    format: ProviderFormat,
    payload: &Value,
) -> Result<bool, String> {
    if format == ProviderFormat::ChatCompletions && !is_openai_exact_roundtrip_scope(payload) {
        return Ok(false);
    }
    if format == ProviderFormat::Anthropic && !is_anthropic_exact_roundtrip_scope(payload) {
        return Ok(false);
    }

    let adapter =
        adapter_for_format(format).ok_or_else(|| format!("No adapter for {:?}", format))?;

    let universal = match adapter.request_to_universal(payload.clone()) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };

    let output = adapter
        .request_from_universal(&universal)
        .map_err(|e| format!("request_from_universal({:?}): {}", format, e))?;

    if *payload == output {
        return Ok(true);
    }

    let diff = diff_json(payload, &output);
    let mut issues = Vec::new();
    for f in &diff.lost_fields {
        issues.push(format!("  lost: {}", f));
    }
    for f in &diff.added_fields {
        issues.push(format!("  added: {}", f));
    }
    for (f, before, after) in &diff.changed_fields {
        issues.push(format!("  changed: {} ({} -> {})", f, before, after));
    }

    Err(format!(
        "{:?} roundtrip mismatch:\n{}\n\nInput:  {}\nOutput: {}",
        format,
        issues.join("\n"),
        serde_json::to_string_pretty(payload).unwrap(),
        serde_json::to_string_pretty(&output).unwrap(),
    ))
}

fn append_diff_issues(prefix: &str, before: &Value, after: &Value, issues: &mut Vec<String>) {
    if before == after {
        return;
    }
    let diff = diff_json(before, after);
    for f in &diff.lost_fields {
        issues.push(format!("{prefix} lost: {f}"));
    }
    for f in &diff.added_fields {
        issues.push(format!("{prefix} added: {f}"));
    }
    for (f, before, after) in &diff.changed_fields {
        issues.push(format!("{prefix} changed: {f} ({before} -> {after})"));
    }
}

fn as_pretty_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "<serialization failed>".to_string())
}

fn assert_chat_anthropic_two_arm(payload: &Value) -> Option<Vec<String>> {
    if !is_chat_to_anthropic_two_arm_scope(payload) {
        return None;
    }

    let chat = adapter_for_format(ProviderFormat::ChatCompletions)?;
    let anthropic = adapter_for_format(ProviderFormat::Anthropic)?;

    let universal_1 = chat.request_to_universal(payload.clone()).ok()?;
    let anthropic_1 = match anthropic.request_from_universal(&universal_1) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("chat->anthropic error: {e}")]),
    };
    let universal_2 = match anthropic.request_to_universal(anthropic_1.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("anthropic->universal(1) error: {e}")]),
    };

    let anthropic_2 = match anthropic.request_from_universal(&universal_2) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->anthropic(2) error: {e}")]),
    };
    let universal_3 = match anthropic.request_to_universal(anthropic_2.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("anthropic->universal(2) error: {e}")]),
    };
    let chat_out = match chat.request_from_universal(&universal_3) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->chat error: {e}")]),
    };

    let mut issues = Vec::new();

    let universal_1_json = serde_json::to_value(&universal_1).unwrap_or(Value::Null);
    let universal_2_json = serde_json::to_value(&universal_2).unwrap_or(Value::Null);
    append_diff_issues(
        "universal(1->2):",
        &universal_1_json,
        &universal_2_json,
        &mut issues,
    );
    append_diff_issues("anthropic(1->2):", &anthropic_1, &anthropic_2, &mut issues);
    append_diff_issues("chat(final):", payload, &chat_out, &mut issues);

    Some(issues)
}

fn assert_chat_anthropic_two_arm_verbose(payload: &Value) -> Result<bool, String> {
    if !is_chat_to_anthropic_two_arm_scope(payload) {
        return Ok(false);
    }

    let chat = adapter_for_format(ProviderFormat::ChatCompletions)
        .ok_or_else(|| "No chat-completions adapter".to_string())?;
    let anthropic = adapter_for_format(ProviderFormat::Anthropic)
        .ok_or_else(|| "No anthropic adapter".to_string())?;

    let universal_1 = chat
        .request_to_universal(payload.clone())
        .map_err(|e| format!("chat->universal error: {e}"))?;
    let anthropic_1 = anthropic
        .request_from_universal(&universal_1)
        .map_err(|e| format!("universal->anthropic(1) error: {e}"))?;
    let universal_2 = anthropic
        .request_to_universal(anthropic_1.clone())
        .map_err(|e| format!("anthropic->universal(1) error: {e}"))?;
    let anthropic_2 = anthropic
        .request_from_universal(&universal_2)
        .map_err(|e| format!("universal->anthropic(2) error: {e}"))?;
    let universal_3 = anthropic
        .request_to_universal(anthropic_2.clone())
        .map_err(|e| format!("anthropic->universal(2) error: {e}"))?;
    let chat_out = chat
        .request_from_universal(&universal_3)
        .map_err(|e| format!("universal->chat error: {e}"))?;

    let mut issues = Vec::new();
    let universal_1_json = serde_json::to_value(&universal_1).unwrap_or(Value::Null);
    let universal_2_json = serde_json::to_value(&universal_2).unwrap_or(Value::Null);
    append_diff_issues(
        "universal(1->2):",
        &universal_1_json,
        &universal_2_json,
        &mut issues,
    );
    append_diff_issues("anthropic(1->2):", &anthropic_1, &anthropic_2, &mut issues);
    append_diff_issues("chat(final):", payload, &chat_out, &mut issues);

    if issues.is_empty() {
        return Ok(true);
    }

    Err(format!(
        "chat->universal->anthropic->universal->anthropic->universal->chat mismatch:\n{}\n\n\
         chat_input: {}\n\
         universal_1: {}\n\
         anthropic_1: {}\n\
         universal_2: {}\n\
         anthropic_2: {}\n\
         universal_3: {}\n\
         chat_output: {}",
        issues
            .iter()
            .map(|i| format!("  {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        as_pretty_json(payload),
        as_pretty_json(&universal_1),
        as_pretty_json(&anthropic_1),
        as_pretty_json(&universal_2),
        as_pretty_json(&anthropic_2),
        as_pretty_json(&universal_3),
        as_pretty_json(&chat_out),
    ))
}

fn assert_openai_roundtrip(payload: &Value) -> Option<Vec<String>> {
    assert_provider_roundtrip(ProviderFormat::ChatCompletions, payload)
}

fn assert_openai_roundtrip_verbose(payload: &Value) -> Result<bool, String> {
    assert_provider_roundtrip_verbose(ProviderFormat::ChatCompletions, payload)
}

fn assert_anthropic_roundtrip(payload: &Value) -> Option<Vec<String>> {
    assert_provider_roundtrip(ProviderFormat::Anthropic, payload)
}

fn assert_anthropic_roundtrip_verbose(payload: &Value) -> Result<bool, String> {
    assert_provider_roundtrip_verbose(ProviderFormat::Anthropic, payload)
}

// ============================================================================
// Strategies
// ============================================================================

mod strategies {
    use super::schema_strategy::{load_openapi_definitions, strategy_for_schema_name};
    use super::*;

    fn specs_dir() -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        format!("{}/../..", manifest_dir)
    }

    pub fn arb_openai_payload() -> BoxedStrategy<Value> {
        let defs = load_openapi_definitions(&format!("{}/specs/openai/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateChatCompletionRequest", &defs)
    }

    pub fn arb_openai_two_arm_payload() -> BoxedStrategy<Value> {
        arb_openai_payload()
            .prop_map(sanitize_chat_for_two_arm_payload)
            .prop_filter(
                "payload must be in chat-anthropic two-arm scope",
                |payload| is_chat_to_anthropic_two_arm_scope(payload),
            )
            .boxed()
    }

    pub fn arb_anthropic_payload() -> BoxedStrategy<Value> {
        let defs =
            load_openapi_definitions(&format!("{}/specs/anthropic/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateMessageParams", &defs)
    }

    pub fn arb_anthropic_roundtrip_payload() -> BoxedStrategy<Value> {
        arb_anthropic_payload()
            .prop_map(sanitize_anthropic_roundtrip_payload)
            .prop_filter(
                "payload must be in anthropic exact-roundtrip scope",
                |payload| is_anthropic_exact_roundtrip_scope(payload),
            )
            .boxed()
    }
}

// ============================================================================
// Fuzz tests
// ============================================================================

const CASES: u32 = 256;

fn run_saved_snapshots_suite(suite: &str, assert_verbose: fn(&Value) -> Result<bool, String>) {
    for path in snapshot_request_paths_for_suite(suite) {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read snapshot {}: {}", path.display(), e));
        let payload: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("invalid json in {}: {}", path.display(), e));

        match assert_verbose(&payload) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(e) => panic!("snapshot {} failed roundtrip:\n{}", path.display(), e),
        }
    }
}

fn run_prune_snapshots_suite(suite: &str, assert_fn: fn(&Value) -> Option<Vec<String>>) {
    let mut iterations = 0usize;
    let mut removed_malformed = 0usize;
    let mut removed_orphan_meta = 0usize;
    let mut removed_duplicate_reason = 0usize;

    loop {
        iterations += 1;
        let mut changed = false;

        let orphan_removed = prune_orphan_meta_files_for_suite(suite);
        if orphan_removed > 0 {
            removed_orphan_meta += orphan_removed;
            changed = true;
        }

        let mut kept_by_reason: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in snapshot_request_paths_for_suite(suite) {
            let raw = match fs::read_to_string(&path) {
                Ok(v) => v,
                Err(_) => {
                    delete_snapshot_pair(&path);
                    removed_malformed += 1;
                    changed = true;
                    continue;
                }
            };

            let payload: Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => {
                    delete_snapshot_pair(&path);
                    removed_malformed += 1;
                    changed = true;
                    continue;
                }
            };

            let issues = if let Some(meta_issues) = load_meta_issues(&path) {
                meta_issues
            } else {
                match assert_fn(&payload) {
                    Some(issues) if !issues.is_empty() => issues,
                    _ => continue,
                }
            };

            let signature = canonical_issue_signature(&issues);
            if kept_by_reason.contains_key(&signature) {
                delete_snapshot_pair(&path);
                removed_duplicate_reason += 1;
                changed = true;
            } else {
                kept_by_reason.insert(signature, path);
            }
        }

        if !changed {
            break;
        }

        if iterations > 20 {
            panic!("prune did not converge after 20 iterations");
        }
    }

    eprintln!(
        "{suite} prune complete after {} iterations: removed_malformed={} removed_orphan_meta={} removed_duplicate_reason={}",
        iterations,
        removed_malformed,
        removed_orphan_meta,
        removed_duplicate_reason
    );
}

fn run_fail_fast_suite(
    suite: &str,
    provider: &str,
    kind: &str,
    strategy: BoxedStrategy<Value>,
    assert_fn: fn(&Value) -> Option<Vec<String>>,
    assert_verbose: fn(&Value) -> Result<bool, String>,
) {
    let config = ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    proptest!(config, |(payload in strategy.clone())| {
        if let Some(issues) = assert_fn(&payload) {
            if !issues.is_empty() {
                let snapshot_msg = match save_failing_snapshot_for_suite(
                    suite,
                    provider,
                    kind,
                    &payload,
                    &issues,
                ) {
                    Ok(path) => format!("\nSaved failing snapshot: {}", path.display()),
                    Err(err) => format!("\nFailed to save snapshot: {}", err),
                };
                let verbose = assert_verbose(&payload)
                    .err()
                    .unwrap_or_else(|| "roundtrip mismatch (verbose details unavailable)".to_string());
                prop_assert!(false, "{}{}", verbose, snapshot_msg);
            }
        }
    });
}

fn run_stats_suite(
    suite: &str,
    provider: &str,
    kind: &str,
    label: &str,
    strategy: BoxedStrategy<Value>,
    assert_fn: fn(&Value) -> Option<Vec<String>>,
) {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    });

    let mut failures: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut errored = 0usize;

    for _ in 0..CASES {
        let value_tree = match strategy.new_tree(&mut runner) {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let payload = value_tree.current();

        match assert_fn(&payload) {
            None => skipped += 1,
            Some(issues) if issues.is_empty() => passed += 1,
            Some(issues) => {
                errored += 1;
                if let Ok(path) =
                    save_failing_snapshot_for_suite(suite, provider, kind, &payload, &issues)
                {
                    eprintln!("saved failing snapshot: {}", path.display());
                }
                for issue in issues {
                    let entry = failures
                        .entry(issue)
                        .or_insert_with(|| (0, serde_json::to_string_pretty(&payload).unwrap()));
                    entry.0 += 1;
                }
            }
        }
    }

    eprintln!(
        "\n--- {}: {} passed, {} failed, {} skipped (of {}) ---",
        label, passed, errored, skipped, CASES,
    );

    if !failures.is_empty() {
        eprintln!("\nUnique issues ({}):\n", failures.len());
        for (issue, (count, example)) in &failures {
            eprintln!("  [{}x] {}", count, issue);
            eprintln!("    example: {}\n", example.lines().next().unwrap_or(""));
        }
        panic!(
            "{} unique roundtrip issues found across {} cases",
            failures.len(),
            CASES,
        );
    }
}

#[test]
fn openai_roundtrip_saved_snapshots() {
    run_saved_snapshots_suite(SNAPSHOT_SUITE_OPENAI, assert_openai_roundtrip_verbose);
}

#[test]
fn anthropic_roundtrip_saved_snapshots() {
    run_saved_snapshots_suite(SNAPSHOT_SUITE_ANTHROPIC, assert_anthropic_roundtrip_verbose);
}

#[test]
fn chat_anthropic_two_arm_saved_snapshots() {
    run_saved_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        assert_chat_anthropic_two_arm_verbose,
    );
}

/// Prune fuzz snapshots in a loop until stable:
/// - remove malformed request/meta pairs
/// - remove orphan meta files
/// - dedupe snapshots that fail for the same reason
///
/// Conservative by default: keeps snapshots that pass or are out-of-scope.
#[test]
#[ignore]
fn openai_roundtrip_prune_snapshots() {
    run_prune_snapshots_suite(SNAPSHOT_SUITE_OPENAI, assert_openai_roundtrip);
}

#[test]
#[ignore]
fn anthropic_roundtrip_prune_snapshots() {
    run_prune_snapshots_suite(SNAPSHOT_SUITE_ANTHROPIC, assert_anthropic_roundtrip);
}

#[test]
#[ignore]
fn chat_anthropic_two_arm_prune_snapshots() {
    run_prune_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        assert_chat_anthropic_two_arm,
    );
}

/// Fail on the first error with verbose output (input, output, diff).
/// Use for debugging a specific issue.
#[test]
#[ignore]
fn openai_roundtrip() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_OPENAI,
        "chat-completions",
        "request-roundtrip",
        strategies::arb_openai_payload(),
        assert_openai_roundtrip,
        assert_openai_roundtrip_verbose,
    );
}

#[test]
#[ignore]
fn anthropic_roundtrip() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_ANTHROPIC,
        "anthropic",
        "request-roundtrip",
        strategies::arb_anthropic_roundtrip_payload(),
        assert_anthropic_roundtrip,
        assert_anthropic_roundtrip_verbose,
    );
}

#[test]
#[ignore]
fn chat_anthropic_two_arm() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        "chat-completions",
        "chat-anthropic-two-arm",
        strategies::arb_openai_two_arm_payload(),
        assert_chat_anthropic_two_arm,
        assert_chat_anthropic_two_arm_verbose,
    );
}

/// Run all cases and report an aggregated summary of unique issues.
/// Use for triaging the full scope of failures.
#[test]
#[ignore]
fn openai_roundtrip_stats() {
    run_stats_suite(
        SNAPSHOT_SUITE_OPENAI,
        "chat-completions",
        "request-roundtrip",
        "OpenAI roundtrip fuzz",
        strategies::arb_openai_payload(),
        assert_openai_roundtrip,
    );
}

#[test]
#[ignore]
fn anthropic_roundtrip_stats() {
    run_stats_suite(
        SNAPSHOT_SUITE_ANTHROPIC,
        "anthropic",
        "request-roundtrip",
        "Anthropic roundtrip fuzz",
        strategies::arb_anthropic_roundtrip_payload(),
        assert_anthropic_roundtrip,
    );
}

#[test]
#[ignore]
fn chat_anthropic_two_arm_stats() {
    run_stats_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        "chat-completions",
        "chat-anthropic-two-arm",
        "Chat->Anthropic two-arm fuzz",
        strategies::arb_openai_two_arm_payload(),
        assert_chat_anthropic_two_arm,
    );
}
