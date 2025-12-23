/*!
Unified payload transformation API.

This module provides a single entry point for validating and transforming
payloads between different provider formats. The key principle is:

**If a payload can be deserialized into the target provider struct, use it as-is (pass-through).**
**Otherwise, detect the source format, convert to universal, and transform to target format.**

This replaces heuristic-based detection with struct-based validation.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::adapters;
use crate::serde_json::Value;
use thiserror::Error;

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

    #[error("Streaming not implemented: {0}")]
    StreamingNotImplemented(String),
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

    // Step 2: Detect source adapter
    let source_adapter = adapters()
        .into_iter()
        .find(|a| a.detect_request(payload))
        .ok_or(TransformError::UnableToDetectFormat)?;

    let source_format = source_adapter.format();

    // Step 3: Get target adapter
    let target_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    // Step 4: Convert: source -> universal -> target
    let universal = source_adapter.request_to_universal(payload)?;
    let transformed = target_adapter.request_from_universal(&universal)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

/// Transform a full request payload to the target format with provider defaults applied.
///
/// Unlike `validate_or_transform` which only transforms messages, this function
/// handles the complete request including model parameters. It applies provider-specific
/// defaults (e.g., `max_tokens` for Anthropic) only when transformation is needed.
///
/// Returns `TransformResult::PassThrough` when the payload is already valid for the
/// target format - in this case, use the original payload as-is with zero overhead.
///
/// # Arguments
///
/// * `payload` - The incoming JSON payload (full request)
/// * `target_format` - The target provider format
/// * `model` - Optional model name to inject if source doesn't have one (for Google/Bedrock)
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Payload is already valid, use original as-is
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Transformed with defaults
/// * `Err(TransformError)` - If transformation fails
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::{transform_request, TransformResult};
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // OpenAI request without max_tokens
/// let openai_payload = json!({
///     "model": "gpt-4",
///     "messages": [{"role": "user", "content": "Hello"}]
/// });
///
/// // Transform to Anthropic - max_tokens will be added with default value
/// let result = transform_request(&openai_payload, ProviderFormat::Anthropic, None).unwrap();
/// let final_payload = result.payload_or_original(openai_payload);
/// ```
pub fn transform_request(
    payload: &Value,
    target_format: ProviderFormat,
    model: Option<&str>,
) -> Result<TransformResult, TransformError> {
    // Step 1: Check if payload is already valid for target format (passthrough)
    if is_valid_for_format(payload, target_format) {
        return Ok(TransformResult::PassThrough);
    }

    // Step 2: Detect source format
    let source_format = detect_source_format(payload)?;

    // Step 3: Get adapters for source and target
    let source_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == source_format)
        .ok_or(TransformError::UnsupportedSourceFormat(source_format))?;
    let target_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    // Step 4: Convert source payload to UniversalRequest
    let mut universal = source_adapter.request_to_universal(payload)?;

    // Step 5: Inject model from parameter if not present
    if model.is_some() && universal.model.is_none() {
        universal.model = model.map(String::from);
    }

    // Step 6: Apply target provider defaults
    target_adapter.apply_defaults(&mut universal);

    // Step 7: Convert UniversalRequest to target format
    let transformed = target_adapter.request_from_universal(&universal)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

/// Check if a payload is valid for a specific format by attempting deserialization.
pub fn is_valid_for_format(payload: &Value, format: ProviderFormat) -> bool {
    adapters()
        .into_iter()
        .any(|a| a.format() == format && a.detect_request(payload))
}

/// Detect the source format by trying each adapter in priority order.
///
/// Priority is determined by adapter registration order in `adapters()`.
fn detect_source_format(payload: &Value) -> Result<ProviderFormat, TransformError> {
    adapters()
        .into_iter()
        .find(|a| a.detect_request(payload))
        .map(|a| a.format())
        .ok_or(TransformError::UnableToDetectFormat)
}

// ============================================================================
// Response transformation
// ============================================================================

/// Transform a response payload from one format to another.
///
/// This extracts the message(s) from the source response envelope, converts
/// them via the universal Message format, and builds a new response envelope
/// in the target format.
///
/// # Arguments
///
/// * `response` - The source response JSON payload
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Response is already valid for target format
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Transformed response
/// * `Err(TransformError)` - If transformation fails
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::transform_response;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // Google response
/// let google_response = json!({
///     "candidates": [{
///         "content": {
///             "role": "model",
///             "parts": [{"text": "Hello!"}]
///         }
///     }]
/// });
///
/// // Transform to OpenAI format
/// let result = transform_response(&google_response, ProviderFormat::OpenAI).unwrap();
/// ```
pub fn transform_response(
    response: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Detect source format using adapters
    let source_adapter = adapters()
        .into_iter()
        .find(|a| a.detect_response(response))
        .ok_or(TransformError::UnableToDetectFormat)?;

    let source_format = source_adapter.format();

    // Step 2: PassThrough if source matches target (no transformation needed)
    // This is critical for preserving provider-specific fields and avoiding overhead
    if source_format == target_format {
        return Ok(TransformResult::PassThrough);
    }

    // Step 3: Get target adapter
    let target_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    // Step 4: Convert: source -> universal -> target
    let universal = source_adapter.response_to_universal(response)?;
    let transformed = target_adapter.response_from_universal(&universal)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
    })
}

// ============================================================================
// Streaming transformation
// ============================================================================

/// Transform a streaming chunk from one format to another.
///
/// This handles per-event transformation for streaming responses. Each event
/// is processed independently (stateless).
///
/// # Arguments
///
/// * `chunk` - The source streaming chunk JSON payload
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough)` - Chunk is already valid for target format
/// * `Ok(TransformResult::Transformed { payload, source_format })` - Transformed chunk
/// * `Err(TransformError)` - If transformation fails
pub fn transform_stream_chunk(
    chunk: &Value,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    // Step 1: Detect source format using streaming detection
    let source_adapter = adapters()
        .into_iter()
        .find(|a| a.detect_stream_response(chunk))
        .ok_or(TransformError::UnableToDetectFormat)?;

    let source_format = source_adapter.format();

    // Step 2: PassThrough if source matches target
    if source_format == target_format {
        return Ok(TransformResult::PassThrough);
    }

    // Step 3: Get target adapter
    let target_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    // Step 4: Convert: source -> universal -> target
    let universal = source_adapter.stream_to_universal(chunk)?;

    match universal {
        Some(universal_chunk) => {
            // Transform to target format
            let transformed = target_adapter.stream_from_universal(&universal_chunk)?;
            Ok(TransformResult::Transformed {
                payload: transformed,
                source_format,
            })
        }
        None => {
            // Source returned None (terminal event) - return empty object
            Ok(TransformResult::Transformed {
                payload: crate::serde_json::json!({}),
                source_format,
            })
        }
    }
}

/// Transform an array of streaming chunks from one format to another.
///
/// This is useful for testing with snapshot files which contain arrays of events.
///
/// # Arguments
///
/// * `chunks` - The source streaming chunks as a JSON array value
/// * `target_format` - The target provider format
///
/// # Returns
///
/// A vector of transform results, one per chunk.
pub fn transform_stream_array(
    chunks: &Value,
    target_format: ProviderFormat,
) -> Result<Vec<TransformResult>, TransformError> {
    let array = chunks
        .as_array()
        .ok_or_else(|| TransformError::ToUniversalFailed("expected array".to_string()))?;

    array
        .iter()
        .map(|chunk| transform_stream_chunk(chunk, target_format))
        .collect()
}

/// Detect the source format of a streaming chunk.
pub fn detect_stream_source_format(chunk: &Value) -> Result<ProviderFormat, TransformError> {
    adapters()
        .into_iter()
        .find(|a| a.detect_stream_response(chunk))
        .map(|a| a.format())
        .ok_or(TransformError::UnableToDetectFormat)
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

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_adds_anthropic_max_tokens() {
        // OpenAI request without max_tokens
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic, None).unwrap();

        // Should be transformed (not pass-through) since source is OpenAI
        assert!(!result.is_pass_through());

        let final_payload = result.payload_or_original(payload);

        // Should have max_tokens added with default value (4096)
        assert_eq!(
            final_payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(4096)
        );
        // Should have messages transformed
        assert!(final_payload.get("messages").is_some());
        // Should have model preserved
        assert_eq!(
            final_payload.get("model").and_then(|v| v.as_str()),
            Some("gpt-4")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_preserves_existing_max_tokens() {
        // OpenAI request with max_tokens
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 8192
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic, None).unwrap();
        let final_payload = result.payload_or_original(payload);

        // Should preserve the existing max_tokens value
        assert_eq!(
            final_payload.get("max_tokens").and_then(|v| v.as_i64()),
            Some(8192)
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_transform_request_passthrough_returns_passthrough() {
        // Valid Anthropic request - should pass through with zero overhead
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic, None).unwrap();

        // Should be pass-through since payload is already valid Anthropic format
        assert!(result.is_pass_through());

        // Using payload_or_original returns the original payload as-is
        let final_payload = result.payload_or_original(payload.clone());
        assert_eq!(final_payload, payload);
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_copies_common_params() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7,
            "top_p": 0.9,
            "stream": true
        });

        let result = transform_request(&payload, ProviderFormat::Anthropic, None).unwrap();
        let final_payload = result.payload_or_original(payload);

        assert_eq!(
            final_payload.get("temperature").and_then(|v| v.as_f64()),
            Some(0.7)
        );
        assert_eq!(
            final_payload.get("top_p").and_then(|v| v.as_f64()),
            Some(0.9)
        );
        assert_eq!(
            final_payload.get("stream").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
