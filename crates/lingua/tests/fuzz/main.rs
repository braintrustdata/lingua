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
const SNAPSHOT_SUITE_RESPONSES: &str = "responses-roundtrip";
const SNAPSHOT_SUITE_ANTHROPIC: &str = "anthropic-roundtrip";
const SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM: &str = "chat-anthropic-two-arm";
const SNAPSHOT_SUITE_CHAT_RESPONSES_ANTHROPIC_THREE_ARM: &str =
    "chat-responses-anthropic-three-arm";

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

fn update_snapshot_meta_issues(request_path: &Path, issues: &[String]) {
    let meta_path = snapshot_meta_path(request_path);
    let mut meta_value = fs::read_to_string(&meta_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    if !meta_value.is_object() {
        meta_value = serde_json::json!({});
    }

    if let Some(meta_obj) = meta_value.as_object_mut() {
        let issue_values = issues
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        meta_obj.insert("issues".to_string(), Value::Array(issue_values));
    }

    if let Ok(pretty) = serde_json::to_string_pretty(&meta_value) {
        let _ = fs::write(meta_path, pretty + "\n");
    }
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
    if let Some(idx) = trimmed.find("Tool '") {
        let suffix = "' of type 'custom' is not supported by anthropic";
        if let Some(end) = trimmed[idx + 6..].find(suffix) {
            let prefix = &trimmed[..idx];
            let after = idx + 6 + end;
            let rest = &trimmed[after..];
            return format!("{prefix}Tool '<custom>'{rest}");
        }
    }

    for marker in ["changed: ", "lost: ", "added: "] {
        if let Some(idx) = trimmed.find(marker) {
            let prefix = &trimmed[..idx];
            let body = &trimmed[idx + marker.len()..];
            let path = if marker == "changed: " {
                body.split_once(" (").map_or(body, |(left, _)| left)
            } else {
                body
            };
            return format!("{prefix}{marker}{}", normalize_path_indices(path.trim()));
        }
    }

    normalize_path_indices(trimmed)
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
        if !request_cases.contains(&case_id) && fs::remove_file(&meta_path).is_ok() {
            removed += 1;
        }
    }
    removed
}

// ============================================================================
// Roundtrip assertion
// ============================================================================

/// Provider JSON -> Universal -> Provider JSON.
/// Returns a list of diff descriptions, or empty if exact match.
/// Returns None only when no adapter exists for the provider format.
fn assert_provider_roundtrip(format: ProviderFormat, payload: &Value) -> Option<Vec<String>> {
    let adapter = adapter_for_format(format)?;

    let universal = match adapter.request_to_universal(payload.clone()) {
        Ok(u) => u,
        Err(e) => return Some(vec![format!("request_to_universal error: {}", e)]),
    };
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
    let adapter =
        adapter_for_format(format).ok_or_else(|| format!("No adapter for {:?}", format))?;

    let universal = adapter
        .request_to_universal(payload.clone())
        .map_err(|e| format!("request_to_universal({:?}): {}", format, e))?;

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
    let chat = adapter_for_format(ProviderFormat::ChatCompletions)?;
    let anthropic = adapter_for_format(ProviderFormat::Anthropic)?;

    let universal_1 = match chat.request_to_universal(payload.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("chat->universal error: {e}")]),
    };
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

fn assert_responses_roundtrip(payload: &Value) -> Option<Vec<String>> {
    assert_provider_roundtrip(ProviderFormat::Responses, payload)
}

fn assert_responses_roundtrip_verbose(payload: &Value) -> Result<bool, String> {
    assert_provider_roundtrip_verbose(ProviderFormat::Responses, payload)
}

