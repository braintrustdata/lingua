/*!
OpenAI format validation.
*/

use crate::providers::openai::generated::{
    CreateChatCompletionRequestClass, CreateChatCompletionResponse,
};
use crate::validation::{validate_json, ValidationError};

/// Validates a JSON string as an OpenAI chat completion request
pub fn validate_openai_request(
    json: &str,
) -> Result<CreateChatCompletionRequestClass, ValidationError> {
    validate_json(json)
}

/// Validates a JSON string as an OpenAI chat completion response
pub fn validate_openai_response(
    json: &str,
) -> Result<CreateChatCompletionResponse, ValidationError> {
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
