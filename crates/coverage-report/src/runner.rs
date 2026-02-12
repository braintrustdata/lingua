/*!
Test execution for coverage-report.
*/

use std::collections::HashMap;

use lingua::capabilities::ProviderFormat;
use lingua::processing::adapters::ProviderAdapter;
use lingua::serde_json::Value;
use lingua::universal::{UniversalRequest, UniversalResponse, UniversalStreamChunk};

use crate::discovery::{discover_test_cases_filtered, load_payload};
use crate::expected::TestCategory;
use crate::normalizers::{
    normalize_request_for_comparison, normalize_response_for_comparison,
    normalize_stream_chunk_for_comparison,
};
use crate::types::{PairResult, TestFilter, TransformResult, ValidationLevel};

type PairResults = HashMap<(usize, usize), PairResult>;
type AllResults = (PairResults, PairResults, PairResults);

fn universal_request_to_value(req: &UniversalRequest) -> Value {
    lingua::serde_json::to_value(normalize_request_for_comparison(req)).unwrap_or(Value::Null)
}

fn universal_response_to_value(resp: &UniversalResponse) -> Value {
    lingua::serde_json::to_value(normalize_response_for_comparison(resp)).unwrap_or(Value::Null)
}

fn universal_stream_to_value(chunk: &UniversalStreamChunk) -> Value {
    lingua::serde_json::to_value(normalize_stream_chunk_for_comparison(chunk))
        .unwrap_or(Value::Null)
}

fn diff_to_transform_result(result: RoundtripResult) -> TransformResult {
    // For limitations, extract reason from expected_diffs if available
    let limitation_reason = if result.level == ValidationLevel::Limitation {
        result
            .diff
            .as_ref()
            .and_then(|d| d.expected_diffs.first())
            .map(|(_, _, _, reason)| reason.clone())
    } else {
        None
    };

    TransformResult {
        level: result.level,
        error: result.error,
        diff: result.diff,
        limitation_reason,
    }
}

// Validation uses request_to_universal/response_to_universal from the adapter trait.
// These methods return Result with detailed error info when validation fails.

