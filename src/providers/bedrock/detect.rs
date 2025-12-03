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
    use crate::serde_json::json;

    #[test]
    fn test_bedrock_converse_with_model_id() {
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet-20240229-v1:0",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_inference_config() {
        let payload = json!({
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }],
            "inferenceConfig": {
                "maxTokens": 1024
            }
        });
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_tool_use() {
        let payload = json!({
            "messages": [{
                "role": "assistant",
                "content": [{
                    "toolUse": {
                        "toolUseId": "tool_123",
                        "name": "get_weather",
                        "input": {"location": "SF"}
                    }
                }]
            }]
        });
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_bedrock_converse_with_tool_result() {
        let payload = json!({
            "messages": [{
                "role": "user",
                "content": [{
                    "toolResult": {
                        "toolUseId": "tool_123",
                        "content": [{"text": "72°F"}]
                    }
                }]
            }]
        });
        assert!(is_bedrock_converse(&payload));
    }

    #[test]
    fn test_not_bedrock_openai_format() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_bedrock_converse(&payload));
    }

    #[test]
    fn test_not_bedrock_anthropic_format() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!is_bedrock_converse(&payload));
    }

    #[test]
    fn test_detector_trait() {
        let detector = ConverseDetector;
        assert_eq!(detector.format(), ProviderFormat::Converse);
        assert_eq!(detector.priority(), 95);
    }
}
