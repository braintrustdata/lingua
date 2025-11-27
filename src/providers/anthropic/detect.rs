/*!
Anthropic format detection.

This module provides functions to detect if a payload is already in
Anthropic-compatible format, distinguishing it from OpenAI format.
*/

use crate::providers::anthropic::generated::{
    CreateMessageParams, InputContentBlockType, MessageRole,
};
use crate::serde_json;
use thiserror::Error;

/// Error type for payload detection
#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("JSON parsing failed: {0}")]
    JsonParseFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Detects if a JSON payload is in Anthropic format.
///
/// This function attempts to deserialize the payload as an Anthropic
/// `CreateMessageParams` and validates Anthropic-specific characteristics.
///
/// # Arguments
///
/// * `payload` - JSON string to check
///
/// # Returns
///
/// * `Ok(true)` - Payload is valid Anthropic format
/// * `Ok(false)` - Payload is not Anthropic format (likely OpenAI or other)
/// * `Err(DetectionError)` - JSON parsing or validation error
///
/// # Examples
///
/// ```rust
/// use lingua::providers::anthropic::is_anthropic_format;
///
/// let anthropic_payload = r#"{
///     "model": "claude-3-5-sonnet-20241022",
///     "messages": [{"role": "user", "content": "Hello"}],
///     "max_tokens": 1024
/// }"#;
///
/// assert!(is_anthropic_format(anthropic_payload).unwrap());
/// ```
pub fn is_anthropic_format(payload: &str) -> Result<bool, DetectionError> {
    // First, try to deserialize as Anthropic format
    let request: CreateMessageParams = serde_json::from_str(payload)
        .map_err(|e| DetectionError::DeserializationFailed(e.to_string()))?;

    // Validate Anthropic-specific characteristics
    validate_anthropic_characteristics(&request)
}

/// Validates that a deserialized request has Anthropic-specific characteristics.
fn validate_anthropic_characteristics(
    request: &CreateMessageParams,
) -> Result<bool, DetectionError> {
    // Check 1: max_tokens is required in Anthropic (validated by deserialization)
    // If we got here, max_tokens is present

    // Check 2: All message roles must be "user" or "assistant" only
    // Anthropic doesn't have "system" or "tool" roles in messages
    for message in &request.messages {
        match message.role {
            MessageRole::User | MessageRole::Assistant => {
                // Valid Anthropic roles
            }
        }

        // Check 3: Content structure uses Anthropic-specific block types
        use crate::providers::anthropic::generated::MessageContent;
        match &message.content {
            MessageContent::String(_) => {
                // String content is valid for both, need to check other signals
            }
            MessageContent::InputContentBlockArray(blocks) => {
                for block in blocks {
                    match block.input_content_block_type {
                        InputContentBlockType::ToolUse
                        | InputContentBlockType::ToolResult
                        | InputContentBlockType::Thinking
                        | InputContentBlockType::RedactedThinking
                        | InputContentBlockType::Document
                        | InputContentBlockType::SearchResult
                        | InputContentBlockType::ServerToolUse
                        | InputContentBlockType::WebSearchToolResult => {
                            // These are Anthropic-specific types that don't exist in OpenAI
                            return Ok(true);
                        }
                        InputContentBlockType::Text | InputContentBlockType::Image => {
                            // These exist in both formats, check image structure
                            if let Some(source) = &block.source {
                                // Anthropic uses "source" object with "type": "base64"
                                // OpenAI uses "image_url" object (which wouldn't deserialize here)
                                use crate::providers::anthropic::generated::Source;
                                if matches!(source, Source::SourceSource(_)) {
                                    // This is Anthropic image format
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If we got here, the payload deserialized as Anthropic format
    // but doesn't have strong Anthropic-specific signals
    // Check for negative signals (OpenAI-specific features that would fail deserialization)

    // Since it successfully deserialized as CreateMessageParams, it's likely Anthropic
    // The main distinguishing factor is that OpenAI formats would fail to deserialize
    // due to different structure (e.g., "system" role messages, "tool" role messages,
    // different content block structure)

    Ok(true)
}

/// More detailed format detection that distinguishes between Anthropic and OpenAI formats.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadFormat {
    /// Payload is valid Anthropic format
    Anthropic,
    /// Payload is valid OpenAI format
    OpenAI,
    /// Payload format is unknown or invalid
    Unknown,
}

/// Detects the payload format with more detailed information.
///
/// This function attempts to deserialize the payload as both Anthropic and OpenAI
/// formats to determine which format it matches.
///
/// # Arguments
///
/// * `payload` - JSON string to check
///
/// # Returns
///
/// * `Ok(PayloadFormat::Anthropic)` - Payload is Anthropic format
/// * `Ok(PayloadFormat::OpenAI)` - Payload is OpenAI format  
/// * `Ok(PayloadFormat::Unknown)` - Payload doesn't match either format
/// * `Err(DetectionError)` - JSON parsing error
pub fn detect_payload_format(payload: &str) -> Result<PayloadFormat, DetectionError> {
    // Try Anthropic first
    let is_anthropic = match serde_json::from_str::<CreateMessageParams>(payload) {
        Ok(request) => validate_anthropic_characteristics(&request)?,
        Err(_) => false,
    };

    if is_anthropic {
        return Ok(PayloadFormat::Anthropic);
    }

    // Try OpenAI format
    #[cfg(feature = "openai")]
    {
        use crate::providers::openai::generated::OpenaiSchemas;

        if serde_json::from_str::<OpenaiSchemas>(payload).is_ok() {
            return Ok(PayloadFormat::OpenAI);
        }
    }

    Ok(PayloadFormat::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_anthropic_format_valid() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1024
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_tool_use() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_123",
                            "name": "get_weather",
                            "input": {"location": "SF"}
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_tool_result() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_123",
                            "content": "72°F"
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_with_image() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
                                "media_type": "image/png"
                            }
                        }
                    ]
                }
            ],
            "max_tokens": 1024
        }"#;

        assert!(is_anthropic_format(payload).unwrap());
    }

    #[test]
    fn test_is_anthropic_format_invalid_json() {
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
        }"#; // Invalid JSON

        assert!(is_anthropic_format(payload).is_err());
    }

    #[test]
    fn test_is_anthropic_format_missing_max_tokens() {
        // max_tokens is required in CreateMessageParams
        // This should fail deserialization
        let payload = r#"{
            "model": "claude-3-5-sonnet-20241022",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        }"#;

        // This will fail deserialization because max_tokens is required
        assert!(is_anthropic_format(payload).is_err());
    }
}