pub fn test_request_transformation(
    test_case: &str,
    source_adapter: &dyn ProviderAdapter,
    target_adapter: &dyn ProviderAdapter,
    filename: &str,
) -> TransformResult {
    let payload = match load_payload(test_case, source_adapter.directory_name(), filename) {
        Some(p) => p,
        None => {
            let error = format!("Source payload not found: {}", filename);
            return TransformResult {
                level: ValidationLevel::Skipped,
                error: Some(error),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    // Provide model for formats that have model in URL, not in payload body
    let model: Option<&str> = match source_adapter.format() {
        ProviderFormat::Google => Some("gemini-1.5-pro"),
        ProviderFormat::Converse => Some("anthropic.claude-3-sonnet"),
        ProviderFormat::BedrockAnthropic => Some("us.anthropic.claude-haiku-4-5-20251001-v1:0"),
        _ => None,
    };

    let payload_value: Value = match lingua::serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to parse source payload: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    let mut expected_universal = match source_adapter.request_to_universal(payload_value) {
        Ok(u) => u,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Conversion to universal format failed: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    if model.is_some() && expected_universal.model.is_none() {
        expected_universal.model = model.map(String::from);
    }

    target_adapter.apply_defaults(&mut expected_universal);
    let expected_universal_value = universal_request_to_value(&expected_universal);

    let provider_value = match target_adapter.request_from_universal(&expected_universal) {
        Ok(v) => v,
        Err(e) => {
            let error_msg = format!("Conversion from universal failed: {}", e);
            let context = CompareContext::for_cross_provider(
                TestCategory::Requests,
                source_adapter,
                target_adapter,
                test_case,
            );
            let reason = context.as_ref().and_then(|ctx| {
                ctx.is_test_case_limitation().or_else(|| {
                    is_expected_error(
                        ctx.category,
                        ctx.source,
                        ctx.target,
                        Some(ctx.test_case),
                        &error_msg,
                    )
                })
            });

            let level = if reason.is_some() {
                ValidationLevel::Limitation
            } else {
                ValidationLevel::Fail
            };

            return TransformResult {
                level,
                error: Some(error_msg),
                diff: None,
                limitation_reason: reason.map(|r| r.to_string()),
            };
        }
    };

    let transformed: Value = match lingua::serde_json::to_value(&provider_value) {
        Ok(v) => v,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to serialize provider value: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    // Use request_to_universal to validate - gives detailed error info
    match target_adapter.request_to_universal(transformed) {
        Ok(mut target_universal) => {
            // For self-roundtrip tests, re-inject the synthetic model so both sides
            // of the comparison match. Formats like BedrockAnthropic, Google, and
            // Converse carry model in the URL path, not the payload body, so the
            // roundtripped universal would otherwise lose the model we injected above.
            if source_adapter.format() == target_adapter.format()
                && model.is_some()
                && target_universal.model.is_none()
            {
                target_universal.model = model.map(String::from);
            }
            let target_universal_value = universal_request_to_value(&target_universal);
            let context = CompareContext::for_cross_provider(
                TestCategory::Requests,
                source_adapter,
                target_adapter,
                test_case,
            );
            let roundtrip_result = compare_values(
                &expected_universal_value,
                &target_universal_value,
                context.as_ref(),
            );
            diff_to_transform_result(roundtrip_result)
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("Conversion from universal format failed: {}", e)),
            diff: None,
            limitation_reason: None,
        },
    }
}

pub fn test_response_transformation(
    test_case: &str,
    source_adapter: &dyn ProviderAdapter,
    target_adapter: &dyn ProviderAdapter,
    filename: &str,
) -> TransformResult {
    let payload = match load_payload(test_case, source_adapter.directory_name(), filename) {
        Some(p) => p,
        None => {
            return TransformResult {
                level: ValidationLevel::Skipped,
                error: Some(format!("Response payload not found: {}", filename)),
                diff: None,
                limitation_reason: None,
            }
        }
    };

    let payload_value: Value = match lingua::serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to parse source payload: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    let expected_universal = match source_adapter.response_to_universal(payload_value) {
        Ok(u) => u,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Conversion to universal format failed: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    let expected_universal_value = universal_response_to_value(&expected_universal);

    let provider_value = match target_adapter.response_from_universal(&expected_universal) {
        Ok(v) => v,
        Err(e) => {
            let error_msg = format!("Conversion from universal failed: {}", e);
            let context = CompareContext::for_cross_provider(
                TestCategory::Responses,
                source_adapter,
                target_adapter,
                test_case,
            );
            let reason = context.as_ref().and_then(|ctx| {
                ctx.is_test_case_limitation().or_else(|| {
                    is_expected_error(
                        ctx.category,
                        ctx.source,
                        ctx.target,
                        Some(ctx.test_case),
                        &error_msg,
                    )
                })
            });

            let level = if reason.is_some() {
                ValidationLevel::Limitation
            } else {
                ValidationLevel::Fail
            };

            return TransformResult {
                level,
                error: Some(error_msg),
                diff: None,
                limitation_reason: reason.map(|r| r.to_string()),
            };
        }
    };

    let transformed: Value = match lingua::serde_json::to_value(&provider_value) {
        Ok(v) => v,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to serialize provider value: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    match target_adapter.response_to_universal(transformed) {
        Ok(target_universal) => {
            let target_universal_value = universal_response_to_value(&target_universal);
            let context = CompareContext::for_cross_provider(
                TestCategory::Responses,
                source_adapter,
                target_adapter,
                test_case,
            );
            let roundtrip_result = compare_values(
                &expected_universal_value,
                &target_universal_value,
                context.as_ref(),
            );
            diff_to_transform_result(roundtrip_result)
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("Conversion from universal format failed: {}", e)),
            diff: None,
            limitation_reason: None,
        },
    }
}

/// Test streaming response transformation for a single test case.
/// Returns a TransformResult indicating pass/fail for the entire streaming file.
pub fn test_streaming_transformation(
    test_case: &str,
    source_adapter: &dyn ProviderAdapter,
    target_adapter: &dyn ProviderAdapter,
    filename: &str,
) -> TransformResult {
    let payload_bytes = match load_payload(test_case, source_adapter.directory_name(), filename) {
        Some(p) => p,
        None => {
            // No streaming file - skip this test
            return TransformResult {
                level: ValidationLevel::Skipped,
                error: Some(format!("Streaming payload not found: {}", filename)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    // Parse the bytes to get the array of events
    let payload: Value = match lingua::serde_json::from_slice(&payload_bytes) {
        Ok(v) => v,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to parse streaming payload: {}", e)),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    let events = match payload.as_array() {
        Some(arr) => arr,
        None => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some("Streaming payload is not an array".to_string()),
                diff: None,
                limitation_reason: None,
            };
        }
    };

    // Test all events - fail if any event fails
    for (idx, event) in events.iter().enumerate() {
        let result = test_single_stream_event(event, source_adapter, target_adapter, test_case);
        if result.level != ValidationLevel::Pass {
            return TransformResult {
                level: result.level,
                error: result
                    .error
                    .map(|e| format!("Event {}: {}", idx, e))
                    .or(Some(format!("Event {} failed", idx))),
                diff: result.diff,
                limitation_reason: result.limitation_reason,
            };
        }
    }

    TransformResult {
        level: ValidationLevel::Pass,
        error: None,
        diff: None,
        limitation_reason: None,
    }
}

/// Test a single streaming event transformation
fn test_single_stream_event(
    event: &Value,
    source_adapter: &dyn ProviderAdapter,
    target_adapter: &dyn ProviderAdapter,
    test_case: &str,
) -> TransformResult {
    let source_universal = match source_adapter.stream_to_universal(event.clone()) {
        Ok(u) => u,
        Err(e) => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Conversion to universal format failed: {}", e)),
                diff: None,
                limitation_reason: None,
            }
        }
    };

    let target_universal = match &source_universal {
        Some(chunk) => {
            let provider_value = match target_adapter.stream_from_universal(chunk) {
                Ok(v) => v,
                Err(e) => {
                    return TransformResult {
                        level: ValidationLevel::Fail,
                        error: Some(format!("Conversion from universal failed: {}", e)),
                        diff: None,
                        limitation_reason: None,
                    };
                }
            };

            let transformed: Value = match lingua::serde_json::to_value(&provider_value) {
                Ok(v) => v,
                Err(e) => {
                    return TransformResult {
                        level: ValidationLevel::Fail,
                        error: Some(format!("Failed to serialize provider value: {}", e)),
                        diff: None,
                        limitation_reason: None,
                    };
                }
            };

            // Convert back to universal for comparison
            match target_adapter.stream_to_universal(transformed) {
                Ok(u) => u,
                Err(e) => {
                    return TransformResult {
                        level: ValidationLevel::Fail,
                        error: Some(format!("Conversion from universal format failed: {}", e)),
                        diff: None,
                        limitation_reason: None,
                    };
                }
            }
        }
        None => {
            // Keep-alive event with no universal representation - pass through
            None
        }
    };

    let context = CompareContext::for_cross_provider(
        TestCategory::Streaming,
        source_adapter,
        target_adapter,
        test_case,
    );

    match (source_universal, target_universal) {
        (None, None) => TransformResult {
            level: ValidationLevel::Pass,
            error: None,
            diff: None,
            limitation_reason: None,
        },
        (Some(source_chunk), Some(target_chunk)) => {
            let source_value = universal_stream_to_value(&source_chunk);
            let target_value = universal_stream_to_value(&target_chunk);
            let roundtrip_result = compare_values(&source_value, &target_value, context.as_ref());
            diff_to_transform_result(roundtrip_result)
        }
        (source, target) => {
            let source_value = source
                .as_ref()
                .map(universal_stream_to_value)
                .unwrap_or(Value::Null);
            let target_value = target
                .as_ref()
                .map(universal_stream_to_value)
                .unwrap_or(Value::Null);
            let roundtrip_result = compare_values(&source_value, &target_value, context.as_ref());
            diff_to_transform_result(roundtrip_result)
        }
    }
}

/// Run all cross-transformation tests and collect results
pub fn run_all_tests(adapters: &[Box<dyn ProviderAdapter>], filter: &TestFilter) -> AllResults {
    let test_cases = discover_test_cases_filtered(filter);
    let mut request_results: PairResults = HashMap::new();
    let mut response_results: PairResults = HashMap::new();
    let mut streaming_results: PairResults = HashMap::new();

    // Initialize results for all pairs that match the filter (including self-pairs for roundtrip)
    for (source_idx, source_adapter) in adapters.iter().enumerate() {
        for (target_idx, target_adapter) in adapters.iter().enumerate() {
            if filter.matches_provider_pair(source_adapter.format(), target_adapter.format()) {
                request_results.insert((source_idx, target_idx), PairResult::default());
                response_results.insert((source_idx, target_idx), PairResult::default());
                streaming_results.insert((source_idx, target_idx), PairResult::default());
            }
        }
    }

    // Test each source→target pair for each test case (including self-pairs for roundtrip)
    for test_case in &test_cases {
        for (source_idx, source) in adapters.iter().enumerate() {
            for (target_idx, target) in adapters.iter().enumerate() {
                // Skip pairs that don't match the filter
                if !filter.matches_provider_pair(source.format(), target.format()) {
                    continue;
                }

                let source = source.as_ref();
                let target = target.as_ref();

                // Test first turn request
                let result = test_request_transformation(test_case, source, target, "request.json");
                let pair_result = request_results.get_mut(&(source_idx, target_idx)).unwrap();

                match result.level {
                    ValidationLevel::Skipped => { /* do nothing */ }
                    ValidationLevel::Pass => pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        pair_result.failed += 1;
                        let error = result.error.unwrap_or_else(|| "Unknown error".to_string());
                        pair_result.failures.push((
                            format!("{} (request)", test_case),
                            error,
                            result.diff,
                        ));
                    }
                    ValidationLevel::Limitation => {
                        pair_result.limitations += 1;
                        let detail = result
                            .limitation_reason
                            .or(result.error)
                            .unwrap_or_else(|| "Unknown limitation".to_string());
                        pair_result.limitation_details.push((
                            format!("{} (request)", test_case),
                            detail,
                            result.diff,
                        ));
                    }
                }

                // Test followup request if exists
                let followup_result =
                    test_request_transformation(test_case, source, target, "followup-request.json");
                match followup_result.level {
                    ValidationLevel::Skipped => { /* do nothing */ }
                    ValidationLevel::Pass => pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        pair_result.failed += 1;
                        let error = followup_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string());
                        pair_result.failures.push((
                            format!("{} (followup)", test_case),
                            error,
                            followup_result.diff,
                        ));
                    }
                    ValidationLevel::Limitation => {
                        pair_result.limitations += 1;
                        let detail = followup_result
                            .limitation_reason
                            .or(followup_result.error)
                            .unwrap_or_else(|| "Unknown limitation".to_string());
                        pair_result.limitation_details.push((
                            format!("{} (followup)", test_case),
                            detail,
                            followup_result.diff,
                        ));
                    }
                }

                // Test response transformation (source response transforms to target format)
                let response_result =
                    test_response_transformation(test_case, source, target, "response.json");
                let resp_pair_result = response_results.get_mut(&(source_idx, target_idx)).unwrap();

                match response_result.level {
                    ValidationLevel::Skipped => { /* do nothing */ }
                    ValidationLevel::Pass => resp_pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        resp_pair_result.failed += 1;
                        let error = response_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string());
                        resp_pair_result.failures.push((
                            format!("{} (response)", test_case),
                            error,
                            response_result.diff,
                        ));
                    }
                    ValidationLevel::Limitation => {
                        resp_pair_result.limitations += 1;
                        let detail = response_result
                            .limitation_reason
                            .or(response_result.error)
                            .unwrap_or_else(|| "Unknown limitation".to_string());
                        resp_pair_result.limitation_details.push((
                            format!("{} (response)", test_case),
                            detail,
                            response_result.diff,
                        ));
                    }
                }

                // Test streaming response transformation
                let stream_pair_result = streaming_results
                    .get_mut(&(source_idx, target_idx))
                    .unwrap();

                let streaming_result = test_streaming_transformation(
                    test_case,
                    source,
                    target,
                    "response-streaming.json",
                );
                match streaming_result.level {
                    ValidationLevel::Skipped => { /* do nothing */ }
                    ValidationLevel::Pass => stream_pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        stream_pair_result.failed += 1;
                        let error = streaming_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string());
                        stream_pair_result.failures.push((
                            format!("{} (streaming)", test_case),
                            error,
                            streaming_result.diff,
                        ));
                    }
                    ValidationLevel::Limitation => {
                        stream_pair_result.limitations += 1;
                        let detail = streaming_result
                            .limitation_reason
                            .or(streaming_result.error)
                            .unwrap_or_else(|| "Unknown limitation".to_string());
                        stream_pair_result.limitation_details.push((
                            format!("{} (streaming)", test_case),
                            detail,
                            streaming_result.diff,
                        ));
                    }
                }

                // Test followup streaming if exists
                let followup_streaming_result = test_streaming_transformation(
                    test_case,
                    source,
                    target,
                    "followup-response-streaming.json",
                );
                match followup_streaming_result.level {
                    ValidationLevel::Skipped => { /* do nothing */ }
                    ValidationLevel::Pass => stream_pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        stream_pair_result.failed += 1;
                        let error = followup_streaming_result
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string());
                        stream_pair_result.failures.push((
                            format!("{} (followup-streaming)", test_case),
                            error,
                            followup_streaming_result.diff,
                        ));
                    }
                    ValidationLevel::Limitation => {
                        stream_pair_result.limitations += 1;
                        let detail = followup_streaming_result
                            .limitation_reason
                            .or(followup_streaming_result.error)
                            .unwrap_or_else(|| "Unknown limitation".to_string());
                        stream_pair_result.limitation_details.push((
                            format!("{} (followup-streaming)", test_case),
                            detail,
                            followup_streaming_result.diff,
                        ));
                    }
                }
            }
        }
    }

    (request_results, response_results, streaming_results)
}

