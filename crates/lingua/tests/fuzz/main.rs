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
use std::collections::BTreeMap;
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
    let dir = fuzz_snapshot_dir();
    if !dir.exists() {
        return;
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

    for path in entries {
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
