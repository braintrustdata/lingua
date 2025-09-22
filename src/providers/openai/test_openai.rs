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
        Provider::OpenAIResponses,
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
            // Extract response content (output messages from OpenAI Responses API converted to InputItems)
            |response: &TheResponseObject| -> Result<Vec<InputItem>, String> {
                // Convert OutputItems to InputItems for comparison (this is the supported direction)
                let input_items_from_output: Vec<InputItem> = response
                    .output
                    .iter()
                    .map(|output_item| {
                        <InputItem as TryFromLLM<OutputItem>>::try_from(output_item.clone())
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("Failed to convert OutputItem to InputItem: {}", e))?;
                Ok(input_items_from_output)
            },
            // Convert response to universal (InputItems are already converted from OutputItems)
            |input_messages: &Vec<InputItem>| {
                // Convert InputItems to universal format
                <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(input_messages.clone())
                    .map_err(|e| format!("Failed to convert InputItems to universal format: {}", e))
            },
            // Convert universal to response (via InputItem conversion - note: we compare InputItems, not OutputItems)
            |messages: Vec<Message>| {
                // Convert universal messages back to InputItems (this is what we can compare)
                <Vec<InputItem> as TryFromLLM<Vec<Message>>>::try_from(messages)
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