// ============================================================================
// Roundtrip testing (Provider → Universal → Provider)
// ============================================================================

use crate::expected::{is_expected_error, is_expected_field, is_expected_test_case};
use crate::types::{RoundtripDiff, RoundtripResult};
use std::collections::HashSet;

/// Context for value comparison, carrying provider names for expected-difference filtering.
struct CompareContext<'a> {
    category: TestCategory,
    source: &'a str,
    target: &'a str,
    test_case: &'a str,
}

impl<'a> CompareContext<'a> {
    fn new(category: TestCategory, source: &'a str, target: &'a str, test_case: &'a str) -> Self {
        Self {
            category,
            source,
            target,
            test_case,
        }
    }

    /// Create context for cross-provider comparison, or None for roundtrip tests.
    /// Roundtrip tests (source == target) don't use expected differences because
    /// any data loss in Format→Universal→Format is a real bug, not a "limitation".
    fn for_cross_provider(
        category: TestCategory,
        source_adapter: &'a dyn ProviderAdapter,
        target_adapter: &'a dyn ProviderAdapter,
        test_case: &'a str,
    ) -> Option<Self> {
        if source_adapter.format() == target_adapter.format() {
            None
        } else {
            Some(Self::new(
                category,
                source_adapter.display_name(),
                target_adapter.display_name(),
                test_case,
            ))
        }
    }

