use crate::providers::openai::generated::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, CreateChatCompletionRequestClass,
    CreateChatCompletionResponse,
};
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type OpenAIChatCompletionsTestCase =
    TestCase<CreateChatCompletionRequestClass, CreateChatCompletionResponse, Value>;

pub fn discover_openai_chat_completions_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<OpenAIChatCompletionsTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateChatCompletionRequestClass, CreateChatCompletionResponse, Value>(
        Provider::ChatCompletions,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_openai_chat_completions_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test(
            case,
            // Extract messages from request
            |request: &CreateChatCompletionRequestClass| Ok(&request.messages),
            // Convert to universal
            |messages: &Vec<ChatCompletionRequestMessage>| {
                <Vec<Message> as TryFromLLM<Vec<ChatCompletionRequestMessage>>>::try_from(
                    messages.clone(),
                )
                .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
            },
            // Extract response content (collect response messages from choices)
            |response: &CreateChatCompletionResponse| {
                let response_messages: Vec<_> = response
                    .choices
                    .iter()
                    .map(|choice| choice.message.clone())
                    .collect();
                Ok(response_messages)
            },
            // Convert response to universal
            |response_messages: &Vec<ChatCompletionResponseMessage>| {
                let mut universal_messages = Vec::new();
                for response_message in response_messages {
                    let universal_msg: Message = <Message as TryFromLLM<
                        &ChatCompletionResponseMessage,
                    >>::try_from(response_message)
                    .map_err(|e| {
                        format!("Failed to convert response to universal format: {}", e)
                    })?;
                    universal_messages.push(universal_msg);
                }
                Ok(universal_messages)
            },
            // Convert universal to response
            |messages: Vec<Message>| {
                let mut response_messages = Vec::new();
                for universal_msg in &messages {
                    let response_msg: ChatCompletionResponseMessage =
                        <ChatCompletionResponseMessage as TryFromLLM<&Message>>::try_from(
                            universal_msg,
                        )
                        .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))?;
                    response_messages.push(response_msg);
                }
                Ok(response_messages)
            },
        )
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
