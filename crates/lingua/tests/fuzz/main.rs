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

mod schema_strategy;

// ============================================================================
// Roundtrip assertion
// ============================================================================

/// Provider JSON -> Universal -> Provider JSON.
/// Returns a list of diff descriptions, or empty if exact match.
/// Returns None if the payload was rejected by request_to_universal (skip).
fn assert_provider_roundtrip(format: ProviderFormat, payload: &Value) -> Option<Vec<String>> {
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
        if let Err(e) = assert_provider_roundtrip_verbose(ProviderFormat::ChatCompletions, &payload) {
            prop_assert!(false, "{}", e);
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
