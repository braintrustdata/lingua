use crate::providers::openai::generated::{
    CreateResponseClass, InputItem, Instructions, OutputItem, TheResponseObject,
};
use crate::serde_json::Value;
use crate::universal::{convert::TryFromLLM, Message};
use crate::util::test_runner::{run_roundtrip_test_with_config, RoundtripTestConfig};
use crate::util::testutil::{discover_test_cases_typed, Provider, TestCase};

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

    fn normalize_responses_defaults(value: Value) -> Value {
        if let Ok(mut items) = crate::serde_json::from_value::<Vec<OutputItem>>(value.clone()) {
            for item in &mut items {
                normalize_output_item(item);
            }
            return match crate::serde_json::to_value(items) {
                Ok(value) => value,
                Err(_) => value,
            };
        }

        if let Ok(mut items) = crate::serde_json::from_value::<Vec<InputItem>>(value.clone()) {
            for item in &mut items {
                normalize_input_item(item);
            }
            return match crate::serde_json::to_value(items) {
                Ok(value) => value,
                Err(_) => value,
            };
        }

        if let Ok(mut item) = crate::serde_json::from_value::<OutputItem>(value.clone()) {
            normalize_output_item(&mut item);
            return match crate::serde_json::to_value(item) {
                Ok(value) => value,
                Err(_) => value,
            };
        }

        if let Ok(mut item) = crate::serde_json::from_value::<InputItem>(value.clone()) {
            normalize_input_item(&mut item);
            return match crate::serde_json::to_value(item) {
                Ok(value) => value,
                Err(_) => value,
            };
        }

        value
    }

    fn normalize_input_item(item: &mut InputItem) {
        if item.input_item_type.is_none() && item.role.is_some() && item.content.is_some() {
            item.input_item_type =
                Some(crate::providers::openai::generated::InputItemType::Message);
        }
        if item.input_item_type
            == Some(crate::providers::openai::generated::InputItemType::Reasoning)
            && item.content.is_none()
        {
            item.content = Some(
                crate::providers::openai::generated::InputItemContent::InputContentArray(vec![]),
            );
        }
    }

    fn normalize_output_item(item: &mut OutputItem) {
        if item.output_item_type.is_none() && item.role.is_some() && item.content.is_some() {
            item.output_item_type =
                Some(crate::providers::openai::generated::OutputItemType::Message);
        }
        if item.output_item_type
            == Some(crate::providers::openai::generated::OutputItemType::Reasoning)
            && item.content.is_none()
        {
            item.content = Some(vec![]);
        }
    }

    pub fn run_single_test_case(full_case_name: &str) -> Result<(), String> {
        let cases = discover_openai_responses_test_cases(None)
            .map_err(|e| format!("Failed to discover test cases: {}", e))?;

        let case = cases
            .iter()
            .find(|c| c.name == full_case_name)
            .ok_or_else(|| format!("Test case '{}' not found", full_case_name))?;

        run_roundtrip_test_with_config(
            case,
            RoundtripTestConfig {
                // Extract messages from request (OpenAI Responses API has complex input structure)
                extract_messages: |request: &CreateResponseClass| match &request.input {
                    Some(Instructions::InputItemArray(msgs)) => Ok(msgs.clone()),
                    o => Err(format!(
                        "Invalid missing or non-array input messages: {:?}",
                        o
                    )),
                },
                // Convert to universal
                convert_to_universal: |messages: &Vec<InputItem>| {
                    <Vec<Message> as TryFromLLM<Vec<InputItem>>>::try_from(messages.clone())
                        .map_err(|e| format!("Failed to convert to universal format: {}", e))
                },
                // Convert from universal
                convert_from_universal: |messages: Vec<Message>| {
                    <Vec<InputItem> as TryFromLLM<Vec<Message>>>::try_from(messages)
                        .map_err(|e| format!("Failed to roundtrip conversion: {}", e))
                },
                // Extract response content (output messages from OpenAI Responses API)
                extract_response_content:
                    |response: &TheResponseObject| -> Result<Vec<OutputItem>, String> {
                        Ok(response.output.clone())
                    },
                // Convert response to universal (OutputItems to Messages)
                convert_response_to_universal: |output_items: &Vec<OutputItem>| {
                    <Vec<Message> as TryFromLLM<Vec<OutputItem>>>::try_from(output_items.clone())
                        .map_err(|e| {
                            format!("Failed to convert OutputItems to universal format: {}", e)
                        })
                },
                // Convert universal to response (Messages to OutputItems)
                convert_universal_to_response: |messages: Vec<Message>| {
                    <Vec<OutputItem> as TryFromLLM<Vec<Message>>>::try_from(messages).map_err(|e| {
                        format!("Failed to roundtrip conversion from universal: {}", e)
                    })
                },
                normalize_provider_message: normalize_responses_defaults,
                normalize_response_content: normalize_responses_defaults,
            },
        )
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }

    // The Responses `usage.input_tokens_details.cache_write_tokens` field is
    // marked `required` in the synchronized spec, but real captured responses
    // (every response emitted before the field shipped) omit it. The generated
    // type relaxes it to optional so those genuine provider payloads keep
    // deserializing. These guard that relaxation in both directions.
    #[test]
    fn response_usage_deserializes_without_cache_write_tokens() {
        use crate::providers::openai::generated::ResponseUsage;

        let payload = crate::serde_json::json!({
            "input_tokens": 13,
            "input_tokens_details": { "cached_tokens": 0 },
            "output_tokens": 8,
            "output_tokens_details": { "reasoning_tokens": 0 },
            "total_tokens": 21
        });

        let usage: ResponseUsage = crate::serde_json::from_value(payload)
            .expect("response usage without cache_write_tokens must deserialize");
        assert_eq!(usage.input_tokens_details.cache_write_tokens, None);

        // Absent input stays absent on re-serialization (no fabricated field).
        let reserialized = crate::serde_json::to_value(&usage).unwrap();
        assert!(reserialized["input_tokens_details"]
            .get("cache_write_tokens")
            .is_none());
    }

    #[test]
    fn response_usage_preserves_cache_write_tokens_when_present() {
        use crate::providers::openai::generated::ResponseUsage;

        let payload = crate::serde_json::json!({
            "input_tokens": 13,
            "input_tokens_details": { "cached_tokens": 4, "cache_write_tokens": 9 },
            "output_tokens": 8,
            "output_tokens_details": { "reasoning_tokens": 0 },
            "total_tokens": 21
        });

        let usage: ResponseUsage = crate::serde_json::from_value(payload.clone())
            .expect("response usage with cache_write_tokens must deserialize");
        assert_eq!(usage.input_tokens_details.cache_write_tokens, Some(9));

        // Present value round-trips unchanged.
        let reserialized = crate::serde_json::to_value(&usage).unwrap();
        assert_eq!(
            reserialized["input_tokens_details"]["cache_write_tokens"],
            9
        );
    }
}
