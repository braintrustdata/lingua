use crate::providers::openai::generated::{CreateResponseClass, OutputItem, TheResponseObject};
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
        universal::{convert::TryFromLLM, Message},
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

        let universal_request: Vec<Message> =
            <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(messages.clone())
                .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

        debug!("✓ Universal: {} Messages", universal_request.len());
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&universal_request).unwrap()
        );

        debug!("↩️  Converting back to InputItems...");

        let roundtripped: Vec<InputItem> =
            <Vec<InputItem> as TryFromLLM<Vec<Message>>>::try_from(universal_request.clone())
                .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;

        debug!("\n{}", serde_json::to_string_pretty(&roundtripped).unwrap());

        let diff = diff_serializable(&messages, &roundtripped, "items");
        if !diff.starts_with("✅") {
            return Err(format!("Roundtrip conversion failed:\n{}", diff));
        }

        println!("✅ {} - request roundtrip conversion passed", case.name);

        // Test response conversion if available
        if let Some(response) = &case.non_streaming_response {
            info!("🧪 Testing response conversion for: {}", case.name);

            // Response messages are in the output field, not input
            let response_messages = &response.output;

            debug!(
                "📄 Response Original: {} OutputItems",
                response_messages.len()
            );
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&response_messages).unwrap()
            );

            // Convert OutputItem to InputItem using proper conversion
            let response_as_input: Vec<InputItem> = response_messages
                .iter()
                .map(|output_item| {
                    <InputItem as TryFromLLM<OutputItem>>::try_from(output_item.clone()).unwrap()
                })
                .collect();

            debug!("🔄 Converting response to universal format...");

            let universal_response: Vec<Message> =
                <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(response_as_input.clone())
                    .map_err(|e| format!("Failed to convert response to universal format: {}", e))?;

            debug!(
                "✓ Universal Response: {} Messages",
                universal_response.len()
            );
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&universal_response).unwrap()
            );

            debug!("↩️  Converting response back to InputItems...");

            let roundtripped_response: Vec<InputItem> =
                <Vec<InputItem> as TryFromLLM<Vec<Message>>>::try_from(universal_response.clone())
                    .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))?;

            debug!(
                "\n{}",
                serde_json::to_string_pretty(&roundtripped_response).unwrap()
            );

            let response_diff =
                diff_serializable(&response_as_input, &roundtripped_response, "response items");
            if !response_diff.starts_with("✅") {
                return Err(format!(
                    "Response roundtrip conversion failed:\n{}",
                    response_diff
                ));
            }

            println!("✅ {} - response roundtrip conversion passed", case.name);
        }

        println!("✅ {} - all conversions passed", case.name);
        Ok(())
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }
}
