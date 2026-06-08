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
use crate::processing::normalize_json_lone_surrogate_escapes;
#[cfg(feature = "openai")]
use crate::providers::openai::model_needs_transforms;
use crate::serde_json;
use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UniversalReasoningDelta, UniversalResponse,
    UniversalStreamChoice, UniversalStreamChunk, UniversalStreamDelta, UniversalToolCallDelta,
    UniversalToolFunctionDelta,
};
use serde::de::DeserializeOwned;
use thiserror::Error;

/// Static empty JSON object bytes for terminal/keep-alive events.
/// Using `Bytes::from_static` avoids allocation; `.clone()` is a cheap refcount bump.
static EMPTY_JSON: Bytes = Bytes::from_static(b"{}");

/// Error type for transformation operations
#[derive(Debug, Error)]
pub enum TransformError {
    #[error("Unable to detect source format")]
    UnableToDetectFormat,

    #[error("Unable to detect request source format")]
    UnableToDetectRequestFormat,

    #[error("Unable to detect response source format")]
    UnableToDetectResponseFormat,

    #[error("Unable to detect stream source format")]
    UnableToDetectStreamFormat,

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
                | TransformError::UnableToDetectRequestFormat
                | TransformError::UnableToDetectResponseFormat
                | TransformError::UnableToDetectStreamFormat
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
        /// The format the payload was actually transformed into.
        ///
        /// Usually equal to the `target_format` passed to the transform function, but may
        /// differ when the transform function upgrades the target (e.g. `ChatCompletions` →
        /// `Responses` when `reasoning_effort` + `tools` are present).
        actual_target_format: ProviderFormat,
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

pub(crate) struct StreamTransformStep {
    pub(crate) result: TransformResult,
    pub(crate) source_format: ProviderFormat,
    pub(crate) source_is_native_stream: bool,
    pub(crate) universal: Option<UniversalStreamChunk>,
    pub(crate) event_type: Option<String>,
    pub(crate) is_passthrough: bool,
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
    let payload = parse_json_value(input).ok()?;

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
/// * `model` - Optional model name to use for the transformed target request
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
/// Returns `true` when a Chat Completions request should be redirected to the Responses API.
///
/// The `/v1/chat/completions` endpoint rejects requests that combine `reasoning_effort`
/// with function `tools` for certain models (e.g. `gpt-5.4-mini`). The Responses API
/// (`/v1/responses`) supports this combination, so we upgrade the target automatically.
#[cfg(feature = "openai")]
fn chat_completions_needs_responses_upgrade(payload: &Value) -> bool {
    let has_reasoning_effort = payload
        .get("reasoning_effort")
        .and_then(Value::as_str)
        .is_some_and(|e| e != "none");

    let has_tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|t| !t.is_empty());

    has_reasoning_effort && has_tools
}

#[cfg(feature = "openai")]
fn chat_completions_model_disables_responses_upgrade(model: &str) -> bool {
    model.starts_with("gemini-") || model.starts_with("models/gemini-")
}

#[cfg(feature = "openai")]
fn chat_completions_request_model(request_bytes: &[u8]) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct RequestModel {
        model: Option<String>,
    }

    parse_json::<RequestModel>(request_bytes)
        .ok()
        .and_then(|request| request.model)
}

#[cfg(feature = "openai")]
fn chat_completions_upgrade_model(
    model: Option<&str>,
    request_model: Option<&str>,
) -> Option<String> {
    if let Some(model) = model {
        return Some(model.to_string());
    }

    request_model.map(str::to_string)
}