    /// Check if this entire test case is an expected limitation.
    fn is_test_case_limitation(&self) -> Option<String> {
        is_expected_test_case(self.category, self.source, self.target, self.test_case)
    }

    /// Check if a field difference is expected for this source→target translation.
    /// Returns the reason if expected, None otherwise.
    fn is_expected(&self, field: &str) -> Option<String> {
        is_expected_field(
            self.category,
            self.source,
            self.target,
            Some(self.test_case),
            field,
        )
    }
}

/// Compare two JSON values and produce a RoundtripDiff.
///
/// When `context` is provided, expected differences (based on source/target provider)
/// are filtered out and tracked as limitations. When `context` is None, all differences are reported.
fn compare_values(
    original: &Value,
    roundtripped: &Value,
    context: Option<&CompareContext>,
) -> RoundtripResult {
    // Check if entire test case is a known limitation (coarsest check)
    let test_case_limitation = context.and_then(|ctx| ctx.is_test_case_limitation());

    // Always run comparison to capture the actual diffs
    let mut diff = RoundtripDiff::default();
    compare_recursive(original, roundtripped, "", &mut diff, context);

    // If this is a test-case-level limitation, move all diffs to expected_diffs
    if let Some(reason) = &test_case_limitation {
        // Move lost fields to expected_diffs
        for field in diff.lost_fields.drain(..) {
            diff.expected_diffs.push((
                field,
                "(had value)".to_string(),
                "(missing)".to_string(),
                reason.clone(),
            ));
        }
        // Move added fields to expected_diffs
        for field in diff.added_fields.drain(..) {
            diff.expected_diffs.push((
                field,
                "(missing)".to_string(),
                "(has value)".to_string(),
                reason.clone(),
            ));
        }
        // Move changed fields to expected_diffs
        for (field, before, after) in diff.changed_fields.drain(..) {
            diff.expected_diffs
                .push((field, before, after, reason.clone()));
        }
    }

    let has_real_diffs = !diff.lost_fields.is_empty()
        || !diff.added_fields.is_empty()
        || !diff.changed_fields.is_empty();
    let has_expected_diffs = !diff.expected_diffs.is_empty();

    if has_real_diffs {
        // Real failures - report as Fail
        RoundtripResult {
            level: ValidationLevel::Fail,
            error: Some(format!(
                "{} lost, {} added, {} changed",
                diff.lost_fields.len(),
                diff.added_fields.len(),
                diff.changed_fields.len()
            )),
            diff: Some(diff),
        }
    } else if has_expected_diffs {
        // Only expected differences - report as Limitation
        let error_msg = if let Some(reason) = test_case_limitation {
            format!("Expected limitation: {}", reason)
        } else {
            format!("{} expected limitation(s)", diff.expected_diffs.len())
        };
        RoundtripResult {
            level: ValidationLevel::Limitation,
            error: Some(error_msg),
            diff: Some(diff),
        }
    } else {
        // No differences at all - Pass
        RoundtripResult {
            level: ValidationLevel::Pass,
            error: None,
            diff: None,
        }
    }
}

