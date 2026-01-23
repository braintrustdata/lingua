/*!
Test execution for coverage-report.
*/

use std::collections::HashMap;

use bytes::Bytes;
use lingua::capabilities::ProviderFormat;
use lingua::processing::adapters::ProviderAdapter;
use lingua::processing::transform::{
    transform_request, transform_response, transform_stream_chunk,
};
use lingua::serde_json::Value;

use crate::discovery::{discover_test_cases, load_payload};
use crate::types::{PairResult, TransformResult, ValidationLevel};

type PairResults = HashMap<(usize, usize), PairResult>;
type AllResults = (PairResults, PairResults, PairResults);

// Patterns that indicate provider limitations (real gaps, not bugs)
const LIMITATION_PATTERNS: &[&str] = &[
    "Provider limitation",
    "has no OpenAI equivalent",
    "has no Anthropic equivalent",
    "has no Bedrock equivalent",
    "has no Google equivalent",
    "Unsupported",
];

// Patterns that indicate missing test fixtures (test coverage gaps)
const MISSING_FIXTURE_PATTERNS: &[&str] = &["Source payload not found"];

/// Classify an error into failure, limitation, or missing fixture.
fn classify_error(error: &str) -> ValidationLevel {
    if MISSING_FIXTURE_PATTERNS.iter().any(|p| error.contains(p)) {
        ValidationLevel::MissingFixture
    } else if LIMITATION_PATTERNS.iter().any(|p| error.contains(p)) {
        ValidationLevel::Limitation
    } else {
        ValidationLevel::Fail
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
                level: ValidationLevel::MissingFixture,
                error: Some(error),
            };
        }
    };

    // Provide model for formats that have model in URL (Google, Bedrock)
    let model: Option<&str> = match source_adapter.format() {
        ProviderFormat::Google => Some("gemini-1.5-pro"),
        ProviderFormat::Converse => Some("anthropic.claude-3-sonnet"),
        _ => None,
    };

    match transform_request(payload, target_adapter.format(), model) {
        Ok(result) => {
            if result.is_passthrough() && source_adapter.format() == target_adapter.format() {
                return TransformResult {
                    level: ValidationLevel::Pass,
                    error: None,
                };
            }

            // Parse result bytes to Value for validation
            let output_bytes = result.into_bytes();
            let transformed: Value = match lingua::serde_json::from_slice(&output_bytes) {
                Ok(v) => v,
                Err(e) => {
                    return TransformResult {
                        level: ValidationLevel::Fail,
                        error: Some(format!("Failed to parse transformed output: {}", e)),
                    }
                }
            };

            // Use request_to_universal to validate - gives detailed error info
            match target_adapter.request_to_universal(transformed) {
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
        Err(e) => {
            let error = format!("{}", e);
            let level = classify_error(&error);
            TransformResult {
                level,
                error: Some(error),
            }
        }
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

    match transform_response(payload, target_adapter.format()) {
        Ok(result) => {
            // Parse result bytes to Value for validation
            let output_bytes = result.into_bytes();
            let transformed: Value = match lingua::serde_json::from_slice(&output_bytes) {
                Ok(v) => v,
                Err(e) => {
                    return TransformResult {
                        level: ValidationLevel::Fail,
                        error: Some(format!("Failed to parse transformed output: {}", e)),
                    }
                }
            };

            // Use response_to_universal to validate - gives detailed error info
            match target_adapter.response_to_universal(transformed) {
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
    let payload_bytes = match load_payload(test_case, source_adapter.directory_name(), filename) {
        Some(p) => p,
        None => {
            // No streaming file - report as not found
            return TransformResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Streaming payload not found: {}", filename)),
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
        // Serialize each event back to bytes for the transform function
        let event_bytes = match lingua::serde_json::to_vec(event) {
            Ok(b) => Bytes::from(b),
            Err(e) => {
                return TransformResult {
                    level: ValidationLevel::Fail,
                    error: Some(format!("Event {}: failed to serialize: {}", idx, e)),
                };
            }
        };

        if let Err(e) = test_single_stream_event(event_bytes, target_adapter) {
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
    event: Bytes,
    target_adapter: &dyn ProviderAdapter,
) -> Result<(), String> {
    // Transform the event to target format
    let result =
        transform_stream_chunk(event, target_adapter.format()).map_err(|e| e.to_string())?;

    // Parse result bytes to Value for validation
    let output_bytes = result.into_bytes();
    let transformed: Value =
        lingua::serde_json::from_slice(&output_bytes).map_err(|e| e.to_string())?;

    // Validate transformed output can be parsed by target adapter
    match target_adapter.stream_to_universal(transformed) {
        Ok(Some(_chunk)) => Ok(()),
        Ok(None) => Ok(()), // Keep-alive events are valid
        Err(e) => Err(e.to_string()),
    }
}

/// Run all cross-transformation tests and collect results
pub fn run_all_tests(adapters: &[Box<dyn ProviderAdapter>]) -> AllResults {
    let test_cases = discover_test_cases();
    let mut request_results: PairResults = HashMap::new();
    let mut response_results: PairResults = HashMap::new();
    let mut streaming_results: PairResults = HashMap::new();

    // Initialize results for all pairs
    for (source_idx, _) in adapters.iter().enumerate() {
        for (target_idx, _) in adapters.iter().enumerate() {
            if source_idx != target_idx {
                request_results.insert((source_idx, target_idx), PairResult::default());
                response_results.insert((source_idx, target_idx), PairResult::default());
                streaming_results.insert((source_idx, target_idx), PairResult::default());
            }
        }
    }

    // Test each source→target pair for each test case
    for test_case in &test_cases {
        for (source_idx, source) in adapters.iter().enumerate() {
            for (target_idx, target) in adapters.iter().enumerate() {
                if source_idx == target_idx {
                    continue;
                }

                let source = source.as_ref();
                let target = target.as_ref();

                // Test first turn request
                let result = test_request_transformation(test_case, source, target, "request.json");
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
                    ValidationLevel::Limitation => {
                        pair_result.limitations += 1;
                        if let Some(error) = result.error {
                            pair_result
                                .limitation_details
                                .push((format!("{} (request)", test_case), error));
                        }
                    }
                    ValidationLevel::MissingFixture => {
                        pair_result.missing_fixtures += 1;
                        if let Some(error) = result.error {
                            pair_result
                                .missing_fixture_details
                                .push((format!("{} (request)", test_case), error));
                        }
                    }
                }

                // Test followup request if exists
                let followup_result =
                    test_request_transformation(test_case, source, target, "followup-request.json");
                if followup_result
                    .error
                    .as_ref()
                    .is_none_or(|e| !e.contains("not found"))
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
                        ValidationLevel::Limitation => {
                            pair_result.limitations += 1;
                            if let Some(error) = followup_result.error {
                                pair_result
                                    .limitation_details
                                    .push((format!("{} (followup)", test_case), error));
                            }
                        }
                        ValidationLevel::MissingFixture => {
                            pair_result.missing_fixtures += 1;
                            if let Some(error) = followup_result.error {
                                pair_result
                                    .missing_fixture_details
                                    .push((format!("{} (followup)", test_case), error));
                            }
                        }
                    }
                }

                // Test response transformation (source response transforms to target format)
                let response_result =
                    test_response_transformation(test_case, source, target, "response.json");
                let resp_pair_result = response_results.get_mut(&(source_idx, target_idx)).unwrap();

                if response_result
                    .error
                    .as_ref()
                    .is_none_or(|e| !e.contains("not found"))
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
                        ValidationLevel::Limitation => {
                            resp_pair_result.limitations += 1;
                            if let Some(error) = response_result.error {
                                resp_pair_result
                                    .limitation_details
                                    .push((format!("{} (response)", test_case), error));
                            }
                        }
                        ValidationLevel::MissingFixture => {
                            resp_pair_result.missing_fixtures += 1;
                            if let Some(error) = response_result.error {
                                resp_pair_result
                                    .missing_fixture_details
                                    .push((format!("{} (response)", test_case), error));
                            }
                        }
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
                if streaming_result
                    .error
                    .as_ref()
                    .is_none_or(|e| !e.contains("not found"))
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
                        ValidationLevel::Limitation => {
                            stream_pair_result.limitations += 1;
                            if let Some(error) = streaming_result.error {
                                stream_pair_result
                                    .limitation_details
                                    .push((format!("{} (streaming)", test_case), error));
                            }
                        }
                        ValidationLevel::MissingFixture => {
                            stream_pair_result.missing_fixtures += 1;
                            if let Some(error) = streaming_result.error {
                                stream_pair_result
                                    .missing_fixture_details
                                    .push((format!("{} (streaming)", test_case), error));
                            }
                        }
                    }
                }

                // Test followup streaming if exists
                let followup_streaming_result = test_streaming_transformation(
                    test_case,
                    source,
                    target,
                    "followup-response-streaming.json",
                );
                if followup_streaming_result
                    .error
                    .as_ref()
                    .is_none_or(|e| !e.contains("not found"))
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
                        ValidationLevel::Limitation => {
                            stream_pair_result.limitations += 1;
                            if let Some(error) = followup_streaming_result.error {
                                stream_pair_result
                                    .limitation_details
                                    .push((format!("{} (followup-streaming)", test_case), error));
                            }
                        }
                        ValidationLevel::MissingFixture => {
                            stream_pair_result.missing_fixtures += 1;
                            if let Some(error) = followup_streaming_result.error {
                                stream_pair_result
                                    .missing_fixture_details
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

// ============================================================================
// Roundtrip testing (Provider → Universal → Provider)
// ============================================================================

use crate::types::{ProviderRoundtripResult, RoundtripDiff, RoundtripResult};
use std::collections::HashSet;

/// Type alias for roundtrip results indexed by adapter index
pub type RoundtripResults = HashMap<usize, ProviderRoundtripResult>;

/// Fields that are expected to change during roundtrip and should be ignored.
/// These are typically metadata fields set by providers or computed values.
const IGNORED_FIELDS: &[&str] = &[
    "id",
    "created",
    "system_fingerprint",
    "service_tier",
    "object",
];

/// Compare two JSON values and produce a RoundtripDiff.
fn compare_values(original: &Value, roundtripped: &Value) -> RoundtripResult {
    let mut diff = RoundtripDiff::default();
    compare_recursive(original, roundtripped, "", &mut diff);

    if diff.is_empty() {
        RoundtripResult {
            level: ValidationLevel::Pass,
            error: None,
            diff: None,
        }
    } else {
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
    }
}

/// Recursively compare two JSON values and accumulate differences.
fn compare_recursive(original: &Value, roundtripped: &Value, path: &str, diff: &mut RoundtripDiff) {
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
                // Skip ignored fields
                if !IGNORED_FIELDS.contains(&key.as_str()) {
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
                // Skip ignored fields
                if !IGNORED_FIELDS.contains(&key.as_str()) {
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
                compare_recursive(&orig[*key], &round[*key], &new_path, diff);
            }
        }
        (Value::Array(orig), Value::Array(round)) => {
            // Compare array lengths
            if orig.len() != round.len() {
                diff.changed_fields.push((
                    format!("{}.length", path),
                    orig.len().to_string(),
                    round.len().to_string(),
                ));
                return;
            }

            // Compare element by element
            for (idx, (o, r)) in orig.iter().zip(round.iter()).enumerate() {
                let new_path = format!("{}[{}]", path, idx);
                compare_recursive(o, r, &new_path, diff);
            }
        }
        (Value::Null, Value::Null) => {}
        (Value::Bool(o), Value::Bool(r)) if o == r => {}
        (Value::Number(o), Value::Number(r)) if o == r => {}
        (Value::String(o), Value::String(r)) if o == r => {}
        _ => {
            // Values differ - skip if this is an ignored field
            let field_name = path.rsplit('.').next().unwrap_or(path);
            if !IGNORED_FIELDS.contains(&field_name) {
                diff.changed_fields.push((
                    path.to_string(),
                    lingua::serde_json::to_string(original).unwrap_or_else(|_| "?".to_string()),
                    lingua::serde_json::to_string(roundtripped).unwrap_or_else(|_| "?".to_string()),
                ));
            }
        }
    }
}

/// Test request roundtrip: Provider → Universal → Provider
pub fn test_request_roundtrip(
    test_case: &str,
    adapter: &dyn ProviderAdapter,
    filename: &str,
) -> Option<RoundtripResult> {
    // 1. Load payload
    let payload = load_payload(test_case, adapter.directory_name(), filename)?;

    // 2. Parse to Value
    let original: Value = match lingua::serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to parse payload: {}", e)),
                diff: None,
            });
        }
    };

    // 3. Convert to Universal
    let universal = match adapter.request_to_universal(original.clone()) {
        Ok(u) => u,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("request_to_universal failed: {}", e)),
                diff: None,
            });
        }
    };

    // 4. Convert back to provider format
    let roundtripped = match adapter.request_from_universal(&universal) {
        Ok(r) => r,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("request_from_universal failed: {}", e)),
                diff: None,
            });
        }
    };

    // 5. Compare original vs roundtripped
    Some(compare_values(&original, &roundtripped))
}

