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

        // Log original input with verbose details
        debug!("📄 Original InputItems ({} items):", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            debug!(
                "  [{}] {}",
                i,
                serde_json::to_string_pretty(msg)
                    .unwrap_or_else(|e| format!("Failed to serialize: {}", e))
            );
        }

        let universal_request: Vec<ModelMessage> = messages
            .clone()
            .into_iter()
            .map(|m| m.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

        // Log universal format
        debug!(
            "🔄 Converted to Universal ModelMessages ({} items):",
            universal_request.len()
        );
        for (i, msg) in universal_request.iter().enumerate() {
            debug!("  [{}] {:?}", i, msg);
        }

        let roundtripped: Vec<InputItem> = universal_request
            .iter()
            .map(|m| m.clone().try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;

        // Log roundtripped result
        debug!(
            "↩️  Roundtripped back to InputItems ({} items):",
            roundtripped.len()
        );
        for (i, msg) in roundtripped.iter().enumerate() {
            debug!(
                "  [{}] {}",
                i,
                serde_json::to_string_pretty(msg)
                    .unwrap_or_else(|e| format!("Failed to serialize: {}", e))
            );
        }

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
