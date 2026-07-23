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

    fn normalize_responses_request_defaults(value: Value) -> Value {
        if let Ok(mut items) = crate::serde_json::from_value::<Vec<InputItem>>(value.clone()) {
            for item in &mut items {
                normalize_input_item(item);
            }
            return match crate::serde_json::to_value(items) {
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

    fn normalize_responses_response_defaults(value: Value) -> Value {
        if let Ok(mut items) = crate::serde_json::from_value::<Vec<OutputItem>>(value.clone()) {
            for item in &mut items {
                normalize_output_item(item);
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

        value
    }

    fn normalize_input_item(item: &mut InputItem) {
        // Responses passthrough preserves the raw request, but the generic universal
        // message model has no equivalent for this provider-only message field.
        item.phase = None;
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
                normalize_provider_message: normalize_responses_request_defaults,
                normalize_response_content: normalize_responses_response_defaults,
            },
        )
    }

    // Regression: the synchronized spec marks `cache_write_tokens` as required in the
    // Responses API `input_tokens_details`, but real OpenAI responses omit it when no
    // input tokens were written to the cache. The generated type must tolerate its
    // absence (defaulting to 0) so genuine provider payloads keep deserializing, and
    // must still emit it so outbound payloads satisfy the spec's required constraint.
    #[test]
    fn response_usage_tolerates_missing_cache_write_tokens() {
        use crate::providers::openai::generated::ResponseUsage;

        // A real captured OpenAI Responses payload: input_tokens_details carries only
        // `cached_tokens`, with no `cache_write_tokens` field.
        let real_payload = crate::serde_json::json!({
            "input_tokens": 13,
            "input_tokens_details": { "cached_tokens": 0 },
            "output_tokens": 8,
            "output_tokens_details": { "reasoning_tokens": 0 },
            "total_tokens": 21
        });
        let usage: ResponseUsage = crate::serde_json::from_value(real_payload)
            .expect("Responses usage without cache_write_tokens must still deserialize");
        assert_eq!(usage.input_tokens_details.cache_write_tokens, 0);
        assert_eq!(usage.input_tokens_details.cached_tokens, 0);

        // When present, the value is preserved.
        let with_field = crate::serde_json::json!({
            "input_tokens": 100,
            "input_tokens_details": { "cached_tokens": 40, "cache_write_tokens": 25 },
            "output_tokens": 8,
            "output_tokens_details": { "reasoning_tokens": 0 },
            "total_tokens": 108
        });
        let usage: ResponseUsage = crate::serde_json::from_value(with_field)
            .expect("Responses usage with cache_write_tokens must deserialize");
        assert_eq!(usage.input_tokens_details.cache_write_tokens, 25);

        // Serialization always emits the field, satisfying the spec's required constraint.
        let serialized = crate::serde_json::to_value(&usage).unwrap();
        assert_eq!(
            serialized["input_tokens_details"]["cache_write_tokens"],
            crate::serde_json::json!(25)
        );
    }

    // Regression: removing the stale duplicate `programmatic_tool_calling` variant
    // from the generated `Tool` enum must not stop the (still spec-valid) tool from
    // deserializing or re-serializing.
    #[test]
    fn tool_enum_accepts_programmatic_tool_calling() {
        use crate::providers::openai::generated::Tool;

        let value = crate::serde_json::json!({ "type": "programmatic_tool_calling" });
        let tool: Tool = crate::serde_json::from_value(value)
            .expect("programmatic_tool_calling tool must still deserialize");
        assert!(matches!(tool, Tool::ProgrammaticCallingParam(_)));

        let serialized = crate::serde_json::to_value(&tool).unwrap();
        assert_eq!(serialized["type"], "programmatic_tool_calling");
    }

    // Regression: the synchronized spec added `data_residency_mismatch` to the
    // Responses API `ResponseError.code` enum. The new variant must deserialize and
    // round-trip, and pre-existing variants (a compatibility contract) must remain
    // accepted rather than being rejected by the regenerated enum.
    #[test]
    fn response_error_code_accepts_data_residency_mismatch() {
        use crate::providers::openai::generated::{ResponseError, ResponseErrorCode};

        // The newly added enum value must parse into its dedicated variant.
        let payload = crate::serde_json::json!({
            "code": "data_residency_mismatch",
            "message": "Request could not be served from the required data region."
        });
        let error: ResponseError = crate::serde_json::from_value(payload)
            .expect("data_residency_mismatch error code must deserialize");
        assert_eq!(error.code, ResponseErrorCode::DataResidencyMismatch);

        // And it must serialize back to the exact wire value.
        let serialized = crate::serde_json::to_value(&error).unwrap();
        assert_eq!(serialized["code"], "data_residency_mismatch");

        // Pre-existing variants stay valid (no positive value was dropped).
        for wire in [
            "server_error",
            "rate_limit_exceeded",
            "invalid_prompt",
            "bio_policy",
        ] {
            let value = crate::serde_json::json!({ "code": wire, "message": "x" });
            let error: ResponseError = crate::serde_json::from_value(value)
                .unwrap_or_else(|e| panic!("error code {wire} must still deserialize: {e}"));
            assert_eq!(crate::serde_json::to_value(&error).unwrap()["code"], wire);
        }
    }

    // Include auto-generated test cases from build script
    #[allow(non_snake_case)]
    mod generated {
        include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
    }
}
