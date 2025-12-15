/*!
Bedrock Converse format detection.

This module provides functions to detect if a payload is in
AWS Bedrock Converse API format.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::FormatDetector;
use crate::serde_json::Value;

/// Detector for AWS Bedrock Converse API format.
///
/// Bedrock Converse has very distinctive features:
/// - `modelId` field instead of `model`
/// - `inferenceConfig` instead of generation config
/// - camelCase content types (`toolUse`, `toolResult`)
#[derive(Debug, Clone, Copy)]
pub struct ConverseDetector;

impl FormatDetector for ConverseDetector {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::Converse
    }

    fn detect(&self, payload: &Value) -> bool {
        is_bedrock_converse(payload)
    }

    fn priority(&self) -> u8 {
        95 // Highest priority - very distinctive format
    }
}

/// Check if payload is in Bedrock Converse format.
///
/// Indicators:
/// - Has `modelId` field instead of `model`
/// - Uses camelCase content types (`toolUse`, `toolResult`)
/// - Has `inferenceConfig` instead of `generation_config`
///
/// # Examples
///
/// ```rust
/// use lingua::serde_json::json;
/// use lingua::providers::bedrock::detect::is_bedrock_converse;
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
/// assert!(is_bedrock_converse(&bedrock_payload));
/// ```
pub fn is_bedrock_converse(payload: &Value) -> bool {
    // Primary indicator: modelId field
    if payload.get("modelId").is_some() {
        return true;
    }

    // Secondary indicator: inferenceConfig
    if payload.get("inferenceConfig").is_some() {
        return true;
    }

    // Check for Converse-style message structure
    if let Some(messages) = payload.get("messages").and_then(|v| v.as_array()) {
        for msg in messages {
            if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                for block in content {
                    // Converse uses camelCase: "toolUse", "toolResult"
                    if block.get("toolUse").is_some() || block.get("toolResult").is_some() {
                        return true;
                    }
                }
            }
        }
    }

    false
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
    fn test_bedrock_converse_with_model_id() {
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
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_inference_config() {
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
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_tool_use() {
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
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_tool_result() {
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
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_not_bedrock_openai_format() {
        // Raw JSON for non-Bedrock formats to ensure detection rejects them
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_bedrock_converse(&payload));
    }

    #[test]
    fn test_not_bedrock_anthropic_format() {
        // Raw JSON for non-Bedrock formats to ensure detection rejects them
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_bedrock_converse(&payload));
    }
}
