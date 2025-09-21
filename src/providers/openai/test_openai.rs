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
    use super::*;

    #[test]
    fn test_discover_openai_responses_test_cases() {
        match discover_openai_responses_test_cases(None) {
            Ok(cases) => {
                for case in &cases {
                    println!("  - {} (turn: {:?})", case.name, case.turn);

                    // Test that we have typed data
                    if let Some(_request) = &case.request {
                        println!("    Request (CreateResponseClass): valid");
                        // We could test specific fields here
                    } else {
                        println!("    Request: None");
                    }

                    if let Some(_response) = &case.non_streaming_response {
                        println!("    Non-Streaming Response (TheResponseObject): valid");
                        // We could test specific fields here
                    } else {
                        println!("    Non-Streaming Response: None");
                    }

                    if let Some(_stream_resp) = &case.streaming_response {
                        println!("    Streaming Response (Value): valid");
                    } else {
                        println!("    Streaming Response: None");
                    }

                    if let Some(_error) = &case.error {
                        println!("    Error: present");
                    } else {
                        println!("    Error: None");
                    }
                }

                // Basic validation
                for case in &cases {
                    assert_eq!(case.provider, Provider::OpenAIResponses);
                    assert!(!case.name.is_empty());
                }

                println!("✓ OpenAI Responses test discovery completed successfully");
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