fn assert_chat_responses_anthropic_three_arm(payload: &Value) -> Option<Vec<String>> {
    let chat = adapter_for_format(ProviderFormat::ChatCompletions)?;
    let responses = adapter_for_format(ProviderFormat::Responses)?;
    let anthropic = adapter_for_format(ProviderFormat::Anthropic)?;

    let universal_1 = match chat.request_to_universal(payload.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("chat->universal error: {e}")]),
    };
    let responses_1 = match responses.request_from_universal(&universal_1) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->responses(1) error: {e}")]),
    };
    let universal_2 = match responses.request_to_universal(responses_1.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("responses->universal(1) error: {e}")]),
    };
    let anthropic_1 = match anthropic.request_from_universal(&universal_2) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->anthropic(1) error: {e}")]),
    };
    let universal_3 = match anthropic.request_to_universal(anthropic_1.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("anthropic->universal(1) error: {e}")]),
    };
    let responses_2 = match responses.request_from_universal(&universal_3) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->responses(2) error: {e}")]),
    };
    let universal_4 = match responses.request_to_universal(responses_2.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("responses->universal(2) error: {e}")]),
    };
    let anthropic_2 = match anthropic.request_from_universal(&universal_4) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->anthropic(2) error: {e}")]),
    };
    let universal_5 = match anthropic.request_to_universal(anthropic_2.clone()) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("anthropic->universal(2) error: {e}")]),
    };
    let chat_out = match chat.request_from_universal(&universal_5) {
        Ok(v) => v,
        Err(e) => return Some(vec![format!("universal->chat error: {e}")]),
    };

    let mut issues = Vec::new();
    let universal_1_json = serde_json::to_value(&universal_1).unwrap_or(Value::Null);
    let universal_2_json = serde_json::to_value(&universal_2).unwrap_or(Value::Null);
    let universal_3_json = serde_json::to_value(&universal_3).unwrap_or(Value::Null);
    append_diff_issues(
        "universal(1->2):",
        &universal_1_json,
        &universal_2_json,
        &mut issues,
    );
    append_diff_issues(
        "universal(2->3):",
        &universal_2_json,
        &universal_3_json,
        &mut issues,
    );
    append_diff_issues("responses(1->2):", &responses_1, &responses_2, &mut issues);
    append_diff_issues("anthropic(1->2):", &anthropic_1, &anthropic_2, &mut issues);
    append_diff_issues("chat(final):", payload, &chat_out, &mut issues);

    Some(issues)
}

