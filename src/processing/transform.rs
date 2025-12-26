/*!
Unified payload transformation API.

This module provides a single entry point for validating and transforming
payloads between different provider formats. The key principle is:

**If a payload can be deserialized into the target provider struct, use it as-is (pass-through).**
**Otherwise, detect the source format, convert to universal, and transform to target format.**

This replaces heuristic-based detection with struct-based validation.
*/

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{adapter_for_format, adapters};
use crate::serde_json::Value;
use crate::universal::{UniversalRequest, UniversalResponse};
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
        /// The intermediate universal request representation (only populated for request transformations)
        universal: Option<Box<UniversalRequest>>,
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

    /// Get a reference to the transformed payload, or return the original if pass-through
    pub fn payload_or_original_ref<'a>(&'a self, original: &'a Value) -> &'a Value {
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
        universal: Some(Box::new(universal)),
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
    payload: &Value, // take own values.
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
        universal: Some(Box::new(universal)),
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
    let universal_resp = source_adapter.response_to_universal(response)?;
    let transformed = target_adapter.response_from_universal(&universal_resp)?;

    Ok(TransformResult::Transformed {
        payload: transformed,
        source_format,
        universal: None, // Response transformations don't populate universal request
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
    let stream_universal = source_adapter.stream_to_universal(chunk)?;

    match stream_universal {
        Some(universal_chunk) => {
            // Transform to target format
            let transformed = target_adapter.stream_from_universal(&universal_chunk)?;
            Ok(TransformResult::Transformed {
                payload: transformed,
                source_format,
                universal: None, // Stream transformations don't populate universal request
            })
        }
        None => {
            // Source returned None (terminal event) - return empty object
            Ok(TransformResult::Transformed {
                payload: crate::serde_json::json!({}),
                source_format,
                universal: None,
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

// ============================================================================
// Stream event parsing (for router integration)
// ============================================================================

use crate::universal::UniversalStreamChunk;

/// Result of parsing a streaming event.
///
/// Contains the transformed payload, metadata about the event, and the universal
/// representation for further processing.
#[derive(Debug, Clone)]
pub struct ParsedStreamEvent {
    /// The payload to forward (original if pass-through, transformed otherwise)
    pub payload: Value,
    /// The detected source format
    pub source_format: ProviderFormat,
    /// The target format requested
    pub target_format: ProviderFormat,
    /// The universal representation of the stream chunk (if transformation occurred)
    pub universal: Option<UniversalStreamChunk>,
    /// Whether this is a keep-alive event (no content, just maintains connection)
    pub is_keep_alive: bool,
    /// Whether this event contains a finish_reason (indicates end of generation)
    pub is_final: bool,
}

/// Parse a streaming event, transforming if needed and extracting metadata.
///
/// This is the main entry point for the router to process streaming events. It:
/// 1. Detects the source format of the stream chunk
/// 2. Transforms to target format if needed (pass-through if formats match)
/// 3. Extracts metadata like keep_alive and finish_reason
///
/// # Arguments
///
/// * `chunk` - The streaming chunk JSON payload
/// * `source_format` - The expected source format (from the provider being called)
/// * `target_format` - The target format to transform to
///
/// # Returns
///
/// A `ParsedStreamEvent` containing the payload and metadata.
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::parse_stream_event;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // OpenAI streaming chunk
/// let chunk = json!({
///     "id": "chatcmpl-123",
///     "choices": [{
///         "index": 0,
///         "delta": {"content": "Hello"},
///         "finish_reason": null
///     }]
/// });
///
/// let result = parse_stream_event(&chunk, ProviderFormat::OpenAI, ProviderFormat::OpenAI);
/// ```
pub fn parse_stream_event(
    chunk: &Value,
    source_format: ProviderFormat,
    target_format: ProviderFormat,
) -> Result<ParsedStreamEvent, TransformError> {
    // Step 1: Get adapters
    let source_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == source_format)
        .ok_or(TransformError::UnsupportedSourceFormat(source_format))?;

    // Step 2: Convert to universal
    let universal_opt = source_adapter.stream_to_universal(chunk)?;

    // Step 3: Extract metadata from universal chunk
    let (is_keep_alive, is_final) = match &universal_opt {
        Some(universal) => {
            let is_keep_alive = universal.is_keep_alive();
            let is_final = universal.choices.iter().any(|c| c.finish_reason.is_some());
            (is_keep_alive, is_final)
        }
        None => (true, false), // None means terminal/keep-alive event
    };

    // Step 4: Transform if needed
    if source_format == target_format {
        // Pass-through
        return Ok(ParsedStreamEvent {
            payload: chunk.clone(),
            source_format,
            target_format,
            universal: universal_opt,
            is_keep_alive,
            is_final,
        });
    }

    // Step 5: Get target adapter and transform
    let target_adapter = adapters()
        .into_iter()
        .find(|a| a.format() == target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    let payload = match &universal_opt {
        Some(universal) => target_adapter.stream_from_universal(universal)?,
        None => crate::serde_json::json!({}),
    };

    Ok(ParsedStreamEvent {
        payload,
        source_format,
        target_format,
        universal: universal_opt,
        is_keep_alive,
        is_final,
    })
}

// ============================================================================
// Helper APIs for direct adapter access
// ============================================================================

/// Convert a request payload to UniversalRequest using the specified format's adapter.
///
/// This is useful when you need to access the universal representation directly
/// without transforming to another format.
///
/// # Arguments
///
/// * `payload` - The request payload in the specified format
/// * `format` - The format of the payload
///
/// # Returns
///
/// The universal request representation, or an error if conversion fails.
pub fn to_universal_request(
    payload: &Value,
    format: ProviderFormat,
) -> Result<UniversalRequest, TransformError> {
    let adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedSourceFormat(format))?;
    adapter.request_to_universal(payload)
}

/// Convert a UniversalRequest to a provider-specific payload.
///
/// This is useful when you have a universal request and need to convert it
/// to a specific provider format.
///
/// # Arguments
///
/// * `universal` - The universal request to convert
/// * `format` - The target provider format
///
/// # Returns
///
/// The provider-specific payload, or an error if conversion fails.
pub fn from_universal_request(
    universal: &UniversalRequest,
    format: ProviderFormat,
) -> Result<Value, TransformError> {
    let adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    adapter.request_from_universal(universal)
}

/// Convert a response payload to UniversalResponse using the specified format's adapter.
///
/// # Arguments
///
/// * `payload` - The response payload in the specified format
/// * `format` - The format of the payload
///
/// # Returns
///
/// The universal response representation, or an error if conversion fails.
pub fn to_universal_response(
    payload: &Value,
    format: ProviderFormat,
) -> Result<UniversalResponse, TransformError> {
    let adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedSourceFormat(format))?;
    adapter.response_to_universal(payload)
}

/// Convert a UniversalResponse to a provider-specific payload.
///
/// # Arguments
///
/// * `universal` - The universal response to convert
/// * `format` - The target provider format
///
/// # Returns
///
/// The provider-specific payload, or an error if conversion fails.
pub fn from_universal_response(
    universal: &UniversalResponse,
    format: ProviderFormat,
) -> Result<Value, TransformError> {
    let adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    adapter.response_from_universal(universal)
}

/// Apply provider-specific defaults to a request payload in-place.
///
/// This converts the payload to universal format, applies the target provider's
/// defaults (e.g., Anthropic's required max_tokens), and converts back.
///
/// # Arguments
///
/// * `payload` - The request payload to modify
/// * `format` - The target provider format whose defaults should be applied
///
/// # Returns
///
/// Ok(()) on success, or an error if conversion fails.
pub fn apply_provider_defaults(
    payload: &mut Value,
    format: ProviderFormat,
) -> Result<(), TransformError> {
    let adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;

    // Detect source format to parse the payload
    let source_format = detect_source_format(payload)?;
    let source_adapter = adapter_for_format(source_format)
        .ok_or(TransformError::UnsupportedSourceFormat(source_format))?;

    // Convert to universal, apply defaults, convert back to source format
    let mut universal = source_adapter.request_to_universal(payload)?;
    adapter.apply_defaults(&mut universal);
    *payload = source_adapter.request_from_universal(&universal)?;

    Ok(())
}

/// Sanitize a payload for a target format by parsing and re-serializing.
///
/// This strips unknown fields that strict providers (like Anthropic) would reject.
/// Use this for pass-through payloads that might contain fields from other formats
/// (e.g., OpenAI's `stream_options` which Anthropic doesn't support).
///
/// # Arguments
///
/// * `payload` - The request payload to sanitize
/// * `format` - The target provider format
///
/// # Returns
///
/// The sanitized payload with only known fields, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::sanitize_payload;
/// use lingua::capabilities::ProviderFormat;
/// use lingua::serde_json::json;
///
/// // Anthropic payload with unknown `stream_options` field
/// let payload = json!({
///     "model": "claude-3-5-sonnet",
///     "max_tokens": 1024,
///     "messages": [{"role": "user", "content": "Hello"}],
///     "stream_options": {"include_usage": true}  // OpenAI-specific, unknown to Anthropic
/// });
///
/// // Sanitize for Anthropic - stream_options will be stripped
/// let sanitized = sanitize_payload(&payload, ProviderFormat::Anthropic).unwrap();
/// assert!(sanitized.get("stream_options").is_none());
/// ```
pub fn sanitize_payload(payload: &Value, format: ProviderFormat) -> Result<Value, TransformError> {
    use crate::providers::anthropic::try_parse_anthropic;

    // Provider-specific sanitization that parses into typed struct (drops unknown fields)
    // and re-serializes. This is different from round-tripping through universal which
    // preserves unknown fields in extras.
    match format {
        ProviderFormat::Anthropic => {
            let parsed = try_parse_anthropic(payload)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
            crate::serde_json::to_value(parsed)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))
        }
        // Other formats don't need sanitization (permissive) or aren't supported yet
        _ => Ok(payload.clone()),
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
                universal,
            } => {
                assert_eq!(source_format, ProviderFormat::OpenAI);
                assert!(
                    universal.is_some(),
                    "Request transformation should include universal"
                );
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

    #[test]
    #[cfg(all(feature = "openai", feature = "google"))]
    fn test_transform_request_openai_to_google() {
        // OpenAI request with system and duplicate user messages
        let payload = json!({
            "model": "gemini-2.5-flash",
            "max_tokens": 50,
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say 'hello' and nothing else."},
                {"role": "user", "content": "Say 'goodbye' after 'hello'."}
            ]
        });

        let result = transform_request(&payload, ProviderFormat::Google, None);
        assert!(
            result.is_ok(),
            "transform_request failed: {:?}",
            result.err()
        );

        let result = result.unwrap();
        assert!(!result.is_pass_through());

        let final_payload = result.payload_or_original(payload);

        // Should have contents (Google format), not messages
        assert!(final_payload.get("contents").is_some(), "missing contents");
        assert!(
            final_payload.get("messages").is_none(),
            "should not have messages"
        );

        // Should have systemInstruction extracted
        assert!(
            final_payload.get("systemInstruction").is_some(),
            "missing systemInstruction"
        );

        // Contents should have flattened user messages (consecutive user -> single user)
        let contents = final_payload.get("contents").unwrap().as_array().unwrap();
        assert_eq!(contents.len(), 1, "should have 1 content (flattened users)");
        assert_eq!(
            contents[0].get("role").and_then(|v| v.as_str()),
            Some("user")
        );
    }
}
