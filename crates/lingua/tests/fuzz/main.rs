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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory should exist")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn fuzz_snapshot_dir() -> PathBuf {
    workspace_root().join("payloads/fuzz-snapshots/openai-roundtrip")
}

fn fnv1a_64(input: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn save_failing_snapshot(payload: &Value, issues: &[String]) -> Result<PathBuf, String> {
    let compact =
        serde_json::to_vec(payload).map_err(|e| format!("failed to serialize payload: {}", e))?;
    let hash = fnv1a_64(&compact);
    let case_id = format!("case-{hash:016x}");

    let dir = fuzz_snapshot_dir();
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
        "provider": "chat-completions",
        "kind": "request-roundtrip",
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

fn snapshot_request_paths() -> Vec<PathBuf> {
    let dir = fuzz_snapshot_dir();
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

fn prune_orphan_meta_files() -> usize {
    let dir = fuzz_snapshot_dir();
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
}

// ============================================================================
// Fuzz tests
// ============================================================================

const CASES: u32 = 256;

#[test]
fn openai_roundtrip_saved_snapshots() {
    for path in snapshot_request_paths() {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read snapshot {}: {}", path.display(), e));
        let payload: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("invalid json in {}: {}", path.display(), e));

        if !is_openai_exact_roundtrip_scope(&payload) {
            continue;
        }

        match assert_provider_roundtrip_verbose(ProviderFormat::ChatCompletions, &payload) {
            Ok(true) => {}
            Ok(false) => panic!(
                "snapshot {} is no longer valid chat-completions input",
                path.display()
            ),
            Err(e) => panic!("snapshot {} failed roundtrip:\n{}", path.display(), e),
        }
    }
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
    let mut iterations = 0usize;
    let mut removed_malformed = 0usize;
    let mut removed_orphan_meta = 0usize;
    let mut removed_duplicate_reason = 0usize;

    loop {
        iterations += 1;
        let mut changed = false;

        let orphan_removed = prune_orphan_meta_files();
        if orphan_removed > 0 {
            removed_orphan_meta += orphan_removed;
            changed = true;
        }

        let mut kept_by_reason: BTreeMap<String, PathBuf> = BTreeMap::new();
        for path in snapshot_request_paths() {
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
            } else if is_openai_exact_roundtrip_scope(&payload) {
                match assert_provider_roundtrip(ProviderFormat::ChatCompletions, &payload) {
                    Some(issues) if !issues.is_empty() => issues,
                    _ => continue,
                }
            } else {
                continue;
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
        "prune complete after {} iterations: removed_malformed={} removed_orphan_meta={} removed_duplicate_reason={}",
        iterations, removed_malformed, removed_orphan_meta, removed_duplicate_reason
    );
}

/// Fail on the first error with verbose output (input, output, diff).
/// Use for debugging a specific issue.
#[test]
#[ignore]
fn openai_roundtrip() {
    let config = ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    proptest!(config, |(payload in strategies::arb_openai_payload())| {
        if let Some(issues) = assert_provider_roundtrip(ProviderFormat::ChatCompletions, &payload) {
            if !issues.is_empty() {
                let snapshot_msg = match save_failing_snapshot(&payload, &issues) {
                    Ok(path) => format!("\nSaved failing snapshot: {}", path.display()),
                    Err(err) => format!("\nFailed to save snapshot: {}", err),
                };
                let verbose = assert_provider_roundtrip_verbose(ProviderFormat::ChatCompletions, &payload)
                    .err()
                    .unwrap_or_else(|| "roundtrip mismatch (verbose details unavailable)".to_string());
                prop_assert!(false, "{}{}", verbose, snapshot_msg);
            }
        }
    });
}

/// Run all cases and report an aggregated summary of unique issues.
/// Use for triaging the full scope of failures.
#[test]
#[ignore]
fn openai_roundtrip_stats() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    });

    let mut failures: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut errored = 0usize;

    let strategy = strategies::arb_openai_payload();

    for _ in 0..CASES {
        let value_tree = match strategy.new_tree(&mut runner) {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let payload = value_tree.current();

        match assert_provider_roundtrip(ProviderFormat::ChatCompletions, &payload) {
            None => skipped += 1,
            Some(issues) if issues.is_empty() => passed += 1,
            Some(issues) => {
                errored += 1;
                if let Ok(path) = save_failing_snapshot(&payload, &issues) {
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
        "\n--- OpenAI roundtrip fuzz: {} passed, {} failed, {} skipped (of {}) ---",
        passed, errored, skipped, CASES,
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
