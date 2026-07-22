/*!
OpenAI format validation.
*/

use crate::providers::openai::generated::{
    CreateChatCompletionRequestClass, CreateChatCompletionResponse,
    CreateChatCompletionStreamResponse, CreateResponseClass, TheResponseObject,
};
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as a chat completions request
pub fn validate_chat_completions_request(
    json: &str,
) -> Result<CreateChatCompletionRequestClass, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a chat completions response
pub fn validate_chat_completions_response(
    json: &str,
) -> Result<CreateChatCompletionResponse, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a Responses API request
pub fn validate_responses_request(json: &str) -> Result<CreateResponseClass, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as a Responses API response
pub fn validate_responses_response(json: &str) -> Result<TheResponseObject, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as an OpenAI chat completion request
/// @deprecated Use validate_chat_completions_request instead
pub fn validate_openai_request(
    json: &str,
) -> Result<CreateChatCompletionRequestClass, ValidationError> {
    validate_chat_completions_request(json)
}

/// Validates a JSON string as an OpenAI chat completion response
/// @deprecated Use validate_chat_completions_response instead
pub fn validate_openai_response(
    json: &str,
) -> Result<CreateChatCompletionResponse, ValidationError> {
    validate_chat_completions_response(json)
}

/// Validates a JSON string as a Chat Completions stream chunk
pub fn validate_chat_completions_stream_chunk(
    json: &str,
) -> Result<CreateChatCompletionStreamResponse, ValidationError> {
    validate_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_openai_request_minimal() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        }"#;

        let result = validate_openai_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_openai_request_invalid() {
        let json = r#"{
            "model": "gpt-4"
        }"#; // missing messages

        let result = validate_openai_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_nullable_safety_identifier_and_prompt_cache_key() {
        // The spec changed `safety_identifier` and `prompt_cache_key` from
        // `type: string` to `anyOf: [string, null]`. Explicit nulls must now be
        // accepted (previously-valid string values must still be accepted too).
        let with_nulls = r#"{
            "model": "gpt-4",
            "messages": [{ "role": "user", "content": "Hello" }],
            "safety_identifier": null,
            "prompt_cache_key": null
        }"#;
        let parsed = validate_chat_completions_request(with_nulls)
            .expect("null safety_identifier/prompt_cache_key must validate");
        assert_eq!(parsed.safety_identifier, None);
        assert_eq!(parsed.prompt_cache_key, None);

        let with_strings = r#"{
            "model": "gpt-4",
            "messages": [{ "role": "user", "content": "Hello" }],
            "safety_identifier": "safety-identifier-1234",
            "prompt_cache_key": "prompt-cache-key-1234"
        }"#;
        let parsed = validate_chat_completions_request(with_strings)
            .expect("string safety_identifier/prompt_cache_key must still validate");
        assert_eq!(
            parsed.safety_identifier.as_deref(),
            Some("safety-identifier-1234")
        );
        assert_eq!(
            parsed.prompt_cache_key.as_deref(),
            Some("prompt-cache-key-1234")
        );
    }

    #[test]
    fn test_response_error_code_data_residency_mismatch_roundtrips() {
        use crate::providers::openai::generated::{ResponseError, ResponseErrorCode};

        // `data_residency_mismatch` was added to the ResponseErrorCode enum in the
        // synchronized spec. It must deserialize into the typed variant and
        // re-serialize back to the same wire value (round trip, not one-way).
        let json = r#"{ "code": "data_residency_mismatch", "message": "region mismatch" }"#;
        let parsed: ResponseError =
            serde_json::from_str(json).expect("data_residency_mismatch must deserialize");
        assert_eq!(parsed.code, ResponseErrorCode::DataResidencyMismatch);

        let reserialized = serde_json::to_value(&parsed).expect("must serialize");
        assert_eq!(reserialized["code"], "data_residency_mismatch");

        // A previously-valid error code must still deserialize (no regression).
        let existing = r#"{ "code": "bio_policy", "message": "blocked" }"#;
        let parsed: ResponseError =
            serde_json::from_str(existing).expect("existing error codes must still validate");
        assert_eq!(parsed.code, ResponseErrorCode::BioPolicy);
    }

    #[test]
    fn test_validate_openai_response_minimal() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello!"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 12,
                "total_tokens": 21
            }
        }"#;

        let result = validate_openai_response(json);
        assert!(result.is_ok());
    }
}
