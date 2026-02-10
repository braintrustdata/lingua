/*!
Unified payload transformation API with bytes in/out.

This module provides a single entry point for validating and transforming
payloads between different provider formats. The key principle is:

**If a payload can be deserialized into the target provider struct, use it as-is (pass-through).**
**Otherwise, detect the source format, convert to universal, and transform to target format.**

All public functions take `Bytes` input and return `Bytes` output for zero-copy
passthrough in async contexts.
*/

use bytes::Bytes;

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::processing::adapters::{adapter_for_format, adapters, ProviderAdapter};
#[cfg(feature = "openai")]
use crate::providers::openai::model_needs_transforms;
use crate::serde_json::Value;
use crate::universal::{UniversalResponse, UniversalStreamChunk};
use thiserror::Error;

/// Static empty JSON object bytes for terminal/keep-alive events.
/// Using `Bytes::from_static` avoids allocation; `.clone()` is a cheap refcount bump.
static EMPTY_JSON: Bytes = Bytes::from_static(b"{}");

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

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Unsupported target format: {0:?}")]
    UnsupportedTargetFormat(ProviderFormat),

    #[error("Unsupported source format: {0:?}")]
    UnsupportedSourceFormat(ProviderFormat),

    #[error("Streaming not implemented: {0}")]
    StreamingNotImplemented(String),
}

impl TransformError {
    /// Returns true if this is a client-side error (user's fault).
    ///
    /// Client errors indicate invalid input or unsupported configurations
    /// that the user should fix in their request. This includes conversion
    /// failures which typically mean the user tried to use features that
    /// the target provider doesn't support.
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            TransformError::UnableToDetectFormat
                | TransformError::ValidationFailed { .. }
                | TransformError::DeserializationFailed(_)
                | TransformError::UnsupportedTargetFormat(_)
                | TransformError::UnsupportedSourceFormat(_)
                | TransformError::ToUniversalFailed(_)
                | TransformError::FromUniversalFailed(_)
        )
    }
}

impl From<ConvertError> for TransformError {
    fn from(err: ConvertError) -> Self {
        TransformError::FromUniversalFailed(err.to_string())
    }
}

/// Result of a transformation operation.
///
/// Contains either the original bytes (passthrough) or transformed bytes.
/// `Bytes::clone()` is O(1) - just a refcount bump - making this efficient
/// for async contexts where payloads are passed to multiple tasks.
#[derive(Debug, Clone)]
pub enum TransformResult {
    /// Payload was already valid for target format - returns the original bytes unchanged.
    /// This is the zero-copy path: no serialization occurred.
    PassThrough(Bytes),

    /// Payload was transformed to target format.
    Transformed {
        /// The transformed payload as bytes
        bytes: Bytes,
        /// The detected source format
        source_format: ProviderFormat,
    },
}

impl TransformResult {
    /// Check if this is a pass-through result (no transformation occurred).
    pub fn is_passthrough(&self) -> bool {
        matches!(self, TransformResult::PassThrough(_))
    }

    /// Consume the result and return the final bytes.
    ///
    /// Returns the original bytes for PassThrough, or the transformed bytes for Transformed.
    pub fn into_bytes(self) -> Bytes {
        match self {
            TransformResult::PassThrough(bytes) => bytes,
            TransformResult::Transformed { bytes, .. } => bytes,
        }
    }

    /// Get a reference to the final bytes.
    pub fn as_bytes(&self) -> &Bytes {
        match self {
            TransformResult::PassThrough(bytes) => bytes,
            TransformResult::Transformed { bytes, .. } => bytes,
        }
    }

    /// Get the source format if transformation occurred.
    ///
    /// Returns `None` for passthrough (source format == target format).
    pub fn source_format(&self) -> Option<ProviderFormat> {
        match self {
            TransformResult::PassThrough(_) => None,
            TransformResult::Transformed { source_format, .. } => Some(*source_format),
        }
    }
}

// ============================================================================
// Model extraction
// ============================================================================