fn assert_chat_responses_anthropic_three_arm_verbose(payload: &Value) -> Result<bool, String> {
    let chat = adapter_for_format(ProviderFormat::ChatCompletions)
        .ok_or_else(|| "No chat-completions adapter".to_string())?;
    let responses = adapter_for_format(ProviderFormat::Responses)
        .ok_or_else(|| "No responses adapter".to_string())?;
    let anthropic = adapter_for_format(ProviderFormat::Anthropic)
        .ok_or_else(|| "No anthropic adapter".to_string())?;

    let universal_1 = chat
        .request_to_universal(payload.clone())
        .map_err(|e| format!("chat->universal error: {e}"))?;
    let responses_1 = responses
        .request_from_universal(&universal_1)
        .map_err(|e| format!("universal->responses(1) error: {e}"))?;
    let universal_2 = responses
        .request_to_universal(responses_1.clone())
        .map_err(|e| format!("responses->universal(1) error: {e}"))?;
    let anthropic_1 = anthropic
        .request_from_universal(&universal_2)
        .map_err(|e| format!("universal->anthropic(1) error: {e}"))?;
    let universal_3 = anthropic
        .request_to_universal(anthropic_1.clone())
        .map_err(|e| format!("anthropic->universal(1) error: {e}"))?;
    let responses_2 = responses
        .request_from_universal(&universal_3)
        .map_err(|e| format!("universal->responses(2) error: {e}"))?;
    let universal_4 = responses
        .request_to_universal(responses_2.clone())
        .map_err(|e| format!("responses->universal(2) error: {e}"))?;
    let anthropic_2 = anthropic
        .request_from_universal(&universal_4)
        .map_err(|e| format!("universal->anthropic(2) error: {e}"))?;
    let universal_5 = anthropic
        .request_to_universal(anthropic_2.clone())
        .map_err(|e| format!("anthropic->universal(2) error: {e}"))?;
    let chat_out = chat
        .request_from_universal(&universal_5)
        .map_err(|e| format!("universal->chat error: {e}"))?;

    let mut issues = Vec::new();
    let universal_1_json = serde_json::to_value(&universal_1).unwrap_or(Value::Null);
    let universal_2_json = serde_json::to_value(&universal_2).unwrap_or(Value::Null);
    let universal_3_json = serde_json::to_value(&universal_3).unwrap_or(Value::Null);
    append_diff_issues(
        "universal(1->2):",
        &universal_1_json,
        &universal_2_json,
        &mut issues,
    );
    append_diff_issues(
        "universal(2->3):",
        &universal_2_json,
        &universal_3_json,
        &mut issues,
    );
    append_diff_issues("responses(1->2):", &responses_1, &responses_2, &mut issues);
    append_diff_issues("anthropic(1->2):", &anthropic_1, &anthropic_2, &mut issues);
    append_diff_issues("chat(final):", payload, &chat_out, &mut issues);

    if issues.is_empty() {
        return Ok(true);
    }

    Err(format!(
        "chat->universal->responses->universal->anthropic->universal->responses->universal->anthropic->universal->chat mismatch:\n{}\n\n\
         chat_input: {}\n\
         universal_1: {}\n\
         responses_1: {}\n\
         universal_2: {}\n\
         anthropic_1: {}\n\
         universal_3: {}\n\
         responses_2: {}\n\
         universal_4: {}\n\
         anthropic_2: {}\n\
         universal_5: {}\n\
         chat_output: {}",
        issues
            .iter()
            .map(|i| format!("  {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        as_pretty_json(payload),
        as_pretty_json(&universal_1),
        as_pretty_json(&responses_1),
        as_pretty_json(&universal_2),
        as_pretty_json(&anthropic_1),
        as_pretty_json(&universal_3),
        as_pretty_json(&responses_2),
        as_pretty_json(&universal_4),
        as_pretty_json(&anthropic_2),
        as_pretty_json(&universal_5),
        as_pretty_json(&chat_out),
    ))
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

    pub fn arb_anthropic_payload() -> BoxedStrategy<Value> {
        let defs =
            load_openapi_definitions(&format!("{}/specs/anthropic/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateMessageParams", &defs)
            .prop_filter("payload must parse as Anthropic params", |payload| {
                lingua::providers::anthropic::try_parse_anthropic(payload).is_ok()
            })
            .boxed()
    }

    pub fn arb_responses_payload() -> BoxedStrategy<Value> {
        let defs = load_openapi_definitions(&format!("{}/specs/openai/openapi.yml", specs_dir()));
        strategy_for_schema_name("CreateResponse", &defs)
            .prop_filter("payload must parse as OpenAI Responses params", |payload| {
                lingua::providers::openai::try_parse_responses(payload).is_ok()
            })
            .prop_filter(
                "payload must include model for universal->responses conversion",
                |payload| payload.get("model").and_then(Value::as_str).is_some(),
            )
            .boxed()
    }
}

// ============================================================================
// Fuzz tests
// ============================================================================

const CASES: u32 = 256;

fn run_saved_snapshots_suite(suite: &str, assert_fn: fn(&Value) -> Option<Vec<String>>) {
    for path in snapshot_request_paths_for_suite(suite) {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read snapshot {}: {}", path.display(), e));
        let payload: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("invalid json in {}: {}", path.display(), e));

        match assert_fn(&payload) {
            Some(issues) if !issues.is_empty() => {}
            Some(_) => panic!(
                "snapshot {} no longer reproduces a failure; run prune for this suite",
                path.display()
            ),
            None => panic!(
                "snapshot {} is no longer valid input for this suite",
                path.display()
            ),
        }
    }
}

fn run_prune_snapshots_suite(suite: &str, assert_fn: fn(&Value) -> Option<Vec<String>>) {
    let mut iterations = 0usize;
    let mut removed_malformed = 0usize;
    let mut removed_resolved = 0usize;
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

        let mut covered_issue_types: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut kept_snapshot_paths: BTreeSet<PathBuf> = BTreeSet::new();
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

            let issues = match assert_fn(&payload) {
                Some(issues) if !issues.is_empty() => issues,
                _ => {
                    delete_snapshot_pair(&path);
                    removed_resolved += 1;
                    changed = true;
                    continue;
                }
            };

            let normalized: BTreeSet<String> = issues
                .iter()
                .map(|issue| normalize_issue_for_signature(issue))
                .collect();

            let contributes_new_issue = normalized
                .iter()
                .any(|issue_type| !covered_issue_types.contains_key(issue_type));

            if !contributes_new_issue {
                delete_snapshot_pair(&path);
                removed_duplicate_reason += 1;
                changed = true;
            } else {
                update_snapshot_meta_issues(&path, &issues);
                kept_snapshot_paths.insert(path.clone());
                for issue_type in normalized {
                    covered_issue_types
                        .entry(issue_type)
                        .or_insert_with(|| path.clone());
                }
            }
        }
        if !changed {
            break;
        }

        if iterations > 20 {
            panic!("prune did not converge after 20 iterations");
        }
    }

    let final_paths = snapshot_request_paths_for_suite(suite);
    let final_kept_snapshots = final_paths.len();
    let mut final_issue_types_set = BTreeSet::new();
    for path in &final_paths {
        let raw = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let payload: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(issues) = assert_fn(&payload) {
            for issue in issues {
                final_issue_types_set.insert(normalize_issue_for_signature(&issue));
            }
        }
    }
    let final_issue_types = final_issue_types_set.len();

    eprintln!(
        "{suite} prune complete after {} iterations: kept_snapshots={} issue_types={} removed_malformed={} removed_resolved={} removed_orphan_meta={} removed_duplicate_reason={}",
        iterations,
        final_kept_snapshots,
        final_issue_types,
        removed_malformed,
        removed_resolved,
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
    run_saved_snapshots_suite(SNAPSHOT_SUITE_OPENAI, assert_openai_roundtrip);
}

#[test]
fn anthropic_roundtrip_saved_snapshots() {
    run_saved_snapshots_suite(SNAPSHOT_SUITE_ANTHROPIC, assert_anthropic_roundtrip);
}

#[test]
fn responses_roundtrip_saved_snapshots() {
    run_saved_snapshots_suite(SNAPSHOT_SUITE_RESPONSES, assert_responses_roundtrip);
}

#[test]
fn chat_anthropic_two_arm_saved_snapshots() {
    run_saved_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        assert_chat_anthropic_two_arm,
    );
}

