/*!
Test execution for coverage-report.
*/

use std::collections::HashMap;

use lingua::capabilities::ProviderFormat;
use lingua::processing::adapters::ProviderAdapter;
use lingua::processing::transform::{transform_request, transform_response, transform_stream_chunk};
use lingua::serde_json::Value;

use crate::discovery::{discover_test_cases, load_payload};
use crate::types::{PairResult, TransformResult, ValidationLevel};

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
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Source payload not found: {}", filename)),
            }
        }
    };

    // Provide model for formats that have model in URL (Google, Bedrock)
    let model: Option<&str> = match source_adapter.format() {
        ProviderFormat::Google => Some("gemini-1.5-pro"),
        ProviderFormat::Converse => Some("anthropic.claude-3-sonnet"),
        _ => None,
    };

    match transform_request(&payload, target_adapter.format(), model) {
        Ok(result) => {
            if result.is_pass_through() && source_adapter.format() == target_adapter.format() {
                return TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                };
            }

            let transformed = result.payload_or_original(payload);

            // Use request_to_universal to validate - gives detailed error info
            match target_adapter.request_to_universal(&transformed) {
                Ok(_) => TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                },
                Err(e) => TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("{}", e)),
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
                level: ValidationLevel::Fail,
                error: Some(format!("Response payload not found: {}", filename)),
            }
        }
    };

    match transform_response(&payload, target_adapter.format()) {
        Ok(result) => {
            let transformed = result.payload_or_original(payload);

            // Use response_to_universal to validate - gives detailed error info
            match target_adapter.response_to_universal(&transformed) {
                Ok(_) => TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                },
                Err(e) => TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => TransformResult {
            level: ValidationLevel::Fail,
            error: Some(format!("{}", e)),
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
    let payload = match load_payload(test_case, source_adapter.directory_name(), filename) {
        Some(p) => p,
        None => {
            // No streaming file - report as not found
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Streaming payload not found: {}", filename)),
            };
        }
    };

    let events = match payload.as_array() {
        Some(arr) => arr,
        None => {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some("Streaming payload is not an array".to_string()),
            };
        }
    };

    // Test all events - fail if any event fails
    for (idx, event) in events.iter().enumerate() {
        if let Err(e) = test_single_stream_event(event, target_adapter) {
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Event {}: {}", idx, e)),
            };
        }
    }

    TransformResult {
        level: ValidationLevel::Pass,
        error: None,
    }
}

/// Test a single streaming event transformation
fn test_single_stream_event(
    event: &Value,
    target_adapter: &dyn ProviderAdapter,
) -> Result<(), String> {
    // Transform the event to target format
    let result =
        transform_stream_chunk(event, target_adapter.format()).map_err(|e| e.to_string())?;

    let transformed = result.payload_or_original(event.clone());

    // Validate transformed output can be parsed by target adapter
    match target_adapter.stream_to_universal(&transformed) {
        Ok(Some(_chunk)) => Ok(()),
        Ok(None) => Ok(()), // Keep-alive events are valid
        Err(e) => Err(e.to_string()),
    }
}

/// Run all cross-transformation tests and collect results
pub fn run_all_tests(
    adapters: &[Box<dyn ProviderAdapter>],
) -> (
    HashMap<(usize, usize), PairResult>,
    HashMap<(usize, usize), PairResult>,
    HashMap<(usize, usize), PairResult>,
) {
    let test_cases = discover_test_cases();
    let mut request_results: HashMap<(usize, usize), PairResult> = HashMap::new();
    let mut response_results: HashMap<(usize, usize), PairResult> = HashMap::new();
    let mut streaming_results: HashMap<(usize, usize), PairResult> = HashMap::new();

    // Initialize results for all pairs
    for source_idx in 0..adapters.len() {
        for target_idx in 0..adapters.len() {
            if source_idx != target_idx {
                request_results.insert((source_idx, target_idx), PairResult::default());
                response_results.insert((source_idx, target_idx), PairResult::default());
                streaming_results.insert((source_idx, target_idx), PairResult::default());
            }
        }
    }

    // Test each source→target pair for each test case
    for test_case in &test_cases {
        for source_idx in 0..adapters.len() {
            for target_idx in 0..adapters.len() {
                if source_idx == target_idx {
                    continue;
                }

                let source = &adapters[source_idx];
                let target = &adapters[target_idx];

                // Test first turn request
                let result = test_request_transformation(
                    test_case,
                    source.as_ref(),
                    target.as_ref(),
                    "request.json",
                );
                let pair_result = request_results.get_mut(&(source_idx, target_idx)).unwrap();

                match result.level {
                    ValidationLevel::Pass => pair_result.passed += 1,
                    ValidationLevel::Fail => {
                        pair_result.failed += 1;
                        if let Some(error) = result.error {
                            pair_result
                                .failures
                                .push((format!("{} (request)", test_case), error));
                        }
                    }
                }

                // Test followup request if exists
                let followup_result = test_request_transformation(
                    test_case,
                    source.as_ref(),
                    target.as_ref(),
                    "followup-request.json",
                );
                if followup_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match followup_result.level {
                        ValidationLevel::Pass => pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            pair_result.failed += 1;
                            if let Some(error) = followup_result.error {
                                pair_result
                                    .failures
                                    .push((format!("{} (followup)", test_case), error));
                            }
                        }
                    }
                }

                // Test response transformation (source response transforms to target format)
                let response_result = test_response_transformation(
                    test_case,
                    source.as_ref(),
                    target.as_ref(),
                    "response.json",
                );
                let resp_pair_result = response_results.get_mut(&(source_idx, target_idx)).unwrap();

                if response_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match response_result.level {
                        ValidationLevel::Pass => resp_pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            resp_pair_result.failed += 1;
                            if let Some(error) = response_result.error {
                                resp_pair_result
                                    .failures
                                    .push((format!("{} (response)", test_case), error));
                            }
                        }
                    }
                }

                // Test streaming response transformation
                let stream_pair_result =
                    streaming_results.get_mut(&(source_idx, target_idx)).unwrap();

                let streaming_result = test_streaming_transformation(
                    test_case,
                    source.as_ref(),
                    target.as_ref(),
                    "response-streaming.json",
                );
                if streaming_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match streaming_result.level {
                        ValidationLevel::Pass => stream_pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            stream_pair_result.failed += 1;
                            if let Some(error) = streaming_result.error {
                                stream_pair_result
                                    .failures
                                    .push((format!("{} (streaming)", test_case), error));
                            }
                        }
                    }
                }

                // Test followup streaming if exists
                let followup_streaming_result = test_streaming_transformation(
                    test_case,
                    source.as_ref(),
                    target.as_ref(),
                    "followup-response-streaming.json",
                );
                if followup_streaming_result
                    .error
                    .as_ref()
                    .map_or(true, |e| !e.contains("not found"))
                {
                    match followup_streaming_result.level {
                        ValidationLevel::Pass => stream_pair_result.passed += 1,
                        ValidationLevel::Fail => {
                            stream_pair_result.failed += 1;
                            if let Some(error) = followup_streaming_result.error {
                                stream_pair_result
                                    .failures
                                    .push((format!("{} (followup-streaming)", test_case), error));
                            }
                        }
                    }
                }
            }
        }
    }

    (request_results, response_results, streaming_results)
}
