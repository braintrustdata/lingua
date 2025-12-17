/*!
Unified payload transformation API.

This module provides a single entry point for validating and transforming
payloads between different provider formats. The key principle is:

**If a payload can be deserialized into the target provider struct, use it as-is (pass-through).**
**Otherwise, detect the source format, convert to universal, and transform to target format.**

This replaces heuristic-based detection with struct-based validation.
*/

use crate::capabilities::ProviderFormat;
use crate::serde_json::{self, Value};
use crate::universal::message::Message;
use thiserror::Error;

#[cfg(feature = "anthropic")]
use crate::providers::anthropic::try_parse_anthropic;
#[cfg(feature = "bedrock")]
use crate::providers::bedrock::try_parse_bedrock;
#[cfg(feature = "google")]
use crate::providers::google::try_parse_google;
#[cfg(feature = "mistral")]
use crate::providers::mistral::MistralDetector;
#[cfg(feature = "openai")]
use crate::providers::openai::try_parse_openai;

#[cfg(feature = "mistral")]
use crate::processing::FormatDetector;

/// Error type for transformation operations
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Unable to detect source format")]
    UnableToDetectFormat,

    #[error("Validation failed for target format {target:?}: {reason}")]
    ValidationFailed {
        target: ProviderFormat,
        reason: String,
    },

    #[error("Conversion to universal format failed: {0}")]
    ToUniversalFailed(String),

    #[error("Conversion from universal format failed: {0}")]
    FromUniversalFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Unsupported target format: {0:?}")]
    UnsupportedTargetFormat(ProviderFormat),

    #[error("Unsupported source format: {0:?}")]
    UnsupportedSourceFormat(ProviderFormat),
}

/// Result of a transformation operation
#[derive(Debug, Clone)]
pub enum TransformResult {
    /// Payload was already valid for target format - use original
    PassThrough,

    /// Payload was transformed to target format
    Transformed {
        /// The transformed payload
        payload: Value,
        /// The detected source format
        source_format: ProviderFormat,
    },
}

impl TransformResult {
    /// Check if this is a pass-through result
    pub fn is_pass_through(&self) -> bool {
        matches!(self, TransformResult::PassThrough)
    }

    /// Get the transformed payload, or return the original if pass-through
    pub fn payload_or_original(self, original: Value) -> Value {
        match self {
            TransformResult::PassThrough => original,
            TransformResult::Transformed { payload, .. } => payload,
        }
    }
}

/// Try to validate payload as target format, or transform it.
///
/// This is the main entry point for payload transformation. It:
/// 1. Tries to parse the payload as the target format (if it succeeds, return PassThrough)
/// 2. If parsing fails, detects the source format by trying each format in priority order
/// 3. Converts from source format to universal format
/// 4. Converts from universal format to target format
///
/// # Arguments
///
/// * `payload` - The incoming JSON payload
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Payload is already valid for target format
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Payload was transformed
/// * `Err(TransformError)` - Transformation failed
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::validate_or_transform;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// // If target is OpenAI and payload is OpenAI format, returns PassThrough
/// let result = validate_or_transform(&openai_payload, ProviderFormat::OpenAI);
/// ```
pub fn validate_or_transform(
    payload: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Try to parse as target format
    if is_valid_for_format(payload, target_format) {
        return Ok(TransformResult::PassThrough);
    }

    // Step 2: Detect source format by trying each in priority order
    let source_format = detect_source_format(payload)?;

    // Step 3: Convert to universal format
    let universal = to_universal(payload, source_format)?;

    // Step 4: Convert from universal to target format
    let transformed = from_universal(&universal, target_format)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

/// Check if a payload is valid for a specific format by attempting deserialization.
pub fn is_valid_for_format(payload: &Value, format: ProviderFormat) -> bool {
    match format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI => try_parse_openai(payload).is_ok(),

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => try_parse_anthropic(payload).is_ok(),

        #[cfg(feature = "google")]
        ProviderFormat::Google => try_parse_google(payload).is_ok(),

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => try_parse_bedrock(payload).is_ok(),

        #[cfg(feature = "mistral")]
        ProviderFormat::Mistral => {
            // Mistral needs both valid structure AND Mistral indicators
            MistralDetector.detect(payload)
        }

        ProviderFormat::Unknown => false,

        // When features are disabled, return false
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Detect the source format by trying to parse as each format in priority order.
///
/// Priority order (most specific first):
/// 1. Bedrock Converse (priority 95) - `modelId` field is unique
/// 2. Google (priority 90) - `contents[].parts[]` structure
/// 3. Anthropic (priority 80) - `max_tokens` required, specific roles
/// 4. Mistral (priority 70) - OpenAI-compatible with extras
/// 5. OpenAI (priority 50) - Most permissive, fallback
fn detect_source_format(payload: &Value) -> Result<ProviderFormat, TransformError> {
    // Try most specific formats first

    #[cfg(feature = "bedrock")]
    if try_parse_bedrock(payload).is_ok() {
        return Ok(ProviderFormat::Converse);
    }

    #[cfg(feature = "google")]
    if try_parse_google(payload).is_ok() {
        return Ok(ProviderFormat::Google);
    }

    #[cfg(feature = "anthropic")]
    if try_parse_anthropic(payload).is_ok() {
        return Ok(ProviderFormat::Anthropic);
    }

    #[cfg(feature = "mistral")]
    if MistralDetector.detect(payload) {
        return Ok(ProviderFormat::Mistral);
    }

    #[cfg(feature = "openai")]
    if try_parse_openai(payload).is_ok() {
        return Ok(ProviderFormat::OpenAI);
    }

    Err(TransformError::UnableToDetectFormat)
}

/// Convert a payload from its source format to universal message format.
fn to_universal(
    payload: &Value,
    source_format: ProviderFormat,
) -> Result<Vec<Message>, TransformError> {
    match source_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            // Parse as OpenAI and convert messages
            use crate::providers::openai::generated::CreateChatCompletionRequestClass;
            use crate::universal::convert::TryFromLLM;

            let request: CreateChatCompletionRequestClass = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::CreateMessageParams;
            use crate::universal::convert::TryFromLLM;

            let request: CreateMessageParams = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::{GoogleContent, GoogleGenerateContentRequest};
            use crate::universal::convert::TryFromLLM;

            let request: GoogleGenerateContentRequest = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<GoogleContent>>>::try_from(request.contents)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::request::{BedrockMessage, ConverseRequest};
            use crate::universal::convert::TryFromLLM;

            let request: ConverseRequest = serde_json::from_value(payload.clone())
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

            <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(request.messages)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))
        }

        _ => Err(TransformError::UnsupportedSourceFormat(source_format)),
    }
}

