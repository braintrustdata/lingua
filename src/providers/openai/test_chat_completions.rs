use crate::providers::openai::generated::{
    CreateChatCompletionRequestClass, CreateChatCompletionResponse,
};
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type OpenAIChatCompletionsTestCase =
    TestCase<CreateChatCompletionRequestClass, CreateChatCompletionResponse, Value>;

pub fn discover_openai_chat_completions_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<OpenAIChatCompletionsTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateChatCompletionRequestClass, CreateChatCompletionResponse, Value>(
        Provider::OpenAIChatCompletions,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        providers::openai::generated::ChatCompletionRequestMessage,
        universal::{convert::TryFromLLM, Message},
        util::testutil::diff_serializable,
    };
    use log::{debug, info};

    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        // Initialize env_logger if not already done
        let _ = env_logger::try_init();

        let cases = discover_openai_chat_completions_test_cases(None)
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

        let universal_request: Vec<Message> = <Vec<Message> as TryFromLLM<
            Vec<ChatCompletionRequestMessage>,
        >>::try_from(messages.clone())
        .map_err(|e| format!("Failed to convert to universal format: {}", e))?;

        debug!("✓ Universal: {} Messages", universal_request.len());
        debug!(
            "\n{}",
            serde_json::to_string_pretty(&universal_request).unwrap()
        );

        debug!("↩️  Converting back to ChatCompletionRequestMessage...");

        let roundtripped: Vec<ChatCompletionRequestMessage> =
            <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                universal_request.clone(),
            )
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

            // Response messages are in the choices[].message field
            let response_messages: Vec<_> = response
                .choices
                .iter()
                .map(|choice| &choice.message)
                .collect();

            debug!("📄 Response Original: {} Messages", response_messages.len());
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&response_messages).unwrap()
            );

            debug!("🔄 Converting response to universal format...");

            // Convert each response message to universal format
            let mut universal_response_messages = Vec::new();
            for response_message in &response_messages {
                let universal_msg: Message = <Message as TryFromLLM<
                    &crate::providers::openai::generated::ChatCompletionResponseMessage,
                >>::try_from(*response_message)
                .map_err(|e| format!("Failed to convert response to universal format: {}", e))?;
                universal_response_messages.push(universal_msg);
            }

            debug!(
                "✓ Universal Response: {} Messages",
                universal_response_messages.len()
            );
            debug!(
                "\n{}",
                serde_json::to_string_pretty(&universal_response_messages).unwrap()
            );

            debug!("↩️  Converting response back to ChatCompletionResponseMessage...");

            // Convert back to response message format
            let mut roundtripped_response_messages = Vec::new();
            for universal_msg in &universal_response_messages {
                let response_msg: crate::providers::openai::generated::ChatCompletionResponseMessage =
                    <crate::providers::openai::generated::ChatCompletionResponseMessage as TryFromLLM<&Message>>::try_from(universal_msg)
                        .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))?;
                roundtripped_response_messages.push(response_msg);
            }

            debug!(
                "\n{}",
                serde_json::to_string_pretty(&roundtripped_response_messages).unwrap()
            );

            let response_diff = diff_serializable(
                &response_messages,
                &roundtripped_response_messages,
                "response messages",
            );
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
        include!(concat!(
            env!("OUT_DIR"),
            "/generated_chat_completions_tests.rs"
        ));
    }
}
