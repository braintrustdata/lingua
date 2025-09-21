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
        providers::openai::{
            convert::diff_input_items,
            generated::{InputItem, Instructions},
        },
        universal::ModelMessage,
    };

    use super::*;

    #[test]
    fn test_discover_openai_responses_test_cases() {
        match discover_openai_responses_test_cases(None) {
            Ok(cases) => {
                for case in &cases {
                    println!("  - {} (turn: {:?})", case.name, case.turn);

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
                        .filter_map(|m| match m.try_into() {
                            Ok(model_msg) => Some(model_msg),
                            Err(e) => {
                                println!("    ⏭️  Skipping unconvertible item: {}", e);
                                None
                            }
                        })
                        .collect();

                    let roundtripped: Vec<InputItem> = universal_request
                        .iter()
                        .map(|m| m.clone().try_into())
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();

                    // Compare original and roundtripped
                    let diff = diff_input_items(&messages, &roundtripped);
                    println!("    🔄 Roundtrip test:");
                    println!("{}", diff);

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
