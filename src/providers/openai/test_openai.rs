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

    // Helper function to run a single roundtrip test case
    fn run_single_roundtrip_test(case_name_filter: &str) -> Result<(), String> {
        let cases = discover_openai_responses_test_cases(Some(case_name_filter))
            .map_err(|e| format!("Failed to discover test case: {}", e))?;

        if cases.is_empty() {
            return Err(format!("No test case found matching: {}", case_name_filter));
        }

        for case in cases {
            println!("🧪 Testing roundtrip conversion for: {}", case.name);

            let messages = match &case.request.input {
                Some(Instructions::InputItemArray(msgs)) => msgs.clone(),
                o => {
                    return Err(format!(
                        "Invalid missing or non-array input messages: {:?}",
                        o
                    ));
                }
            };

            let universal_request: Vec<ModelMessage> = messages
                .clone()
                .into_iter()
                .map(|m| m.try_into())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

            let roundtripped: Vec<InputItem> = universal_request
                .iter()
                .map(|m| m.clone().try_into())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;

            let diff = diff_serializable(&messages, &roundtripped, "items");
            if !diff.starts_with("✅") {
                return Err(format!("Roundtrip conversion failed:\n{}", diff));
            }

            println!("✅ {} - roundtrip conversion passed", case.name);
        }

        Ok(())
    }

    // Individual test cases for granular filtering
    #[test]
    fn test_roundtrip_simple_request_first_turn() {
        if let Err(e) = run_single_roundtrip_test("simple_request") {
            // Filter to just the first turn case
            let cases = discover_openai_responses_test_cases(Some("simple_request")).unwrap();
            let first_turn_case = cases.iter().find(|c| c.name.contains("first_turn"));
            if let Some(case) = first_turn_case {
                panic!("First turn test failed for {}: {}", case.name, e);
            } else {
                panic!("No first turn case found: {}", e);
            }
        }
    }

    #[test]
    fn test_roundtrip_simple_request_followup_turn() {
        if let Err(e) = run_single_roundtrip_test("simple_request") {
            let cases = discover_openai_responses_test_cases(Some("simple_request")).unwrap();
            let followup_case = cases.iter().find(|c| c.name.contains("followup_turn"));
            if let Some(case) = followup_case {
                panic!("Followup turn test failed for {}: {}", case.name, e);
            } else {
                panic!("No followup turn case found: {}", e);
            }
        }
    }

    // Dynamic test generation for any discovered test cases
    mod generated {
        use super::*;

        #[test]
        fn test_enumerate_all_roundtrip_scenarios() {
            let cases = match discover_openai_responses_test_cases(None) {
                Ok(cases) => cases,
                Err(e) => {
                    println!("Note: Could not discover test cases: {}", e);
                    return;
                }
            };

            println!("📋 Available test scenarios:");
            for case in &cases {
                println!("  - {} ({})", case.name, case.turn.display_name());
            }

            println!("\n💡 To run individual scenarios:");
            let unique_bases: std::collections::HashSet<_> = cases
                .iter()
                .map(|c| {
                    // Extract base name (everything before provider and turn info)
                    let parts: Vec<_> = c.name.split('_').collect();
                    if parts.len() >= 3 {
                        parts[0] // First part is the test case base name
                    } else {
                        &c.name
                    }
                })
                .collect();

            for base in unique_bases {
                println!("  cargo test test_roundtrip_{}_", base);
            }

            // Also run them all individually and report which ones fail
            let mut failed_cases = Vec::new();

            for case in cases {
                let base_name = {
                    let parts: Vec<_> = case.name.split('_').collect();
                    if parts.len() >= 3 {
                        parts[0] // First part is the test case base name
                    } else {
                        &case.name
                    }
                };
                match run_single_roundtrip_test(base_name) {
                    Ok(()) => {
                        println!("✅ {} passed", case.name);
                    }
                    Err(e) => {
                        println!("❌ {} failed", case.name);
                        failed_cases.push((case.name, e));
                    }
                }
            }

            if !failed_cases.is_empty() {
                let failure_summary = failed_cases
                    .iter()
                    .map(|(name, err)| format!("\n{}: {}", name, err))
                    .collect::<String>();

                panic!("Individual roundtrip tests failed:{}", failure_summary);
            }
        }
    }
}