/// Recursively compare two JSON values and accumulate differences.
///
/// When `context` is provided, expected differences are filtered out.
fn compare_recursive(
    original: &Value,
    roundtripped: &Value,
    path: &str,
    diff: &mut RoundtripDiff,
    context: Option<&CompareContext>,
) {
    match (original, roundtripped) {
        (Value::Object(orig), Value::Object(round)) => {
            let orig_keys: HashSet<_> = orig.keys().collect();
            let round_keys: HashSet<_> = round.keys().collect();

            // Check for lost fields (in original but not in roundtripped)
            for key in orig_keys.difference(&round_keys) {
                let field_path = if path.is_empty() {
                    (*key).clone()
                } else {
                    format!("{}.{}", path, key)
                };
                // Track expected differences as limitations
                if let Some(reason) = context.and_then(|ctx| ctx.is_expected(&field_path)) {
                    let before = lingua::serde_json::to_string(&orig[*key])
                        .unwrap_or_else(|_| "?".to_string());
                    diff.expected_diffs.push((
                        field_path,
                        before,
                        "(missing)".to_string(),
                        reason.to_string(),
                    ));
                } else {
                    diff.lost_fields.push(field_path);
                }
            }

            // Check for added fields (in roundtripped but not in original)
            for key in round_keys.difference(&orig_keys) {
                let field_path = if path.is_empty() {
                    (*key).clone()
                } else {
                    format!("{}.{}", path, key)
                };
                // Track expected differences as limitations
                if let Some(reason) = context.and_then(|ctx| ctx.is_expected(&field_path)) {
                    let after = lingua::serde_json::to_string(&round[*key])
                        .unwrap_or_else(|_| "?".to_string());
                    diff.expected_diffs.push((
                        field_path,
                        "(missing)".to_string(),
                        after,
                        reason.to_string(),
                    ));
                } else {
                    diff.added_fields.push(field_path);
                }
            }

            // Recursively compare common keys
            for key in orig_keys.intersection(&round_keys) {
                let new_path = if path.is_empty() {
                    (*key).clone()
                } else {
                    format!("{}.{}", path, key)
                };
                compare_recursive(&orig[*key], &round[*key], &new_path, diff, context);
            }
        }
        (Value::Array(orig), Value::Array(round)) => {
            // Compare array lengths
            if orig.len() != round.len() {
                let len_path = format!("{}.length", path);
                // Track expected differences as limitations
                if let Some(reason) = context.and_then(|ctx| ctx.is_expected(&len_path)) {
                    diff.expected_diffs.push((
                        len_path,
                        orig.len().to_string(),
                        round.len().to_string(),
                        reason.to_string(),
                    ));
                } else {
                    diff.changed_fields.push((
                        len_path,
                        orig.len().to_string(),
                        round.len().to_string(),
                    ));
                }
                return;
            }

            // Compare element by element
            for (idx, (o, r)) in orig.iter().zip(round.iter()).enumerate() {
                let new_path = format!("{}[{}]", path, idx);
                compare_recursive(o, r, &new_path, diff, context);
            }
        }
        (Value::Null, Value::Null) => {}
        (Value::Bool(o), Value::Bool(r)) if o == r => {}
        (Value::Number(o), Value::Number(r)) if o == r => {}
        (Value::String(o), Value::String(r)) if o == r => {}
        _ => {
            // Values differ - track expected differences as limitations
            let before =
                lingua::serde_json::to_string(original).unwrap_or_else(|_| "?".to_string());
            let after =
                lingua::serde_json::to_string(roundtripped).unwrap_or_else(|_| "?".to_string());
            if let Some(reason) = context.and_then(|ctx| ctx.is_expected(path)) {
                diff.expected_diffs
                    .push((path.to_string(), before, after, reason.to_string()));
            } else {
                diff.changed_fields.push((path.to_string(), before, after));
            }
        }
    }
}
