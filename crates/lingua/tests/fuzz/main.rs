//! Property-based fuzz tests: provider JSON -> Universal -> provider JSON (exact match).
//!
//! Generates random provider payloads from OpenAPI specs, converts to Universal
//! and back, and asserts the output JSON exactly matches the input.
//!
//! Two test modes per provider:
//!   `make run-openai`       / `make run-anthropic`       - fail on first error (debugging)
//!   `make stats-openai`     / `make stats-anthropic`     - aggregated summary (triage)

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

    collect_diff_issues(payload, &output)
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

    format_diff_error(&format!("{:?}", format), payload, &output)
}

/// Source -> Universal -> Target -> Universal -> Source.
/// Returns a list of diff descriptions, or empty if exact match.
/// Returns None if the payload was rejected at any conversion step (skip).
fn assert_cross_provider_roundtrip(
    source: ProviderFormat,
    target: ProviderFormat,
    payload: &Value,
) -> Option<Vec<String>> {
    let src_adapter = adapter_for_format(source)?;
    let tgt_adapter = adapter_for_format(target)?;

    // Source -> Universal
    let universal = src_adapter.request_to_universal(payload.clone()).ok()?;
    // Universal -> Target
    let target_payload = tgt_adapter.request_from_universal(&universal).ok()?;
    // Target -> Universal
    let universal2 = tgt_adapter.request_to_universal(target_payload).ok()?;
    // Universal -> Source
    let output = match src_adapter.request_from_universal(&universal2) {
        Ok(o) => o,
        Err(e) => return Some(vec![format!("final request_from_universal error: {}", e)]),
    };

    if *payload == output {
        return Some(vec![]);
    }

    collect_diff_issues(payload, &output)
}

/// Verbose cross-provider roundtrip check for debugging.
fn assert_cross_provider_roundtrip_verbose(
    source: ProviderFormat,
    target: ProviderFormat,
    payload: &Value,
) -> Result<bool, String> {
    let src_adapter = adapter_for_format(source)
        .ok_or_else(|| format!("No adapter for {:?}", source))?;
    let tgt_adapter = adapter_for_format(target)
        .ok_or_else(|| format!("No adapter for {:?}", target))?;

    let universal = match src_adapter.request_to_universal(payload.clone()) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };
    let target_payload = match tgt_adapter.request_from_universal(&universal) {
        Ok(o) => o,
        Err(_) => return Ok(false),
    };
    let universal2 = match tgt_adapter.request_to_universal(target_payload) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };
    let output = src_adapter
        .request_from_universal(&universal2)
        .map_err(|e| format!("final request_from_universal({:?}): {}", source, e))?;

    if *payload == output {
        return Ok(true);
    }

    format_diff_error(
        &format!("{:?} -> {:?} -> {:?}", source, target, source),
        payload,
        &output,
    )
}

// ============================================================================
// Diff helpers
// ============================================================================

fn collect_diff_issues(input: &Value, output: &Value) -> Option<Vec<String>> {
    let diff = diff_json(input, output);
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

fn format_diff_error(label: &str, input: &Value, output: &Value) -> Result<bool, String> {
    let diff = diff_json(input, output);
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
        "{} roundtrip mismatch:\n{}\n\nInput:  {}\nOutput: {}",
        label,
        issues.join("\n"),
        serde_json::to_string_pretty(input).unwrap(),
        serde_json::to_string_pretty(output).unwrap(),
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
    run_roundtrip_stats("OpenAI", ProviderFormat::ChatCompletions, strategies::arb_openai_payload());
}

/// Anthropic: fail on the first error with verbose output.
#[test]
#[ignore]
fn anthropic_roundtrip() {
    let config = ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    proptest!(config, |(payload in strategies::arb_anthropic_payload())| {
        if let Err(e) = assert_provider_roundtrip_verbose(ProviderFormat::Anthropic, &payload) {
            prop_assert!(false, "{}", e);
        }
    });
}

/// Anthropic: run all cases and report aggregated summary.
#[test]
#[ignore]
fn anthropic_roundtrip_stats() {
    run_roundtrip_stats("Anthropic", ProviderFormat::Anthropic, strategies::arb_anthropic_payload());
}

/// Anthropic -> OpenAI -> Anthropic: fail on first error with verbose output.
#[test]
#[ignore]
fn anthropic_openai_anthropic() {
    let config = ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    proptest!(config, |(payload in strategies::arb_anthropic_payload())| {
        if let Err(e) = assert_cross_provider_roundtrip_verbose(
            ProviderFormat::Anthropic,
            ProviderFormat::ChatCompletions,
            &payload,
        ) {
            prop_assert!(false, "{}", e);
        }
    });
}

/// Anthropic -> OpenAI -> Anthropic: aggregated summary.
#[test]
#[ignore]
fn anthropic_openai_anthropic_stats() {
    run_cross_provider_stats(
        "Anthropic -> OpenAI -> Anthropic",
        ProviderFormat::Anthropic,
        ProviderFormat::ChatCompletions,
        strategies::arb_anthropic_payload(),
    );
}

// ============================================================================
// Shared stats runners
// ============================================================================

fn run_roundtrip_stats(
    label: &str,
    format: ProviderFormat,
    strategy: BoxedStrategy<Value>,
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

        match assert_provider_roundtrip(format, &payload) {
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
        "\n--- {} roundtrip fuzz: {} passed, {} failed, {} skipped (of {}) ---",
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

fn run_cross_provider_stats(
    label: &str,
    source: ProviderFormat,
    target: ProviderFormat,
    strategy: BoxedStrategy<Value>,
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

        match assert_cross_provider_roundtrip(source, target, &payload) {
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
        "\n--- {} fuzz: {} passed, {} failed, {} skipped (of {}) ---",
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
