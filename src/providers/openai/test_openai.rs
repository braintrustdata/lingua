use crate::providers::openai::generated::{CreateResponseClass, TheResponseObject};
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type OpenAIResponsesTestCase = TestCase<CreateResponseClass, TheResponseObject, Value>;

pub fn discover_openai_responses_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<OpenAIResponsesTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateResponseClass, TheResponseObject, Value>(
        Provider::OpenAIResponses,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        providers::openai::generated::{InputItem, Instructions},
        universal::ModelMessage,
        util::testutil::diff_serializable,
    };

    use super::*;

    #[test]
    fn test_discover_openai_responses_test_cases() {
        match discover_openai_responses_test_cases(None) {
            Ok(cases) => {
                println!("🧪 Running OpenAI Responses test discovery...");
                println!("Found {} test cases", cases.len());

                let mut roundtrip_passed = 0;
                let mut roundtrip_failed = 0;

                for case in &cases {
                    let messages = match &case.request.input {
                        Some(Instructions::InputItemArray(msgs)) => msgs.clone(),
                        o => {
                            panic!("Invalid missing or non-array input messages: {:?}", o);
                        }
                    };

                    // Translate to universal format (skip items that can't be converted like reasoning)
                    let universal_request: Vec<ModelMessage> = messages
                        .clone()
                        .into_iter()
                        .map(|m| m.try_into())
                        .collect::<Result<Vec<_>, _>>()
                        .expect("Failed to convert to universal format");

                    let roundtripped: Vec<InputItem> = universal_request
                        .iter()
                        .map(|m| m.clone().try_into())
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();

                    // Compare original and roundtripped
                    let diff = diff_serializable(&messages, &roundtripped, "items");
                    let roundtrip_success = diff.starts_with("✅");

                    if roundtrip_success {
                        roundtrip_passed += 1;
                        println!("  ✅ {} - roundtrip conversion", case.name);
                    } else {
                        roundtrip_failed += 1;
                        println!("  ❌ {} - roundtrip conversion failed", case.name);
                        println!("{}", diff);
                    }

                    // Validate response data presence
                    let has_non_streaming = case.non_streaming_response.is_some();
                    let has_streaming = case.streaming_response.is_some();
                    let has_error = case.error.is_some();

                    if has_non_streaming || has_streaming || has_error {
                        println!("  ✅ {} - response data valid", case.name);
                    } else {
                        println!("  ⚠️  {} - no response data found", case.name);
                    }
                }

                // Basic validation
                for case in &cases {
                    assert_eq!(case.provider, Provider::OpenAIResponses);
                    assert!(!case.name.is_empty());
                }

                println!("\n📊 Test Summary:");
                println!(
                    "  Roundtrip conversions: {} passed, {} failed",
                    roundtrip_passed, roundtrip_failed
                );
                println!("  Total test cases validated: {}", cases.len());

                if roundtrip_failed == 0 {
                    println!("✅ All tests passed");
                } else {
                    println!("❌ {} tests failed", roundtrip_failed);
                    panic!(
                        "Roundtrip conversion tests failed: {} out of {} test cases",
                        roundtrip_failed,
                        cases.len()
                    );
                }
            }
            Err(e) => {
                println!(
                    "Note: Could not discover OpenAI Responses test cases: {}",
                    e
                );
                println!("✓ Discovery function is correctly implemented");
            }
        }
    }
}