pub fn transform_request(
    input: Bytes,
    target_format: ProviderFormat,
    model: Option<&str>,
) -> Result<TransformResult, TransformError> {
    let parsed = parse_json_body(input)?;
    let payload = parsed.value;
    let request_bytes = parsed.bytes;

    let source_adapter = detect_adapter(&payload, DetectKind::Request)?;

    #[cfg(feature = "openai")]
    let request_model = chat_completions_request_model(&request_bytes);
    #[cfg(feature = "openai")]
    let upgrade_model = chat_completions_upgrade_model(model, request_model.as_deref());
    #[cfg(not(feature = "openai"))]
    let request_model: Option<String> = None;
    #[cfg(not(feature = "openai"))]
    let upgrade_model = model.map(str::to_string);

    // Upgrade ChatCompletions → Responses when reasoning_effort + tools are
    // both present, except for OpenAI-compatible providers that do not support
    // the Responses API.
    #[cfg(feature = "openai")]
    let target_format = if target_format == ProviderFormat::ChatCompletions
        && chat_completions_needs_responses_upgrade(&payload)
        && !upgrade_model
            .as_deref()
            .is_some_and(chat_completions_model_disables_responses_upgrade)
    {
        ProviderFormat::Responses
    } else {
        target_format
    };

    let source_format = source_adapter.format();
    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;

    if source_format == target_format
        && !request_model_needs_forced_translation(request_model.as_deref(), model, target_format)
        && target_adapter.detect_passthrough_request(&payload)
    {
        return Ok(TransformResult::PassThrough(request_bytes));
    }

    let mut universal = source_adapter.request_to_universal(payload)?;

    if let Some(model) = model {
        universal.model = Some(model.to_string());
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
        actual_target_format: target_format,
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
    let parsed = parse_json_body(input)?;
    let response = parsed.value;
    let response_bytes = parsed.bytes;

    let source_adapter = detect_adapter(&response, DetectKind::Response)?;

    if source_adapter.format() == target_format {
        return Ok(TransformResult::PassThrough(response_bytes));
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
        actual_target_format: target_format,
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
    let response = parse_json_value(&input)?;

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
    Ok(transform_stream_chunk_step(input, target_format, true)?.result)
}

fn extract_event_type(value: &Value) -> Option<String> {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn serialize_stream_value(value: &Value) -> Result<Bytes, TransformError> {
    Ok(Bytes::from(crate::serde_json::to_vec(value).map_err(
        |e| TransformError::SerializationFailed(e.to_string()),
    )?))
}

fn assistant_content_to_stream_delta(content: &AssistantContent) -> UniversalStreamDelta {
    match content {
        AssistantContent::String(text) => UniversalStreamDelta {
            role: Some("assistant".to_string()),
            content: Some(text.clone()),
            ..Default::default()
        },
        AssistantContent::Array(parts) => {
            let mut text = String::new();
            let mut reasoning = Vec::new();
            let mut tool_calls = Vec::new();
            let mut reasoning_signature = None;

            for part in parts {
                match part {
                    AssistantContentPart::Text(text_part) => {
                        text.push_str(&text_part.text);
                    }
                    AssistantContentPart::Reasoning {
                        text,
                        encrypted_content,
                    } => {
                        reasoning.push(UniversalReasoningDelta {
                            content: Some(text.clone()),
                        });
                        if reasoning_signature.is_none() {
                            reasoning_signature = encrypted_content.clone();
                        }
                    }
                    AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        ..
                    } => {
                        let tool_call_index = tool_calls.len() as u32;
                        tool_calls.push(UniversalToolCallDelta {
                            index: Some(tool_call_index),
                            id: Some(tool_call_id.clone()),
                            call_type: Some("function".to_string()),
                            function: Some(UniversalToolFunctionDelta {
                                name: Some(tool_name.clone()),
                                arguments: Some(arguments.to_string()),
                            }),
                        });
                    }
                    AssistantContentPart::File { .. } | AssistantContentPart::ToolResult { .. } => {
                    }
                }
            }

            UniversalStreamDelta {
                role: Some("assistant".to_string()),
                content: (!text.is_empty()).then_some(text),
                tool_calls,
                reasoning,
                reasoning_signature,
            }
        }
    }
}

fn response_to_stream_chunk(response: UniversalResponse) -> UniversalStreamChunk {
    let choices = response
        .messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| match message {
            Message::Assistant { content, .. } => Some(UniversalStreamChoice {
                index: index as u32,
                delta: Some(Value::from(assistant_content_to_stream_delta(content))),
                finish_reason: response.finish_reason.as_ref().map(ToString::to_string),
            }),
            Message::System { .. }
            | Message::Developer { .. }
            | Message::User { .. }
            | Message::Tool { .. } => None,
        })
        .collect();

    UniversalStreamChunk::new(response.id, response.model, choices, None, response.usage)
}

pub(crate) fn transform_stream_chunk_step(
    input: Bytes,
    target_format: ProviderFormat,
    allow_full_response_fallback: bool,
) -> Result<StreamTransformStep, TransformError> {
    let parsed = parse_json_body(input)?;
    let chunk = parsed.value;
    let chunk_bytes = parsed.bytes;
    let event_type = extract_event_type(&chunk);

    let detection =
        detect_adapter_with_kind(&chunk, DetectKind::Stream, allow_full_response_fallback)?;
    let source_adapter = detection.adapter;
    let source_format = source_adapter.format();
    let source_is_native_stream = matches!(detection.kind, DetectKind::Stream);
    let universal = match detection.kind {
        DetectKind::Stream => source_adapter.stream_to_universal(chunk)?,
        DetectKind::Response => {
            let response = source_adapter.response_to_universal(chunk)?;
            Some(response_to_stream_chunk(response))
        }
        DetectKind::Request => {
            unreachable!("stream detection never falls back to request payloads")
        }
    };

    if source_format == target_format && matches!(detection.kind, DetectKind::Stream) {
        return Ok(StreamTransformStep {
            result: TransformResult::PassThrough(chunk_bytes),
            source_format,
            source_is_native_stream,
            universal,
            event_type,
            is_passthrough: true,
        });
    }

    let target_adapter = adapter_for_format(target_format)
        .ok_or(TransformError::UnsupportedTargetFormat(target_format))?;
    let bytes = match &universal {
        Some(universal_chunk) => {
            serialize_stream_value(&target_adapter.stream_from_universal(universal_chunk)?)?
        }
        None => EMPTY_JSON.clone(),
    };

    Ok(StreamTransformStep {
        result: TransformResult::Transformed {
            bytes,
            source_format,
            actual_target_format: target_format,
        },
        source_format,
        source_is_native_stream,
        universal,
        event_type: None,
        is_passthrough: false,
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

struct AdapterDetection {
    adapter: &'static dyn ProviderAdapter,
    kind: DetectKind,
}

fn detect_adapter(
    payload: &Value,
    kind: DetectKind,
) -> Result<&'static dyn ProviderAdapter, TransformError> {
    detect_adapter_with_kind(payload, kind, true).map(|detection| detection.adapter)
}

fn detect_adapter_with_kind(
    payload: &Value,
    kind: DetectKind,
    allow_full_response_fallback: bool,
) -> Result<AdapterDetection, TransformError> {
    let adapter = detect_adapter_exact(payload, kind);
    if let Some(adapter) = adapter {
        return Ok(AdapterDetection { adapter, kind });
    }

    // Vertex will possibly respond with a response payload for streaming requests, so we need to detect that.
    if matches!(kind, DetectKind::Stream) && allow_full_response_fallback {
        if let Some(adapter) = detect_adapter_exact(payload, DetectKind::Response) {
            return Ok(AdapterDetection {
                adapter,
                kind: DetectKind::Response,
            });
        }
    }

    Err(unable_to_detect_format_error(kind))
}

fn unable_to_detect_format_error(kind: DetectKind) -> TransformError {
    match kind {
        DetectKind::Request => TransformError::UnableToDetectRequestFormat,
        DetectKind::Response => TransformError::UnableToDetectResponseFormat,
        DetectKind::Stream => TransformError::UnableToDetectStreamFormat,
    }
}

fn detect_adapter_exact(payload: &Value, kind: DetectKind) -> Option<&'static dyn ProviderAdapter> {
    adapters()
        .iter()
        .find(|a| match kind {
            DetectKind::Request => a.detect_request(payload),
            DetectKind::Response => a.detect_response(payload),
            DetectKind::Stream => a.detect_stream_response(payload),
        })
        .map(|a| a.as_ref())
}

#[cfg(feature = "openai")]
fn request_model_needs_forced_translation(
    request_model: Option<&str>,
    override_model: Option<&str>,
    target: ProviderFormat,
) -> bool {
    if !matches!(
        target,
        ProviderFormat::ChatCompletions | ProviderFormat::Responses
    ) {
        return false;
    }

    if request_model.map(model_needs_transforms).unwrap_or(false) {
        return true;
    }

    if override_model.map(model_needs_transforms).unwrap_or(false) {
        return true;
    }

    target == ProviderFormat::ChatCompletions
        && request_model.is_some_and(is_models_prefixed_gemini_model)
        && override_model.is_some_and(is_bare_gemini_model)
}

#[cfg(feature = "openai")]
fn is_models_prefixed_gemini_model(model: &str) -> bool {
    model.starts_with("models/gemini-")
}

#[cfg(feature = "openai")]
fn is_bare_gemini_model(model: &str) -> bool {
    model.starts_with("gemini-")
}

#[cfg(not(feature = "openai"))]
fn request_model_needs_forced_translation(
    _request_model: Option<&str>,
    _override_model: Option<&str>,
    _target: ProviderFormat,
) -> bool {
    false
}

/// Sanitize a payload for a target format by parsing and re-serializing.
///
/// This strips unknown fields that strict providers (like Anthropic) would reject.
pub fn sanitize_payload(input: Bytes, format: ProviderFormat) -> Result<Bytes, TransformError> {
    use crate::providers::anthropic::try_parse_anthropic;

    let payload = parse_json_value(&input)?;

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

pub fn parse_json_value(input: &[u8]) -> Result<Value, TransformError> {
    parse_json(input).map_err(|err| TransformError::DeserializationFailed(err.to_string()))
}

pub struct ParsedJsonBody {
    pub value: Value,
    pub bytes: Bytes,
}

pub fn parse_json_body(input: Bytes) -> Result<ParsedJsonBody, TransformError> {
    match serde_json::from_slice(&input) {
        Ok(value) => Ok(ParsedJsonBody {
            value,
            bytes: input,
        }),
        Err(original) => {
            let Some(repaired) = normalize_json_lone_surrogate_escapes(&input) else {
                return Err(TransformError::DeserializationFailed(original.to_string()));
            };

            let repaired = Bytes::from(repaired);
            let value = serde_json::from_slice(&repaired)
                .map_err(|err| TransformError::DeserializationFailed(err.to_string()))?;
            Ok(ParsedJsonBody {
                value,
                bytes: repaired,
            })
        }
    }
}

pub fn parse_json<T>(input: &[u8]) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    match serde_json::from_slice(input) {
        Ok(value) => Ok(value),
        Err(original) => {
            let Some(repaired) = normalize_json_lone_surrogate_escapes(input) else {
                return Err(original);
            };
            serde_json::from_slice(&repaired)
        }
    }
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
    #[cfg(feature = "openai")]
    fn test_transform_request_passthrough_repairs_lone_surrogate() {
        let input = Bytes::from_static(
            br#"{"model":"gpt-4","messages":[{"role":"user","content":"bad \uD83D text"}]}"#,
        );
        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        assert!(result.is_passthrough());
        let output = result.into_bytes();
        assert_eq!(
            output,
            Bytes::from_static(
                br#"{"model":"gpt-4","messages":[{"role":"user","content":"bad \uFFFD text"}]}"#
            )
        );
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_preserves_suffix_messages_in_chat_rewrite() {
        let payload = json!({
            "model": "brain-facet-1",
            "messages": [
                { "role": "system", "content": "Classify each request." },
                {
                    "role": "user",
                    "content": "Shared preprocessed conversation text for topic facets."
                }
            ],
            "suffix_messages": [
                [{ "role": "user", "content": "Does this mention billing?" }],
                [{ "role": "user", "content": "Does this mention deployment?" }]
            ],
            "stream": false,
            "max_tokens": 20000,
            "chat_template_kwargs": { "enable_thinking": false }
        });
        let input = to_bytes(&payload);

        let result =
            transform_request(input, ProviderFormat::ChatCompletions, Some("gpt-5-nano")).unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(
            output.get("model").and_then(Value::as_str),
            Some("gpt-5-nano")
        );
        assert!(output.get("max_tokens").is_none());
        assert_eq!(
            output.get("max_completion_tokens").and_then(Value::as_i64),
            Some(20000)
        );
        assert_eq!(
            output.get("suffix_messages"),
            payload.get("suffix_messages")
        );
        assert_eq!(
            output.get("chat_template_kwargs"),
            payload.get("chat_template_kwargs")
        );
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_rejects_truncated_unicode_escape() {
        let input = Bytes::from_static(
            br#"{"model":"gpt-4","messages":[{"role":"user","content":"bad \uD83"}]}"#,
        );
        let result = transform_request(input, ProviderFormat::ChatCompletions, None);

        assert!(matches!(
            result.unwrap_err(),
            TransformError::DeserializationFailed(_)
        ));
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
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_openai_system_role_to_anthropic_top_level_system() {
        let payload = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 50,
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say hello."}
            ]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(
            output.get("system").and_then(Value::as_str),
            Some("You are a helpful assistant.")
        );
        let messages = output
            .get("messages")
            .and_then(Value::as_array)
            .expect("messages should be an array");
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].get("role").and_then(Value::as_str),
            Some("user")
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_transform_request_opus_4_8_mid_conversation_system_passthrough() {
        let payload = json!({
            "model": "claude-opus-4-8",
            "max_tokens": 50,
            "messages": [
                {"role": "user", "content": "Review this function."},
                {"role": "system", "content": "From now on, include type annotations."}
            ]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        assert!(result.is_passthrough());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_openai_empty_tools_not_detected_as_anthropic() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "tools": []
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_invalid_leading_anthropic_system_role_not_detected_as_anthropic() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 1024,
            "system": "Top-level instructions.",
            "messages": [
                {"role": "system", "content": "Leading message instructions."},
                {"role": "user", "content": "Hello"}
            ]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(
            output.get("system").and_then(Value::as_str),
            Some("Leading message instructions.")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_chat_completions_does_not_add_anthropic_cache_control() {
        let payload = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "Be helpful."},
                {"role": "user", "content": "Hello"}
            ]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Anthropic, None).unwrap();

        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("cache_control").is_none());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_claude_code_messages_to_openai() {
        let payload = json!({
            "model": "gpt-5.5",
            "max_tokens": 32000,
            "system": [
                {"type": "text", "text": "You are running inside Claude Code."},
                {
                    "type": "text",
                    "text": "Preserve the user's coding instructions.",
                    "cache_control": {"type": "ephemeral"}
                }
            ],
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "hello world",
                            "cache_control": {"type": "ephemeral"}
                        }
                    ]
                },
                {"role": "system", "content": "Use the provided tools exactly."}
            ],
            "tools": [
                {
                    "name": "Read",
                    "description": "Read a file.",
                    "input_schema": {
                        "type": "object",
                        "properties": {"file_path": {"type": "string"}},
                        "required": ["file_path"],
                        "additionalProperties": false
                    }
                }
            ],
            "thinking": {"type": "adaptive"},
            "output_config": {"effort": "high"}
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(result.source_format(), Some(ProviderFormat::Anthropic));

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(output.get("model").and_then(Value::as_str), Some("gpt-5.5"));
        assert_eq!(
            output.get("max_completion_tokens").and_then(Value::as_i64),
            Some(32000)
        );
        assert!(output.get("max_tokens").is_none());
        assert_eq!(
            output.get("reasoning_effort").and_then(Value::as_str),
            Some("high")
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_transform_request_claude_code_messages_to_google() {
        let payload = json!({
            "model": "gemini-2.5-flash",
            "max_tokens": 1024,
            "system": [{"type": "text", "text": "You are running inside Claude Code."}],
            "messages": [
                {"role": "user", "content": "hello world"},
                {"role": "system", "content": "Use the provided tools exactly."}
            ],
            "thinking": {"type": "adaptive"}
        });
        let input = to_bytes(&payload);

        let result =
            transform_request(input, ProviderFormat::Google, Some("gemini-2.5-flash")).unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(result.source_format(), Some(ProviderFormat::Anthropic));

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(
            output.get("model").and_then(Value::as_str),
            Some("gemini-2.5-flash")
        );
        assert!(output.get("contents").is_some());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_openai_system_only_to_anthropic_rejected() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "system", "content": "Always say ok."}]
        });
        let input = to_bytes(&payload);

        let err = transform_request(input, ProviderFormat::Anthropic, None).unwrap_err();

        assert!(err.is_client_error());
        match err {
            TransformError::ValidationFailed { target, reason } => {
                assert_eq!(target, ProviderFormat::Anthropic);
                assert!(reason.contains("at least one non-system message"));
                assert!(reason.contains("system prompt alone cannot be sent"));
            }
            other => panic!("expected Anthropic validation failure, got {other:?}"),
        }
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_request_empty_openai_messages_to_anthropic_rejected() {
        let payload = json!({
            "model": "gpt-4",
            "messages": []
        });
        let input = to_bytes(&payload);

        let err = transform_request(input, ProviderFormat::Anthropic, None).unwrap_err();

        assert!(err.is_client_error());
        match err {
            TransformError::ValidationFailed { target, reason } => {
                assert_eq!(target, ProviderFormat::Anthropic);
                assert_eq!(
                    reason,
                    "Anthropic requires at least one message in 'messages'."
                );
            }
            other => panic!("expected Anthropic validation failure, got {other:?}"),
        }
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "google"))]
    fn test_transform_request_openai_system_only_to_google_rejected() {
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "system", "content": "Always say ok."}]
        });
        let input = to_bytes(&payload);

        let err = transform_request(input, ProviderFormat::Google, None).unwrap_err();

        assert!(err.is_client_error());
        match err {
            TransformError::ValidationFailed { target, reason } => {
                assert_eq!(target, ProviderFormat::Google);
                assert!(reason.contains("at least one non-system message"));
                assert!(reason.contains("system prompt alone cannot be sent"));
            }
            other => panic!("expected Google validation failure, got {other:?}"),
        }
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "google"))]
    fn test_transform_request_empty_openai_messages_to_google_rejected() {
        let payload = json!({
            "model": "gpt-4",
            "messages": []
        });
        let input = to_bytes(&payload);

        let err = transform_request(input, ProviderFormat::Google, None).unwrap_err();

        assert!(err.is_client_error());
        match err {
            TransformError::ValidationFailed { target, reason } => {
                assert_eq!(target, ProviderFormat::Google);
                assert_eq!(
                    reason,
                    "Google requires at least one message in 'contents'."
                );
            }
            other => panic!("expected Google validation failure, got {other:?}"),
        }
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
    fn test_transform_request_unable_to_detect_mentions_request() {
        let input = Bytes::from_static(br#"{"not":"a supported request shape"}"#);

        let err = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap_err();

        assert!(matches!(err, TransformError::UnableToDetectRequestFormat));
        assert_eq!(err.to_string(), "Unable to detect request source format");
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
    fn test_transform_response_unable_to_detect_mentions_response() {
        let input = Bytes::from_static(br#"{"not":"a supported response shape"}"#);

        let err = transform_response(input, ProviderFormat::ChatCompletions).unwrap_err();

        assert!(matches!(err, TransformError::UnableToDetectResponseFormat));
        assert_eq!(err.to_string(), "Unable to detect response source format");
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_detection_falls_back_to_full_response() {
        let payload = json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-haiku-4-5",
            "content": [{
                "type": "text",
                "text": "Hello from Vertex"
            }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 8,
                "output_tokens": 4
            }
        });

        assert!(detect_adapter_exact(&payload, DetectKind::Stream).is_none());

        let detection = detect_adapter_with_kind(&payload, DetectKind::Stream, true).unwrap();

        assert_eq!(detection.adapter.format(), ProviderFormat::Anthropic);
        assert!(matches!(detection.kind, DetectKind::Response));
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_stream_chunk_accepts_full_anthropic_message() {
        let payload = json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-haiku-4-5",
            "content": [{
                "type": "text",
                "text": "Hello from Vertex"
            }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 8,
                "output_tokens": 4
            }
        });
        let input = to_bytes(&payload);

        let result = transform_stream_chunk(input, ProviderFormat::ChatCompletions).unwrap();
        let output_bytes = result.into_bytes();
        let output: Value = crate::serde_json::from_slice(&output_bytes).unwrap();

        assert_eq!(
            output
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("delta"))
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str),
            Some("Hello from Vertex")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_transform_stream_chunk_accepts_full_anthropic_tool_call_message() {
        let payload = json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-haiku-4-5",
            "content": [{
                "type": "tool_use",
                "id": "toolu_test",
                "name": "get_weather",
                "input": {
                    "location": "San Francisco, CA"
                }
            }],
            "stop_reason": "tool_use",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 8,
                "output_tokens": 4
            }
        });
        let input = to_bytes(&payload);

        let result = transform_stream_chunk(input, ProviderFormat::ChatCompletions).unwrap();
        let output_bytes = result.into_bytes();
        let output: Value = crate::serde_json::from_slice(&output_bytes).unwrap();

        let tool_call = output
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("tool_calls"))
            .and_then(Value::as_array)
            .and_then(|tool_calls| tool_calls.first())
            .unwrap();

        assert_eq!(
            tool_call
                .get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str),
            Some("get_weather")
        );
        assert_eq!(
            output
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("finish_reason"))
                .and_then(Value::as_str),
            Some("tool_calls")
        );
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
    fn test_reasoning_responses_model_forces_translation() {
        let payload = json!({
            "model": "gpt-5.1-mini",
            "input": [{"role": "user", "content": "Hello"}],
            "top_p": 0.9
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Responses, None).unwrap();

        assert!(
            !result.is_passthrough(),
            "Reasoning Responses models should force translation"
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("top_p").is_none(), "Should not have top_p");
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_reasoning_responses_file_url_preserves_absent_filename() {
        let payload = json!({
            "model": "gpt-5-mini",
            "input": [{
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "Summarize the document."
                    },
                    {
                        "type": "input_file",
                        "file_url": "https://www.berkshirehathaway.com/letters/2024ltr.pdf"
                    }
                ]
            }]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::Responses, None).unwrap();

        assert!(
            !result.is_passthrough(),
            "Reasoning Responses models should force translation"
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        let content = output["input"][0]["content"].as_array().unwrap();
        let file_part = &content[1];

        assert_eq!(
            file_part.get("file_url").and_then(Value::as_str),
            Some("https://www.berkshirehathaway.com/letters/2024ltr.pdf")
        );
        assert!(file_part.get("filename").is_none());
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_overrides_model_before_capability_transforms() {
        let payload = json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "Hello"}],
            "top_p": 0.9
        });
        let input = to_bytes(&payload);

        let result =
            transform_request(input, ProviderFormat::Responses, Some("gpt-5-nano")).unwrap();

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(output.get("model").unwrap().as_str().unwrap(), "gpt-5-nano");
        assert!(output.get("top_p").is_none(), "Should not have top_p");
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "google"))]
    fn test_google_top_k_openai_model_drops_top_k_for_chat_completions() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Write a short sentence about API gateways."}]
            }],
            "generationConfig": {
                "topK": 1,
                "maxOutputTokens": 1024
            }
        });
        let input = to_bytes(&payload);

        let result =
            transform_request(input, ProviderFormat::ChatCompletions, Some("gpt-5-nano")).unwrap();

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(result.source_format(), Some(ProviderFormat::Google));
        assert_eq!(
            output.get("model").and_then(Value::as_str),
            Some("gpt-5-nano")
        );
        assert!(
            output.get("top_k").is_none(),
            "OpenAI Chat Completions should not receive top_k"
        );
        assert_eq!(
            output.get("max_completion_tokens").and_then(Value::as_i64),
            Some(1024)
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "google"))]
    fn test_google_top_k_openai_model_drops_top_k_for_responses() {
        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": "Write a short sentence about API gateways."}]
            }],
            "generationConfig": {
                "topK": 1,
                "maxOutputTokens": 1024
            }
        });
        let input = to_bytes(&payload);

        let result =
            transform_request(input, ProviderFormat::Responses, Some("gpt-5-nano")).unwrap();

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(result.source_format(), Some(ProviderFormat::Google));
        assert_eq!(
            output.get("model").and_then(Value::as_str),
            Some("gpt-5-nano")
        );
        assert!(
            output.get("top_k").is_none(),
            "OpenAI Responses should not receive top_k"
        );
        assert_eq!(
            output.get("max_output_tokens").and_then(Value::as_i64),
            Some(1024)
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

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_bedrock_anthropic_model_openai_input() {
        // OpenAI input targeting internal BedrockAnthropic format should produce
        // invoke-ready Anthropic body.
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::BedrockAnthropic,
            Some("us.anthropic.claude-haiku-4-5-20251001-v1:0"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("model").is_none());
        assert_eq!(
            output.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
        assert!(output.get("max_tokens").is_some());
        assert!(output.get("messages").is_some());
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_bedrock_anthropic_system_role_becomes_top_level_system() {
        let payload = json!({
            "model": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Say hello."}
            ],
            "max_tokens": 50
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::BedrockAnthropic,
            Some("us.anthropic.claude-haiku-4-5-20251001-v1:0"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("model").is_none());
        assert_eq!(
            output.get("anthropic_version").and_then(Value::as_str),
            Some("bedrock-2023-05-31")
        );
        assert_eq!(
            output.get("system").and_then(Value::as_str),
            Some("You are a helpful assistant.")
        );
        let messages = output
            .get("messages")
            .and_then(Value::as_array)
            .expect("messages should be an array");
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].get("role").and_then(Value::as_str),
            Some("user")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_bedrock_anthropic_opus_4_8_mid_conversation_system_role_passthrough() {
        let payload = json!({
            "model": "us.anthropic.claude-opus-4-8-v1:0",
            "messages": [
                {"role": "user", "content": "Review this function."},
                {"role": "system", "content": "From now on, include type annotations."}
            ],
            "max_tokens": 50
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::BedrockAnthropic,
            Some("us.anthropic.claude-opus-4-8-v1:0"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("model").is_none());
        assert_eq!(
            output.get("anthropic_version").and_then(Value::as_str),
            Some("bedrock-2023-05-31")
        );
        let messages = output
            .get("messages")
            .and_then(Value::as_array)
            .expect("messages should be an array");
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[1].get("role").and_then(Value::as_str),
            Some("system")
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_bedrock_anthropic_model_anthropic_input() {
        // Anthropic input targeting internal BedrockAnthropic format skips passthrough
        // and emits invoke-ready body.
        let payload = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::BedrockAnthropic,
            Some("us.anthropic.claude-haiku-4-5-20251001-v1:0"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(result.source_format(), Some(ProviderFormat::Anthropic));

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(output.get("model").is_none());
        assert_eq!(
            output.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
        assert_eq!(output.get("max_tokens").unwrap().as_i64().unwrap(), 1024);
        assert!(output.get("messages").is_some());
    }

    // =========================================================================
    // chat_completions_needs_responses_upgrade / format upgrade tests
    // =========================================================================

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_triggers_with_reasoning_effort_and_tools() {
        let payload = json!({
            "reasoning_effort": "medium",
            "tools": [{"type": "function", "function": {"name": "get_weather"}}]
        });
        assert!(chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_triggers_with_low_reasoning_effort() {
        let payload = json!({
            "reasoning_effort": "low",
            "tools": [{"type": "function", "function": {"name": "fn"}}]
        });
        assert!(chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_skips_when_reasoning_effort_none() {
        let payload = json!({
            "reasoning_effort": "none",
            "tools": [{"type": "function", "function": {"name": "fn"}}]
        });
        assert!(!chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_skips_when_no_reasoning_effort() {
        let payload = json!({
            "tools": [{"type": "function", "function": {"name": "fn"}}]
        });
        assert!(!chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_skips_when_no_tools() {
        let payload = json!({ "reasoning_effort": "medium" });
        assert!(!chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_upgrade_detection_skips_when_tools_empty() {
        let payload = json!({ "reasoning_effort": "medium", "tools": [] });
        assert!(!chat_completions_needs_responses_upgrade(&payload));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_upgrades_target_to_responses_for_reasoning_plus_tools() {
        let payload = json!({
            "model": "gpt-5.4-mini",
            "messages": [{"role": "user", "content": "Tokyo weather?"}],
            "reasoning_effort": "medium",
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {
                        "type": "object",
                        "properties": {"location": {"type": "string"}},
                        "required": ["location"]
                    }
                }
            }]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        match result {
            TransformResult::Transformed {
                actual_target_format,
                ..
            } => {
                assert_eq!(
                    actual_target_format,
                    ProviderFormat::Responses,
                    "Should upgrade to Responses when reasoning_effort + tools are present"
                );
            }
            TransformResult::PassThrough(_) => {
                panic!("Expected transformation, got passthrough");
            }
        }
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_does_not_upgrade_gemini_reasoning_plus_tools() {
        let payload = json!({
            "model": "gemini-2.5-flash",
            "messages": [{"role": "user", "content": "Tokyo weather?"}],
            "reasoning_effort": "medium",
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }]
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        match result {
            TransformResult::PassThrough(_) => {}
            TransformResult::Transformed {
                actual_target_format,
                ..
            } => {
                assert_eq!(actual_target_format, ProviderFormat::ChatCompletions);
            }
        }
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_normalizes_prefixed_gemini_chat_completions_model() {
        let payload = json!({
            "model": "models/gemini-2.5-flash",
            "messages": [{"role": "user", "content": "Ping"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::ChatCompletions,
            Some("gemini-2.5-flash"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert_eq!(
            output.get("model").and_then(Value::as_str),
            Some("gemini-2.5-flash")
        );
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_transform_request_does_not_upgrade_without_tools() {
        let payload = json!({
            "model": "gpt-5.4-mini",
            "messages": [{"role": "user", "content": "Hello"}],
            "reasoning_effort": "medium"
        });
        let input = to_bytes(&payload);

        let result = transform_request(input, ProviderFormat::ChatCompletions, None).unwrap();

        match result {
            TransformResult::Transformed {
                actual_target_format,
                ..
            } => {
                assert_eq!(
                    actual_target_format,
                    ProviderFormat::ChatCompletions,
                    "Should not upgrade without tools"
                );
            }
            TransformResult::PassThrough(_) => {
                // PassThrough is also acceptable here (no upgrade, no forced translation)
            }
        }
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "bedrock"))]
    fn test_non_anthropic_bedrock_model_uses_converse() {
        // Non-anthropic bedrock models still go through Converse translation.
        let payload = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let input = to_bytes(&payload);

        let result = transform_request(
            input,
            ProviderFormat::Converse,
            Some("amazon.nova-pro-v1:0"),
        )
        .unwrap();

        assert!(!result.is_passthrough());
        assert_eq!(
            result.source_format(),
            Some(ProviderFormat::ChatCompletions)
        );

        let output: Value = crate::serde_json::from_slice(result.as_bytes()).unwrap();
        assert!(
            output.get("anthropic_version").is_none(),
            "Non-anthropic models should not have anthropic_version"
        );
    }
}
