use crate::providers::openai::generated::{
    CreateResponseClass, InputItem, Instructions, OutputItem, TheResponseObject,
};
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};
use serde_json::Value;

pub type OpenAIResponsesTestCase = TestCase<CreateResponseClass, TheResponseObject, Value>;

pub fn discover_openai_responses_test_cases(
    test_name_filter: Option<&str>,
) -> Result<Vec<OpenAIResponsesTestCase>, crate::util::testutil::TestDiscoveryError> {
    discover_test_cases_typed::<CreateResponseClass, TheResponseObject, Value>(
        Provider::Responses,
        test_name_filter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_openai_responses_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test(
            case,
            // Extract messages from request (OpenAI Responses API has complex input structure)
            |request: &CreateResponseClass| match &request.input {
                Some(Instructions::InputItemArray(msgs)) => Ok(msgs),
                o => Err(format!(
                    "Invalid missing or non-array input messages: {:?}",
                    o
                )),
            },
            // Convert to universal
            |messages: &Vec<InputItem>| {
                <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(messages.clone())
                    .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                <Vec<InputItem> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
            },
            // Extract response content (output messages from OpenAI Responses API)
            |response: &TheResponseObject| -> Result<Vec<OutputItem>, String> {
                Ok(response.output.clone())
            },
            // Convert response to universal (OutputItems to Messages)
            |output_items: &Vec<OutputItem>| {
                <Vec<Message> as TryFromLLM<Vec<OutputItem>>>::try_from(output_items.clone())
                    .map_err(|e| {
                        format!("Failed to convert OutputItems to universal format: {}", e)
                    })
            },
            // Convert universal to response (Messages to OutputItems)
            |messages: Vec<Message>| {
                <Vec<OutputItem> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion from universal: {}", e))
            },
        )
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }
}