/// Test response roundtrip: Provider → Universal → Provider
pub fn test_response_roundtrip(
    test_case: &str,
    adapter: &dyn ProviderAdapter,
    filename: &str,
) -> Option<RoundtripResult> {
    // 1. Load payload
    let payload = load_payload(test_case, adapter.directory_name(), filename)?;

    // 2. Parse to Value
    let original: Value = match lingua::serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("Failed to parse payload: {}", e)),
                diff: None,
            });
        }
    };

    // 3. Convert to Universal
    let universal = match adapter.response_to_universal(original.clone()) {
        Ok(u) => u,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("response_to_universal failed: {}", e)),
                diff: None,
            });
        }
    };

    // 4. Convert back to provider format
    let roundtripped = match adapter.response_from_universal(&universal) {
        Ok(r) => r,
        Err(e) => {
            return Some(RoundtripResult {
                level: ValidationLevel::Fail,
                error: Some(format!("response_from_universal failed: {}", e)),
                diff: None,
            });
        }
    };

    // 5. Compare original vs roundtripped
    Some(compare_values(&original, &roundtripped))
}

/// Run all roundtrip tests for all providers.
pub fn run_roundtrip_tests(adapters: &[Box<dyn ProviderAdapter>]) -> RoundtripResults {
    let test_cases = discover_test_cases();
    let mut results: RoundtripResults = HashMap::new();

    // Initialize results for each adapter
    for (adapter_idx, _) in adapters.iter().enumerate() {
        results.insert(adapter_idx, ProviderRoundtripResult::default());
    }

    // Test each provider's roundtrip for each test case
    for test_case in &test_cases {
        for (adapter_idx, adapter) in adapters.iter().enumerate() {
            let adapter = adapter.as_ref();
            let provider_result = results.get_mut(&adapter_idx).unwrap();

            // Test request roundtrip
            if let Some(result) = test_request_roundtrip(test_case, adapter, "request.json") {
                match result.level {
                    ValidationLevel::Pass => provider_result.request_passed += 1,
                    ValidationLevel::Fail
                    | ValidationLevel::Limitation
                    | ValidationLevel::MissingFixture => {
                        provider_result.request_failed += 1;
                        provider_result
                            .request_failures
                            .push((format!("{} (request)", test_case), result));
                    }
                }
            }

            // Test followup request roundtrip if exists
            if let Some(result) =
                test_request_roundtrip(test_case, adapter, "followup-request.json")
            {
                match result.level {
                    ValidationLevel::Pass => provider_result.request_passed += 1,
                    ValidationLevel::Fail
                    | ValidationLevel::Limitation
                    | ValidationLevel::MissingFixture => {
                        provider_result.request_failed += 1;
                        provider_result
                            .request_failures
                            .push((format!("{} (followup-request)", test_case), result));
                    }
                }
            }

            // Test response roundtrip
            if let Some(result) = test_response_roundtrip(test_case, adapter, "response.json") {
                match result.level {
                    ValidationLevel::Pass => provider_result.response_passed += 1,
                    ValidationLevel::Fail
                    | ValidationLevel::Limitation
                    | ValidationLevel::MissingFixture => {
                        provider_result.response_failed += 1;
                        provider_result
                            .response_failures
                            .push((format!("{} (response)", test_case), result));
                    }
                }
            }
        }
    }

    results
}