/// Extract model name from request bytes without full transformation.
///
/// This is a fast path for routing decisions that only need the model name.
/// Parses just enough to find the model field.
///
/// # Returns
///
/// - `Some(model)` if a model field was found
/// - `None` if parsing fails or no model field exists (e.g., Google format has model in URL)
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::extract_model;
/// use bytes::Bytes;
///
/// let openai = Bytes::from(r#"{"model": "gpt-4", "messages": []}"#);
/// assert_eq!(extract_model(&openai), Some("gpt-4".to_string()));
///
/// let bedrock = Bytes::from(r#"{"modelId": "anthropic.claude-3", "messages": []}"#);
/// assert_eq!(extract_model(&bedrock), Some("anthropic.claude-3".to_string()));
/// ```
pub fn extract_model(input: &[u8]) -> Option<String> {
    let payload: Value = crate::serde_json::from_slice(input).ok()?;

    // Try common model field names across providers
    payload
        .get("model") // OpenAI, Anthropic
        .or_else(|| payload.get("modelId")) // Bedrock
        .and_then(|v| v.as_str())
        .map(String::from)
}

// ============================================================================
// Request transformation
// ============================================================================

/// Transform a request payload to the target format.
///
/// This is the main entry point for request transformation. It:
/// 1. Parses the input bytes to detect the format
/// 2. If already valid for target format, returns the original bytes (zero-copy passthrough)
/// 3. Otherwise, transforms via universal format and serializes to bytes
///
/// # Arguments
///
/// * `input` - The incoming request as bytes
/// * `target_format` - The target provider format
/// * `model` - Optional model name to inject if source doesn't have one (for Google/Bedrock)
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough(bytes))` - Payload is already valid, returns original bytes
/// * `Ok(TransformResult::Transformed { bytes, source_format })` - Transformed to target format
/// * `Err(TransformError)` - If transformation fails
///
/// # Examples
///
/// ```
/// use lingua::processing::transform::transform_request;
/// use lingua::capabilities::ProviderFormat;
/// use bytes::Bytes;
///
/// let openai_payload = Bytes::from(r#"{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}"#);
///
/// // Transform to Anthropic format
/// let result = transform_request(openai_payload, ProviderFormat::Anthropic, None).unwrap();
/// let output_bytes = result.into_bytes();
/// ```
pub fn transform_request(
    input: Bytes,
    target_format: ProviderFormat,
    model: Option<&str>,
) -> Result<TransformResult, TransformError> {
    let payload: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let source_adapter = detect_adapter(&payload, DetectKind::Request)?;

    if source_adapter.format() == target_format
        && !needs_forced_translation(&payload, model, target_format)
    {
        return Ok(TransformResult::PassThrough(input));
    }

    let source_format = source_adapter.format();
    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    let mut universal = source_adapter.request_to_universal(payload)?;

    // Inject model from parameter if not present
    if model.is_some() && universal.model.is_none() {
        universal.model = model.map(String::from);
    }

    // Apply target provider defaults (e.g., Anthropic's required max_tokens)
    target_adapter.apply_defaults(&mut universal);

    // Convert to target format (validation happens in adapter)
    let transformed = target_adapter.request_from_universal(&universal)?;

    let bytes = crate::serde_json::to_vec(&transformed)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

    Ok(TransformResult::Transformed {
        bytes: Bytes::from(bytes),
        source_format,
    })
}

// ============================================================================
// Response transformation
// ============================================================================

