use crate::providers::openai::convert::{
    ChatCompletionRequestMessageExt, ChatCompletionResponseMessageExt,
};
use crate::providers::openai::generated::{
    CreateChatCompletionRequestClass, CreateChatCompletionResponse,
};
use crate::serde_json::Value;
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};

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
            // Extract messages from request (convert to extended type)
            |request: &CreateChatCompletionRequestClass| Ok(&request.messages),
            // Convert to universal (via extended type)
            |messages: &Vec<crate::providers::openai::generated::ChatCompletionRequestMessage>| {
                // Wrap base messages in extended type for conversion
                let ext_messages: Vec<ChatCompletionRequestMessageExt> = messages
                    .iter()
                    .map(|m| ChatCompletionRequestMessageExt {
                        base: m.clone(),
                        reasoning: None,
                        reasoning_signature: None,
                    })
                    .collect();
                <Vec<Message> as TryFromLLM<Vec<ChatCompletionRequestMessageExt>>>::try_from(
                    ext_messages,
                )
                .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                let ext_messages = <Vec<ChatCompletionRequestMessageExt> as TryFromLLM<
                    Vec<Message>,
                >>::try_from(messages)
                .map_err(|e| format!("Failed to roundtrip conversion: {}", e))?;
                // Extract base messages (reasoning would be in separate field)
                Ok(ext_messages.into_iter().map(|m| m.base).collect())
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
            // Convert response to universal (via extended type)
            |response_messages: &Vec<
                crate::providers::openai::generated::ChatCompletionResponseMessage,
            >| {
                let mut universal_messages = Vec::new();
                for response_message in response_messages {
                    // Wrap base message in extended type for conversion
                    let ext_msg = ChatCompletionResponseMessageExt {
                        base: response_message.clone(),
                        reasoning: None,
                        reasoning_signature: None,
                    };
                    let universal_msg: Message = <Message as TryFromLLM<
                        ChatCompletionResponseMessageExt,
                    >>::try_from(ext_msg)
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
                    let ext_msg: ChatCompletionResponseMessageExt =
                        <ChatCompletionResponseMessageExt as TryFromLLM<&Message>>::try_from(
                            universal_msg,
                        )
                        .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))?;
                    // Extract base message (reasoning would be in separate field)
                    response_messages.push(ext_msg.base);
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
