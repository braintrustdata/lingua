/*!
Bedrock Converse format detection.

This module provides functions to detect if a payload is in
AWS Bedrock Converse API format by attempting to deserialize
into the Bedrock struct types.
*/

use crate::providers::bedrock::request::ConverseRequest;
use crate::serde_json::{self, Value};
use thiserror::Error;

/// Error type for Bedrock payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Attempt to parse a JSON Value as Bedrock ConverseRequest.
///
/// Returns the parsed struct if successful, or an error if the payload
/// is not valid Bedrock Converse format.
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::bedrock::detect::try_parse_bedrock;
///
/// let bedrock_payload = json!({
///     "modelId": "anthropic.claude-3-sonnet-20240229-v1:0",
///     "messages": [{
///         "role": "user",
///         "content": [{"text": "Hello"}]
///     }],
///     "inferenceConfig": {
///         "maxTokens": 1024
///     }
/// });
///
/// assert!(try_parse_bedrock(&bedrock_payload).is_ok());
/// ```
pub fn try_parse_bedrock(payload: &Value) -> Result<ConverseRequest, DetectionError> {
    serde_json::from_value(payload.clone())
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::bedrock::request::{
        BedrockContentBlock, BedrockConversationRole, BedrockInferenceConfiguration,
        BedrockMessage, BedrockToolResultBlock, BedrockToolResultContent, BedrockToolUseBlock,
        ConverseRequest,
    };
    use crate::serde_json::{self, json};

    #[test]
    fn test_try_parse_bedrock_with_model_id() {
        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::User,
                content: vec![BedrockContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_bedrock(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_bedrock_with_inference_config() {
        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::User,
                content: vec![BedrockContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: None,
            inference_config: Some(BedrockInferenceConfiguration {
                max_tokens: Some(1024),
                temperature: None,
                top_p: None,
                stop_sequences: None,
            }),
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_bedrock(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_bedrock_with_tool_use() {
        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::Assistant,
                content: vec![BedrockContentBlock::ToolUse {
                    tool_use: BedrockToolUseBlock {
                        tool_use_id: "tool_123".to_string(),
                        name: "get_weather".to_string(),
                        input: json!({"location": "SF"}),
                    },
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_bedrock(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_bedrock_with_tool_result() {
        let request = ConverseRequest {
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::User,
                content: vec![BedrockContentBlock::ToolResult {
                    tool_result: BedrockToolResultBlock {
                        tool_use_id: "tool_123".to_string(),
                        content: vec![BedrockToolResultContent::Text {
                            text: "72°F".to_string(),
                        }],
                        status: None,
                    },
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let payload = serde_json::to_value(&request).unwrap();
        assert!(try_parse_bedrock(&payload).is_ok());
    }

    #[test]
    fn test_try_parse_bedrock_fails_for_openai_format() {
        // OpenAI format uses "model" not "modelId" - should fail struct deserialization
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(try_parse_bedrock(&payload).is_err());
    }

    #[test]
    fn test_try_parse_bedrock_fails_for_anthropic_format() {
        // Anthropic format uses "model" not "modelId" - should fail struct deserialization
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(try_parse_bedrock(&payload).is_err());
    }

    #[test]
    fn test_try_parse_bedrock_success() {
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet-20240229-v1:0",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });

        let result = try_parse_bedrock(&payload);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.model_id, "anthropic.claude-3-sonnet-20240229-v1:0");
    }

    #[test]
    fn test_try_parse_bedrock_fails_without_model_id() {
        // Missing modelId - required field
        let payload = json!({
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });

        let result = try_parse_bedrock(&payload);
        assert!(result.is_err());
    }

}