/// Transform a response payload from one format to another.
///
/// # Arguments
///
/// * `input` - The source response as bytes
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough(bytes))` - Response is already valid for target format
/// * `Ok(TransformResult::Transformed { bytes, source_format })` - Transformed response
/// * `Err(TransformError)` - If transformation fails
pub fn transform_response(
    input: Bytes,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    let response: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let source_adapter = detect_adapter(&response, DetectKind::Response)?;

    if source_adapter.format() == target_format {
        return Ok(TransformResult::PassThrough(input));
    }

    let source_format = source_adapter.format();
    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    let universal_resp = source_adapter.response_to_universal(response)?;
    let transformed = target_adapter.response_from_universal(&universal_resp)?;

    let bytes = crate::serde_json::to_vec(&transformed)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

    Ok(TransformResult::Transformed {
        bytes: Bytes::from(bytes),
        source_format,
    })
}

/// Parse a response to UniversalResponse without re-serializing.
///
/// This is useful when you need to extract data from the universal representation
/// (e.g., usage metrics) before converting to a target format.
///
/// # Arguments
///
/// * `input` - The source response as bytes (any supported provider format)
///
/// # Returns
///
/// * `Ok(UniversalResponse)` - The parsed universal response
/// * `Err(TransformError)` - If parsing fails
pub fn response_to_universal(input: Bytes) -> Result<UniversalResponse, TransformError> {
    let response: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let source_adapter = detect_adapter(&response, DetectKind::Response)?;
    source_adapter.response_to_universal(response)
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
/// * `input` - The source streaming chunk as bytes
/// * `target_format` - The target provider format
///
/// # Returns
///
/// * `Ok(TransformResult::PassThrough(bytes))` - Chunk is already valid for target format
/// * `Ok(TransformResult::Transformed { bytes, source_format })` - Transformed chunk
/// * `Err(TransformError)` - If transformation fails
pub fn transform_stream_chunk(
    input: Bytes,
    target_format: ProviderFormat,
) -> Result<TransformResult, TransformError> {
    let chunk: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let source_adapter = detect_adapter(&chunk, DetectKind::Stream)?;

    if source_adapter.format() == target_format {
        return Ok(TransformResult::PassThrough(input));
    }

    let source_format = source_adapter.format();
    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    let stream_universal = source_adapter.stream_to_universal(chunk)?;

    let bytes = match stream_universal {
        Some(universal_chunk) => {
            let transformed = target_adapter.stream_from_universal(&universal_chunk)?;
            Bytes::from(
                crate::serde_json::to_vec(&transformed)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            )
        }
        None => EMPTY_JSON.clone(), // Terminal event - cheap refcount bump
    };

    Ok(TransformResult::Transformed {
        bytes,
        source_format,
    })
}

// ============================================================================
// Stream event parsing (for router integration)
// ============================================================================

/// Result of parsing a streaming event.
///
/// Contains the transformed bytes, metadata about the event, and optionally
/// the universal representation for further processing.
#[derive(Debug, Clone)]
pub struct ParsedStreamEvent {
    /// The payload to forward (original if pass-through, transformed otherwise)
    pub bytes: Bytes,
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
/// * `input` - The streaming chunk as bytes
/// * `source_format` - The expected source format (from the provider being called)
/// * `target_format` - The target format to transform to
///
/// # Returns
///
/// A `ParsedStreamEvent` containing the bytes and metadata.
pub fn parse_stream_event(
    input: Bytes,
    source_format: ProviderFormat,
    target_format: ProviderFormat,
) -> Result<ParsedStreamEvent, TransformError> {
    let chunk: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let source_adapter = adapter_for_format(source_format)
        .ok_or(TransformError::UnsupportedSourceFormat(source_format))?;

    let universal_opt = source_adapter.stream_to_universal(chunk)?;

    let (is_keep_alive, is_final) = match &universal_opt {
        Some(universal) => {
            let is_keep_alive = universal.is_keep_alive();
            let is_final = universal.choices.iter().any(|c| c.finish_reason.is_some());
            (is_keep_alive, is_final)
        }
        None => (true, false), // None means terminal/keep-alive event
    };

    if source_format == target_format {
        return Ok(ParsedStreamEvent {
            bytes: input,
            source_format,
            target_format,
            universal: universal_opt,
            is_keep_alive,
            is_final,
        });
    }

    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    let bytes = match &universal_opt {
        Some(universal) => {
            let transformed = target_adapter.stream_from_universal(universal)?;
            Bytes::from(
                crate::serde_json::to_vec(&transformed)
                    .map_err(|e| TransformError::SerializationFailed(e.to_string()))?,
            )
        }
        None => EMPTY_JSON.clone(), // Terminal event - cheap refcount bump
    };

    Ok(ParsedStreamEvent {
        bytes,
        source_format,
        target_format,
        universal: universal_opt,
        is_keep_alive,
        is_final,
    })
}

// ============================================================================
// Internal helpers
// ============================================================================

#[derive(Copy, Clone)]
enum DetectKind {
    Request,
    Response,
    Stream,
}

fn detect_adapter(
    payload: &Value,
    kind: DetectKind,
) -> Result<&'static dyn ProviderAdapter, TransformError> {
    adapters()
        .iter()
        .find(|a| match kind {
            DetectKind::Request => a.detect_request(payload),
            DetectKind::Response => a.detect_response(payload),
            DetectKind::Stream => a.detect_stream_response(payload),
        })
        .map(|a| a.as_ref())
        .ok_or(TransformError::UnableToDetectFormat)
}

/// Check if a request needs forced translation even when source == target format.
fn needs_forced_translation(payload: &Value, model: Option<&str>, target: ProviderFormat) -> bool {
    if target != ProviderFormat::ChatCompletions {
        return false;
    }

    #[cfg(feature = "openai")]
    {
        // Force translation if model needs any transforms (temperature stripping, max_tokens conversion, etc.)
        let request_model = payload.get("model").and_then(Value::as_str).or(model);
        request_model.map(model_needs_transforms).unwrap_or(false)
    }

    #[cfg(not(feature = "openai"))]
    false
}

/// Sanitize a payload for a target format by parsing and re-serializing.
///
/// This strips unknown fields that strict providers (like Anthropic) would reject.
pub fn sanitize_payload(input: Bytes, format: ProviderFormat) -> Result<Bytes, TransformError> {
    use crate::providers::anthropic::try_parse_anthropic;

    let payload: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;

    let sanitized = match format {
        ProviderFormat::Anthropic => {
            let parsed = try_parse_anthropic(&payload)
                .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;
            crate::serde_json::to_value(parsed)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?
        }
        _ => payload,
    };

    let bytes = crate::serde_json::to_vec(&sanitized)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

    Ok(Bytes::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    fn to_bytes(value: &Value) -> Bytes {
        Bytes::from(crate::serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn test_extract_model_openai() {
        let input = br#"{"model": "gpt-4", "messages": []}"#;
        assert_eq!(extract_model(input), Some("gpt-4".to_string()));
    }

    #[test]
    fn test_extract_model_bedrock() {
        let input = br#"{"modelId": "anthropic.claude-3", "messages": []}"#;
        assert_eq!(extract_model(input), Some("anthropic.claude-3".to_string()));
    }

    #[test]
    fn test_extract_model_google() {
        // Google format doesn't have model in body
        let input = br#"{"contents": []}"#;
        assert_eq!(extract_model(input), None);
    }

    #[test]
    fn test_extract_model_invalid_json() {
        let input = br#"not valid json"#;
        assert_eq!(extract_model(input), None);
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_passthrough() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);
        let input_ptr = input.as_ptr();

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        // Should be passthrough
        assert!(result.is_passthrough());

        // Should return the exact same bytes (pointer equality)
        let output = result.into_bytes();
        assert_eq!(output.as_ptr(), input_ptr);
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_openai_to_anthropic() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        // Should be transformed
        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        // Parse output and verify
        let output_bytes = result.into_bytes();
        let output: Value = crate::serde_json::from_slice(&output_bytes).unwrap();

        // Should have max_tokens added (Anthropic default)
        assert!(output.get("max_tokens").is_some());
        // Should have messages
        assert!(output.get("messages").is_some());
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_transform_request_anthropic_passthrough() {
        let payload = json!({
            "model": "claude-3-5-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        // Should be passthrough
        assert!(result.is_passthrough());
    }

    #[test]
    fn test_transform_request_invalid_json() {
        let input = Bytes::from("not valid json");

        let result = transform_request(input, ProviderFormat::ChatCompletions, None);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransformError::DeserializationFailed(_)
        ));
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_response() {
        // OpenAI response format
        let payload = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }]
        });
        let input = to_bytes(&payload);

        // Transform to Anthropic format
        let result = transform_response(input, ProviderFormat::Anthropic).unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        // Verify output is valid JSON
        let output_bytes = result.into_bytes();
        let output: Value = crate::serde_json::from_slice(&output_bytes).unwrap();
        assert!(output.get("content").is_some() || output.get("choices").is_some());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_response_passthrough() {
        let payload = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }]
        });
        let input = to_bytes(&payload);
        let input_ptr = input.as_ptr();

        let result = transform_response(input, ProviderFormat::ChatCompletions).unwrap();

        // Should be passthrough with same bytes
        assert!(result.is_passthrough());
        assert_eq!(result.into_bytes().as_ptr(), input_ptr);
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_reasoning_model_forces_translation() {
        // Reasoning models (gpt-5.*, o1-*, o3-*) require max_completion_tokens
        // Even for OpenAI → OpenAI, we must translate to convert max_tokens → max_completion_tokens
        // Note: Include an OpenAI-only field (seed) to ensure detection as OpenAI format
        let payload = json!({
            "model": "gpt-5.1-mini",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 1000,
            "seed": 42
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        // Should NOT passthrough - reasoning models need max_completion_tokens
        assert!(
            !result.is_passthrough(),
            "Reasoning models should force translation"
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(
            output.get("max_completion_tokens").is_some(),
            "Should have max_completion_tokens"
        );
        assert!(
            output.get("max_tokens").is_none(),
            "Should not have max_tokens"
        );
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_non_reasoning_model_still_passthroughs() {
        // Non-reasoning models should still passthrough for efficiency
        // Note: Include an OpenAI-only field (seed) to ensure detection as OpenAI format
        let payload = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 1000,
            "seed": 42
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        // Should passthrough - gpt-4o is not a reasoning model
        assert!(
            result.is_passthrough(),
            "Non-reasoning models should passthrough"
        );
    }
}
