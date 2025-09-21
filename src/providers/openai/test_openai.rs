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
        universal::{convert::TryConvert, Message},
        util::testutil::diff_serializable,
    };
    use log::{debug, info};

    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        // Initialize env_logger if not already done
        let _ = env_logger::try_init();

        let cases = discover_openai_responses_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        info!("🧪 Testing roundtrip conversion for: {}", case.name);

        let messages = match &case.request.input {
            Some(Instructions::InputItemArray(msgs)) => msgs.clone(),
            o => {
                return Err(format!(
                    "Invalid missing or non-array input messages: {:?}",
                    o
                ));
            }
        };

        // Log conversion steps
        debug!("📄 Original: {} InputItems", messages.len());
        debug!("\n{}", serde_json::to_string_pretty(&messages).unwrap());

        debug!("🔄 Converting to universal format...");

        let universal_request: Vec<Message> = Vec::<Message>::try_convert(messages.clone())
            .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

        debug!("✓ Universal: {} Messages", universal_request.len());
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&universal_request).unwrap()
        );

        debug!("↩️  Converting back to InputItems...");

        let roundtripped: Vec<InputItem> = Vec::<InputItem>::try_convert(universal_request.clone())
            .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;

        debug!("\n{}", serde_json::to_string_pretty(&roundtripped).unwrap());

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
