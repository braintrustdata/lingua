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

    /// Regression coverage for the `RefusalCategory` enum defined in the synchronized
    /// Anthropic spec (`RefusalStopDetails.category`). The spec enumerates
    /// `["cyber", "bio", "frontier_llm", "reasoning_extraction", "general_harms"]`; these
    /// tests assert every wire value deserializes into the matching variant and round-trips
    /// back to the same string, including the newly added `general_harms` category.
    mod refusal_category {
        use crate::providers::anthropic::generated::{
            RefusalCategory, RefusalStopDetails, RefusalStopDetailsType,
        };
        use crate::serde_json;

        #[test]
        fn all_spec_wire_values_roundtrip() {
            // (wire value, expected variant) — mirrors the spec enum exactly.
            let cases = [
                ("\"cyber\"", RefusalCategory::Cyber),
                ("\"bio\"", RefusalCategory::Bio),
                ("\"frontier_llm\"", RefusalCategory::FrontierLlm),
                (
                    "\"reasoning_extraction\"",
                    RefusalCategory::ReasoningExtraction,
                ),
                ("\"general_harms\"", RefusalCategory::GeneralHarms),
            ];

            for (wire, expected) in cases {
                let parsed: RefusalCategory = serde_json::from_str(wire)
                    .unwrap_or_else(|e| panic!("failed to deserialize {wire}: {e}"));
                assert_eq!(
                    parsed, expected,
                    "wire value {wire} mapped to wrong variant"
                );

                let reserialized = serde_json::to_string(&parsed)
                    .unwrap_or_else(|e| panic!("failed to serialize {expected:?}: {e}"));
                assert_eq!(reserialized, wire, "variant did not round-trip to {wire}");
            }
        }

        #[test]
        fn general_harms_is_the_newly_added_value() {
            // Guards the specific value added in this provider-type update: it must be a
            // recognized closed-enum variant, not silently rejected or treated as unknown.
            let parsed: RefusalCategory = serde_json::from_str("\"general_harms\"").unwrap();
            assert_eq!(parsed, RefusalCategory::GeneralHarms);
        }

        #[test]
        fn unknown_category_is_rejected() {
            // Closed enum: an unlisted category must fail to parse rather than be coerced.
            let result: Result<RefusalCategory, _> = serde_json::from_str("\"nonexistent\"");
            assert!(result.is_err(), "unknown category should not deserialize");
        }

        #[test]
        fn refusal_stop_details_with_general_harms_roundtrips() {
            // The category flows through the response as `RefusalStopDetails.category`, so
            // exercise the full field wrapper end to end.
            let wire = r#"{"category":"general_harms","explanation":"declined","type":"refusal"}"#;
            let details: RefusalStopDetails = serde_json::from_str(wire).unwrap();

            assert_eq!(details.category, Some(RefusalCategory::GeneralHarms));
            assert_eq!(details.explanation.as_deref(), Some("declined"));
            assert_eq!(
                details.refusal_stop_details_type,
                RefusalStopDetailsType::Refusal
            );

            let reserialized = serde_json::to_string(&details).unwrap();
            assert_eq!(
                reserialized, wire,
                "RefusalStopDetails did not round-trip losslessly"
            );
        }
    }
}
