/*!
Anthropic format validation.
*/

use crate::providers::anthropic::generated::Message;
use crate::providers::anthropic::params::AnthropicParams;
use crate::validation::{validate_json, ValidationError};

const OPENAI_ONLY_FIELDS: &[&str] = &[
    "frequency_penalty",
    "logit_bias",
    "logprobs",
    "max_completion_tokens",
    "n",
    "parallel_tool_calls",
    "presence_penalty",
    "reasoning_effort",
    "response_format",
    "seed",
    "stop",
    "store",
    "stream_options",
    "top_logprobs",
];

/// Validates a JSON string as an Anthropic messages request
pub fn validate_anthropic_request(json: &str) -> Result<AnthropicParams, ValidationError> {
    let request: AnthropicParams = validate_json(json)?;
    if request.model.is_none() {
        return Err(ValidationError::DeserializationFailed(
            "missing field `model`".to_string(),
        ));
    }
    if request.messages.is_none() {
        return Err(ValidationError::DeserializationFailed(
            "missing field `messages`".to_string(),
        ));
    }
    for field in OPENAI_ONLY_FIELDS {
        if request.extras.contains_key(*field) {
            return Err(ValidationError::DeserializationFailed(format!(
                "OpenAI-only field `{}` is not valid Anthropic request syntax",
                field
            )));
        }
    }
    Ok(request)
}

/// Validates a JSON string as an Anthropic messages response
pub fn validate_anthropic_response(json: &str) -> Result<Message, ValidationError> {
    validate_json(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_anthropic_request_minimal() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_anthropic_request_invalid() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022"
        }"#; // missing messages and max_tokens

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_request_rejects_openai_only_fields() {
        let json = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "frequency_penalty": 0.5
        }"#;

        let result = validate_anthropic_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_anthropic_response_minimal() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let result = validate_anthropic_response(json);
        assert!(result.is_ok());
    }
}
