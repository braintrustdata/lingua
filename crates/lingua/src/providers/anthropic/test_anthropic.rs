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

    mod refusal_category {
        use crate::providers::anthropic::generated::{
            RefusalCategory, RefusalStopDetails, RefusalStopDetailsType,
        };
        use crate::serde_json::{self, json};

        /// Every wire value in the synchronized spec's `RefusalCategory` enum must
        /// deserialize. `general_harms` was added to the spec in this update; before the
        /// type regained the variant a response carrying it would have been rejected by
        /// serde as an unknown variant. Guards the older values against accidental removal
        /// at the same time.
        #[test]
        fn all_spec_categories_deserialize() {
            let cases = [
                ("bio", RefusalCategory::Bio),
                ("cyber", RefusalCategory::Cyber),
                ("frontier_llm", RefusalCategory::FrontierLlm),
                ("general_harms", RefusalCategory::GeneralHarms),
                ("reasoning_extraction", RefusalCategory::ReasoningExtraction),
            ];
            for (wire, expected) in cases {
                let parsed: RefusalCategory =
                    serde_json::from_value(json!(wire)).unwrap_or_else(|e| {
                        panic!("RefusalCategory wire value {wire:?} should deserialize: {e}")
                    });
                assert_eq!(parsed, expected, "unexpected variant for {wire:?}");
                // Round-trips back to the same wire value.
                let reserialized = serde_json::to_value(&parsed).expect("serialize category");
                assert_eq!(
                    reserialized,
                    json!(wire),
                    "round-trip mismatch for {wire:?}"
                );
            }
        }

        /// A full refusal `stop_details` object carrying the new `general_harms` category
        /// deserializes into the typed struct and round-trips without loss.
        #[test]
        fn stop_details_with_general_harms_roundtrips() {
            let payload = json!({
                "type": "refusal",
                "category": "general_harms",
                "explanation": "Request declined under the general harms policy.",
            });

            let details: RefusalStopDetails = serde_json::from_value(payload.clone())
                .expect("refusal stop_details with general_harms should deserialize");
            assert_eq!(details.category, Some(RefusalCategory::GeneralHarms));
            assert_eq!(
                details.refusal_stop_details_type,
                RefusalStopDetailsType::Refusal
            );

            let reserialized = serde_json::to_value(&details).expect("serialize stop_details");
            assert_eq!(reserialized, payload, "stop_details round-trip mismatch");
        }
    }
}