/// Convert universal messages to a specific target format.
///
/// This is the main entry point for converting lingua's universal Message format
/// to any supported provider format. The function dispatches to the appropriate
/// TryFromLLM implementation based on the target format.
///
/// # Arguments
///
/// * `messages` - Slice of universal Message objects to convert
/// * `target_format` - The target provider format (OpenAI, Anthropic, Google, etc.)
///
/// # Returns
///
/// * `Ok(Value)` - JSON value containing the converted messages in target format
/// * `Err(TransformError)` - If conversion or serialization fails
pub fn from_universal(
    messages: &[Message],
    target_format: ProviderFormat,
) -> Result<Value, TransformError> {
    match target_format {
        #[cfg(feature = "openai")]
        ProviderFormat::OpenAI | ProviderFormat::Mistral => {
            use crate::providers::openai::generated::ChatCompletionRequestMessage;
            use crate::universal::convert::TryFromLLM;

            let openai_messages: Vec<ChatCompletionRequestMessage> =
                <Vec<ChatCompletionRequestMessage> as TryFromLLM<Vec<Message>>>::try_from(
                    messages.to_vec(),
                )
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(openai_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        #[cfg(feature = "anthropic")]
        ProviderFormat::Anthropic => {
            use crate::providers::anthropic::generated::InputMessage;
            use crate::universal::convert::TryFromLLM;

            let anthropic_messages: Vec<InputMessage> =
                <Vec<InputMessage> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(anthropic_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        #[cfg(feature = "google")]
        ProviderFormat::Google => {
            use crate::providers::google::detect::GoogleContent;
            use crate::universal::convert::TryFromLLM;

            let google_contents: Vec<GoogleContent> =
                <Vec<GoogleContent> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(google_contents)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        #[cfg(feature = "bedrock")]
        ProviderFormat::Converse => {
            use crate::providers::bedrock::request::BedrockMessage;
            use crate::universal::convert::TryFromLLM;

            let bedrock_messages: Vec<BedrockMessage> =
                <Vec<BedrockMessage> as TryFromLLM<Vec<Message>>>::try_from(messages.to_vec())
                    .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

            serde_json::to_value(bedrock_messages)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }

        _ => Err(TransformError::UnsupportedTargetFormat(target_format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    #[cfg(feature = "openai")]
    fn test_validate_openai_passthrough() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::OpenAI).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_openai_to_anthropic() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Anthropic).unwrap();
        match result {
            TransformResult::Transformed {
                payload: _,
                source_format,
            } => {
                assert_eq!(source_format, ProviderFormat::OpenAI);
            }
            TransformResult::PassThrough => {
                panic!("Expected transformation, got pass-through");
            }
        }
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_validate_anthropic_passthrough() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Anthropic).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "google")]
    fn test_validate_google_passthrough() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Hello"}]
            }]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Google).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "bedrock")]
    fn test_validate_bedrock_passthrough() {
        let payload = json!({
            "modelId": "anthropic.claude-3-sonnet",
            "messages": [{
                "role": "user",
                "content": [{"text": "Hello"}]
            }]
        });

        let result = validate_or_transform(&payload, ProviderFormat::Converse).unwrap();
        assert!(result.is_pass_through());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_detect_source_format_openai() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let format = detect_source_format(&payload).unwrap();
        assert_eq!(format, ProviderFormat::OpenAI);
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_detect_source_format_anthropic() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let format = detect_source_format(&payload).unwrap();
        assert_eq!(format, ProviderFormat::Anthropic);
    }

    #[test]
    fn test_detect_source_format_fails_for_invalid() {
        let payload = json!({
            "invalid": "payload"
        });

        let result = detect_source_format(&payload);
        assert!(result.is_err());
    }
}
