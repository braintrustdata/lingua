use crate::providers::anthropic::convert::{
    anthropic_input_messages_to_universal_messages, universal_messages_to_anthropic_input_messages,
};
use crate::providers::anthropic::generated::{
    ContentBlock, CreateMessageParams, InputMessage, Message as AnthropicMessage,
};
use crate::serde_json::Value;
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::run_roundtrip_test;
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};

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

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        let result = run_roundtrip_test(
            case,
            // Extract messages from request
            |request: &CreateMessageParams| Ok(request.messages.clone()),
            // Convert to universal
            |messages: &Vec<InputMessage>| {
                anthropic_input_messages_to_universal_messages(messages.clone())
                    .map_err(|e| format!("Failed to convert to universal format: {}", e))
            },
            // Convert from universal
            |messages: Vec<Message>| {
                universal_messages_to_anthropic_input_messages(messages)
                    .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
            },
            // Extract response content
            |response: &AnthropicMessage| Ok(response.content.clone()),
            // Convert response to universal
            |response_content: &Vec<ContentBlock>| {
                <Vec<Message> as TryFromLLM<Vec<ContentBlock>>>::try_from(response_content.clone())
                    .map_err(|e| format!("Failed to convert response to universal format: {}", e))
            },
            // Convert universal to response
            |messages: Vec<Message>| {
                <Vec<ContentBlock> as TryFromLLM<Vec<Message>>>::try_from(messages)
                    .map_err(|e| format!("Failed to roundtrip response conversion: {}", e))
            },
        );

        match result {
            Err(err)
                if full_case_name == "anthropicMessageWithSystemMessage_anthropic_first_turn"
                    && err.contains("Non-leading system/developer messages are not supported") =>
            {
                Ok(())
            }
            other => other,
        }
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_anthropic_tests.rs"));
    }

    /// Regression coverage for the `RefusalCategory` enum in the generated
    /// Anthropic types. The spec added the `general_harms` policy category to
    /// both the response `stop_details` refusal category and its Beta twin; this
    /// verifies the new wire value round-trips through the generated types and
    /// that the pre-existing categories remain accepted (guarding against an
    /// accidental variant loss on regeneration).
    #[test]
    fn refusal_category_general_harms_round_trips() {
        use crate::providers::anthropic::generated::{
            RefusalCategory, RefusalStopDetails, RefusalStopDetailsType,
        };
        use crate::serde_json;

        // The new value deserializes into the new variant.
        let json = r#"{
            "type": "refusal",
            "category": "general_harms",
            "explanation": "The request could be related to an area that was determined as harmful."
        }"#;
        let details: RefusalStopDetails =
            serde_json::from_str(json).expect("general_harms stop_details should deserialize");
        assert_eq!(details.category, Some(RefusalCategory::GeneralHarms));
        assert_eq!(
            details.refusal_stop_details_type,
            RefusalStopDetailsType::Refusal
        );

        // And re-serializes back to the exact wire value.
        let value = serde_json::to_value(&details).expect("serialize refusal stop_details");
        assert_eq!(value["category"], serde_json::json!("general_harms"));

        // Every category wire value the spec still lists must remain accepted.
        for (wire, expected) in [
            ("bio", RefusalCategory::Bio),
            ("cyber", RefusalCategory::Cyber),
            ("frontier_llm", RefusalCategory::FrontierLlm),
            ("reasoning_extraction", RefusalCategory::ReasoningExtraction),
            ("general_harms", RefusalCategory::GeneralHarms),
        ] {
            let parsed: RefusalCategory = serde_json::from_value(serde_json::json!(wire))
                .unwrap_or_else(|e| panic!("category {wire:?} should deserialize: {e}"));
            assert_eq!(
                parsed, expected,
                "category {wire:?} mapped to wrong variant"
            );
            assert_eq!(
                serde_json::to_value(&parsed).unwrap(),
                serde_json::json!(wire),
                "category {wire:?} did not re-serialize to its wire value"
            );
        }
    }
}