#[test]
fn chat_responses_anthropic_three_arm_saved_snapshots() {
    run_saved_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_RESPONSES_ANTHROPIC_THREE_ARM,
        assert_chat_responses_anthropic_three_arm,
    );
}

/// Prune fuzz snapshots in a loop until stable:
/// - remove malformed request/meta pairs
/// - remove orphan meta files
/// - dedupe snapshots that fail for the same reason
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
fn responses_roundtrip_prune_snapshots() {
    run_prune_snapshots_suite(SNAPSHOT_SUITE_RESPONSES, assert_responses_roundtrip);
}

#[test]
#[ignore]
fn chat_anthropic_two_arm_prune_snapshots() {
    run_prune_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        assert_chat_anthropic_two_arm,
    );
}

#[test]
#[ignore]
fn chat_responses_anthropic_three_arm_prune_snapshots() {
    run_prune_snapshots_suite(
        SNAPSHOT_SUITE_CHAT_RESPONSES_ANTHROPIC_THREE_ARM,
        assert_chat_responses_anthropic_three_arm,
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
        strategies::arb_anthropic_payload(),
        assert_anthropic_roundtrip,
        assert_anthropic_roundtrip_verbose,
    );
}

#[test]
#[ignore]
fn responses_roundtrip() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_RESPONSES,
        "responses",
        "request-roundtrip",
        strategies::arb_responses_payload(),
        assert_responses_roundtrip,
        assert_responses_roundtrip_verbose,
    );
}

#[test]
#[ignore]
fn chat_anthropic_two_arm() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_CHAT_ANTHROPIC_TWO_ARM,
        "chat-completions",
        "chat-anthropic-two-arm",
        strategies::arb_openai_payload(),
        assert_chat_anthropic_two_arm,
        assert_chat_anthropic_two_arm_verbose,
    );
}

#[test]
#[ignore]
fn chat_responses_anthropic_three_arm() {
    run_fail_fast_suite(
        SNAPSHOT_SUITE_CHAT_RESPONSES_ANTHROPIC_THREE_ARM,
        "chat-completions",
        "chat-responses-anthropic-three-arm",
        strategies::arb_openai_payload(),
        assert_chat_responses_anthropic_three_arm,
        assert_chat_responses_anthropic_three_arm_verbose,
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
        strategies::arb_anthropic_payload(),
        assert_anthropic_roundtrip,
    );
}

#[test]
#[ignore]
fn responses_roundtrip_stats() {
    run_stats_suite(
        SNAPSHOT_SUITE_RESPONSES,
        "responses",
        "request-roundtrip",
        "Responses roundtrip fuzz",
        strategies::arb_responses_payload(),
        assert_responses_roundtrip,
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
        strategies::arb_openai_payload(),
        assert_chat_anthropic_two_arm,
    );
}

#[test]
#[ignore]
fn chat_responses_anthropic_three_arm_stats() {
    run_stats_suite(
        SNAPSHOT_SUITE_CHAT_RESPONSES_ANTHROPIC_THREE_ARM,
        "chat-completions",
        "chat-responses-anthropic-three-arm",
        "Chat->Responses->Anthropic three-arm fuzz",
        strategies::arb_openai_payload(),
        assert_chat_responses_anthropic_three_arm,
    );
}
