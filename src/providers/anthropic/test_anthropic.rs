use crate::providers::anthropic::generated::{
    ContentBlock, CreateMessageParams, InputMessage, Message as AnthropicMessage,
};
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type AnthropicTestCase = TestCase<CreateMessageParams, AnthropicMessage, Value>;

pub fn discover_anthropic_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<AnthropicTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateMessageParams, AnthropicMessage, Value>(
        Provider::Anthropic,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_anthropic_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        // Debug: Print all discovered test case names
        eprintln!("DEBUG: Discovered {} test cases:", cases.len());
        for case in &cases {
            eprintln!("  - {}", case.name);
        }

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test(
            case,
            // Extract messages from request
            |request: &CreateMessageParams| Ok(&request.messages),
            // Convert to universal
            |messages: &Vec<InputMessage>| {
                <Vec<Message> as TryFromLLM<Vec<InputMessage>>>::try_from(messages.clone())
                    .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
            },
            // Extract response content
            |response: &AnthropicMessage| Ok(response.content.clone()),
            // Convert response to universal
            |response_content: &Vec<ContentBlock>| {
                <Vec<Message> as TryFromLLM<&Vec<ContentBlock>>>::try_from(response_content)
                    .map_err(|e| format!("Failed to convert response to universal format: {}", e))
            },
            // Convert universal to response
            |messages: Vec<Message>| {
                <Vec<ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))
            },
        )
    }

    #[test]
    fn debug_anthropic_deserialize() {
        use serde_json;
        use std::fs;

        // Try to deserialize request.json
        let request_content =
            fs::read_to_string("payloads/snapshots/toolCallRequest/anthropic/request.json")
                .unwrap();
        let request: Result<CreateMessageParams, _> = serde_json::from_str(&request_content);
        eprintln!("Request deserialization: {:?}", request.is_ok());
        if let Err(e) = &request {
            eprintln!("Request error: {}", e);
        }

        // Try to deserialize response.json
        let response_content =
            fs::read_to_string("payloads/snapshots/toolCallRequest/anthropic/response.json")
                .unwrap();
        let response: Result<AnthropicMessage, _> = serde_json::from_str(&response_content);
        eprintln!("Response deserialization: {:?}", response.is_ok());
        if let Err(e) = &response {
            eprintln!("Response error: {}", e);
        }
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_anthropic_tests.rs"));
    }
}
