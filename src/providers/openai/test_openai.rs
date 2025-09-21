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

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_openai_responses_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

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
        Ok(())
    }

    // Include auto-generated test cases from build script
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }
}
