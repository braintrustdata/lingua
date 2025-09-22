use crate::providers::anthropic::generated::{CreateMessageParams, Message};
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type AnthropicTestCase = TestCase<CreateMessageParams, Message, Value>;

pub fn discover_anthropic_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<AnthropicTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateMessageParams, Message, Value>(
        Provider::Anthropic,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        providers::anthropic::generated::InputMessage,
        universal::{convert::TryFromLLM, Message},
        util::testutil::diff_serializable,
    };
    use log::{debug, info};

    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        // Initialize env_logger if not already done
        let _ = env_logger::try_init();

        let cases = discover_anthropic_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        info!("🧪 Testing roundtrip conversion for: {}", case.name);

        let messages = &case.request.messages;

        // Log conversion steps
        debug!("📄 Original: {} Messages", messages.len());
        debug!("\n{}", serde_json::to_string_pretty(&messages).unwrap());

        debug!("🔄 Converting to universal format...");

        let universal_request: Vec<Message> =
            <Vec<Message> as TryFromLLM<Vec<InputMessage>>>::try_from(messages.clone())
                .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

        debug!("✓ Universal: {} Messages", universal_request.len());
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&universal_request).unwrap()
        );

        debug!("↩️  Converting back to InputMessage...");

        let roundtripped: Vec<InputMessage> =
            <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(universal_request.clone())
                .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;

        debug!("\n{}", serde_json::to_string_pretty(&roundtripped).unwrap());

        let diff = diff_serializable(&messages, &roundtripped, "messages");
        if !diff.starts_with("✅") {
            return Err(format!("Roundtrip conversion failed:\n{}", diff));
        }

        println!("✅ {} - request roundtrip conversion passed", case.name);

        // Test response conversion if available
        if let Some(response) = &case.non_streaming_response {
            info!("🧪 Testing response conversion for: {}", case.name);

            // Response content is in the message content field
            let response_content = &response.content;

            debug!(
                "📄 Response Original: {} Content Blocks",
                response_content.len()
            );
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&response_content).unwrap()
            );

            debug!("🔄 Converting response to universal format...");

            // Convert response content to universal format
            let universal_response: Vec<Message> = <Vec<Message> as TryFromLLM<
                &Vec<crate::providers::anthropic::generated::ContentBlock>,
            >>::try_from(response_content)
            .map_err(|e| format!("Failed to convert response to universal format: {}", e))?;

            debug!(
                "✓ Universal Response: {} Messages",
                universal_response.len()
            );
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&universal_response).unwrap()
            );

            debug!("↩️  Converting response back to ContentBlock...");

            // Convert back to response content format
            let roundtripped_response: Vec<crate::providers::anthropic::generated::ContentBlock> =
                <Vec<crate::providers::anthropic::generated::ContentBlock> as TryFromLLM<
                    Vec<Message>,
                >>::try_from(universal_response.clone())
                .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))?;

            debug!(
                "\n{}",
                serde_json::to_string_pretty(&roundtripped_response).unwrap()
            );

            let response_diff =
                diff_serializable(response_content, &roundtripped_response, "response content");
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
        include!(concat!(env!("OUT_DIR"), "/generated_anthropic_tests.rs"));
    }
}
