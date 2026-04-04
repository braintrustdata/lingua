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
static SSE_DATA_PREFIX: &[u8] = b"data: ";
static SSE_EVENT_PREFIX: &[u8] = b"event: ";
static SSE_EVENT_SUFFIX: &[u8] = b"\n\n";
static SSE_DONE_MARKER_BYTES: Bytes = Bytes::from_static(b"data: [DONE]\n\n");
static SSE_COMMENT_BYTES: Bytes = Bytes::from_static(b":\n\n");
static KEEP_ALIVE_BYTES: &[u8] = b"{\"_keep_alive\":true}";

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

/// A single provider-formatted stream event emitted by a stream transform session.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamOutputChunk {
    /// Serialized JSON payload for a single provider event.
    pub data: Bytes,
    /// Optional provider event type used by SSE serializers such as Anthropic's.
    pub event_type: Option<String>,
}

impl StreamOutputChunk {
    pub fn data(data: Bytes) -> Self {
        Self {
            data,
            event_type: None,
        }
    }

    pub fn with_event(data: Bytes, event_type: String) -> Self {
        Self {
            data,
            event_type: Some(event_type),
        }
    }
}

/// Stateful stream transformation session.
///
/// This wraps the stateless `transform_stream_chunk` API with target-provider
/// sequencing rules such as Anthropic's finish/usage message_delta merge.
#[derive(Debug)]
pub struct StreamTransformSession {
    target_format: ProviderFormat,
    buffered_delta: Option<StreamOutputChunk>,
    buffered_stop: Option<StreamOutputChunk>,
    anthropic_message_started: bool,
}

impl StreamTransformSession {
    pub fn new(target_format: ProviderFormat) -> Self {
        Self {
            target_format,
            buffered_delta: None,
            buffered_stop: None,
            anthropic_message_started: false,
        }
    }

    pub fn push(&mut self, input: Bytes) -> Result<Vec<StreamOutputChunk>, TransformError> {
        let step = transform_stream_chunk_step(input, self.target_format)?;

        if step.is_passthrough {
            return Ok(vec![StreamOutputChunk {
                data: step.result.into_bytes(),
                event_type: step.event_type,
            }]);
        }

        let chunks = build_session_chunks(
            step.result,
            step.source_format,
            self.target_format,
            step.universal.as_ref(),
            self.anthropic_message_started,
        )?;
        Ok(self.process_chunks(chunks))
    }

    pub fn finish(&mut self) -> Vec<StreamOutputChunk> {
        self.flush_buffered()
    }

    pub fn push_sse(&mut self, input: Bytes) -> Result<Vec<Bytes>, TransformError> {
        let chunks = self.push(input)?;
        Ok(chunks
            .iter()
            .map(|chunk| self.format_output_chunk_as_sse(chunk))
            .collect())
    }

    pub fn finish_sse(&mut self) -> Vec<Bytes> {
        let mut out: Vec<Bytes> = self
            .finish()
            .iter()
            .map(|chunk| self.format_output_chunk_as_sse(chunk))
            .collect();
        if let Some(done_marker) = self.done_marker_sse() {
            out.push(done_marker);
        }
        out
    }

    pub fn format_output_chunk_as_sse(&self, chunk: &StreamOutputChunk) -> Bytes {
        format_stream_chunk_as_sse(chunk, self.target_format)
    }

    pub fn done_marker_sse(&self) -> Option<Bytes> {
        sse_done_marker(self.target_format)
    }

    fn process_chunks(&mut self, chunks: Vec<StreamOutputChunk>) -> Vec<StreamOutputChunk> {
        if self.target_format != ProviderFormat::Anthropic {
            return chunks;
        }

        let mut out = Vec::new();
        for chunk in chunks {
            out.extend(self.process_anthropic_chunk(chunk));
        }
        out
    }

    fn process_anthropic_chunk(&mut self, chunk: StreamOutputChunk) -> Vec<StreamOutputChunk> {
        let is_delta = chunk.event_type.as_deref() == Some("message_delta");
        let is_stop = chunk.event_type.as_deref() == Some("message_stop");
        let is_start = chunk.event_type.as_deref() == Some("message_start");

        if is_start {
            self.anthropic_message_started = true;
        }

        if is_delta && self.buffered_delta.is_some() {
            let merged = merge_delta_usage(
                self.buffered_delta
                    .take()
                    .unwrap_or_else(|| StreamOutputChunk::data(Bytes::new())),
                chunk,
            );
            let mut out = vec![merged];
            if let Some(stop) = self.buffered_stop.take() {
                out.push(stop);
            }
            return out;
        }

        if is_delta {
            self.buffered_delta = Some(chunk);
            return Vec::new();
        }

        if is_stop && self.buffered_delta.is_some() {
            self.buffered_stop = Some(chunk);
            return Vec::new();
        }

        if is_stop {
            self.anthropic_message_started = false;
        }

        let mut out = self.flush_buffered();
        out.push(chunk);
        out
    }

    fn flush_buffered(&mut self) -> Vec<StreamOutputChunk> {
        let mut out = Vec::new();
        if let Some(delta) = self.buffered_delta.take() {
            out.push(delta);
        }
        if let Some(stop) = self.buffered_stop.take() {
            self.anthropic_message_started = false;
            out.push(stop);
        }
        out
    }
}

struct StreamTransformStep {
    result: TransformResult,
    source_format: ProviderFormat,
    universal: Option<UniversalStreamChunk>,
    event_type: Option<String>,
    is_passthrough: bool,
}

pub(crate) fn format_stream_chunk_as_sse(
    chunk: &StreamOutputChunk,
    format: ProviderFormat,
) -> Bytes {
    if chunk.data.as_ref() == KEEP_ALIVE_BYTES {
        return SSE_COMMENT_BYTES.clone();
    }

    let event_type = if needs_sse_event_lines(format) {
        chunk.event_type.as_deref()
    } else {
        None
    };

    let event_line_len = event_type
        .map(|t| SSE_EVENT_PREFIX.len() + t.len() + 1)
        .unwrap_or(0);
    let mut buf = Vec::with_capacity(
        event_line_len + SSE_DATA_PREFIX.len() + chunk.data.len() + SSE_EVENT_SUFFIX.len(),
    );
    if let Some(et) = event_type {
        buf.extend_from_slice(SSE_EVENT_PREFIX);
        buf.extend_from_slice(et.as_bytes());
        buf.extend_from_slice(b"\n");
    }
    buf.extend_from_slice(SSE_DATA_PREFIX);
    buf.extend_from_slice(&chunk.data);
    buf.extend_from_slice(SSE_EVENT_SUFFIX);
    Bytes::from(buf)
}

pub(crate) fn sse_done_marker(format: ProviderFormat) -> Option<Bytes> {
    if needs_done_marker(format) {
        Some(SSE_DONE_MARKER_BYTES.clone())
    } else {
        None
    }
}

fn needs_sse_event_lines(format: ProviderFormat) -> bool {
    matches!(
        format,
        ProviderFormat::Anthropic | ProviderFormat::Responses
    )
}

fn needs_done_marker(format: ProviderFormat) -> bool {
    matches!(format, ProviderFormat::ChatCompletions)
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
    Ok(transform_stream_chunk_step(input, target_format)?.result)
}

fn expand_transform_result(
    result: TransformResult,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    let bytes = result.into_bytes();
    if bytes.as_ref() == b"{}" {
        return Ok(vec![]);
    }
    expand_stream_bytes(bytes)
}

fn expand_stream_bytes(bytes: Bytes) -> Result<Vec<StreamOutputChunk>, TransformError> {
    if bytes.first() == Some(&b'[') {
        let arr = crate::serde_json::from_slice::<Vec<Value>>(&bytes)
            .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;
        let mut chunks = Vec::new();
        for value in arr {
            let serialized = crate::serde_json::to_vec(&value)
                .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;
            if serialized == b"{}" {
                continue;
            }
            chunks.push(StreamOutputChunk {
                data: Bytes::from(serialized),
                event_type: extract_event_type(&value),
            });
        }
        return Ok(chunks);
    }

    let value = crate::serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;
    Ok(vec![StreamOutputChunk {
        data: bytes,
        event_type: extract_event_type(&value),
    }])
}

fn extract_event_type(value: &Value) -> Option<String> {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn serialize_stream_value(value: &Value) -> Result<Bytes, TransformError> {
    Ok(Bytes::from(crate::serde_json::to_vec(value).map_err(
        |e| TransformError::SerializationFailed(e.to_string()),
    )?))
}

fn transform_stream_chunk_step(
    input: Bytes,
    target_format: ProviderFormat,
) -> Result<StreamTransformStep, TransformError> {
    let chunk: Value = crate::serde_json::from_slice(&input)
        .map_err(|e| TransformError::DeserializationFailed(e.to_string()))?;
    let event_type = extract_event_type(&chunk);

    let source_adapter = detect_adapter(&chunk, DetectKind::Stream)?;
    let source_format = source_adapter.format();
    let universal = source_adapter.stream_to_universal(chunk)?;

    if source_format == target_format {
        return Ok(StreamTransformStep {
            result: TransformResult::PassThrough(input),
            source_format,
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
        },
        source_format,
        universal,
        event_type: None,
        is_passthrough: false,
    })
}

fn build_session_chunks(
    result: TransformResult,
    source_format: ProviderFormat,
    target_format: ProviderFormat,
    universal: Option<&UniversalStreamChunk>,
    anthropic_message_started: bool,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    let mut chunks = expand_transform_result(result)?;
    if target_format == ProviderFormat::Anthropic {
        return Ok(expand_anthropic_session_chunks(
            chunks,
            source_format,
            universal,
            anthropic_message_started,
        ));
    }
    Ok(std::mem::take(&mut chunks))
}

fn expand_anthropic_session_chunks(
    mut chunks: Vec<StreamOutputChunk>,
    source_format: ProviderFormat,
    universal: Option<&UniversalStreamChunk>,
    anthropic_message_started: bool,
) -> Vec<StreamOutputChunk> {
    if source_format == ProviderFormat::Anthropic {
        return chunks;
    }

    let Some(universal) = universal else {
        return chunks;
    };

    let has_finish = universal
        .choices
        .first()
        .and_then(|c| c.finish_reason.as_ref())
        .is_some();
    let has_metadata =
        universal.model.is_some() || universal.id.is_some() || universal.usage.is_some();
    let choice = universal.choices.first();
    let delta_view = choice.and_then(|c| c.delta_view());
    let has_tool_calls = delta_view
        .as_ref()
        .is_some_and(|d| !d.tool_calls.is_empty());
    let is_initial_tool_call = delta_view
        .as_ref()
        .and_then(|d| d.tool_calls.first())
        .is_some_and(|tc| {
            tc.id.is_some()
                || tc
                    .function
                    .as_ref()
                    .and_then(|f| f.name.as_deref())
                    .is_some()
        });
    let is_initial_metadata = has_metadata
        && !has_finish
        && !has_tool_calls
        && !universal.choices.is_empty()
        && delta_view
            .as_ref()
            .is_none_or(|d| d.content.as_deref().is_none_or(str::is_empty));

    let mut out = Vec::new();

    if is_initial_metadata && !anthropic_message_started {
        if let Some(message_start) = chunks.first() {
            out.push(message_start.clone());
        }

        if let Some(choice) = choice {
            if let Some(delta_view) = delta_view.as_ref() {
                if !delta_view.reasoning.is_empty() {
                    let thinking = delta_view
                        .reasoning
                        .iter()
                        .filter_map(|r| r.content.as_deref())
                        .collect::<String>();
                    if !thinking.is_empty() {
                        out.push(StreamOutputChunk::with_event(
                            Bytes::from_static(
                                br#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
                            ),
                            "content_block_start".to_string(),
                        ));
                        out.push(StreamOutputChunk::with_event(
                            serialize_stream_value(&crate::serde_json::json!({
                                "type": "content_block_delta",
                                "index": choice.index,
                                "delta": {
                                    "type": "thinking_delta",
                                    "thinking": thinking
                                }
                            }))
                            .unwrap_or_else(|_| Bytes::new()),
                            "content_block_delta".to_string(),
                        ));
                        return out;
                    }
                }
            }
        }

        out.push(StreamOutputChunk::with_event(
            Bytes::from_static(
                br#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            ),
            "content_block_start".to_string(),
        ));
        return out;
    }

    if has_finish {
        if let Some(content) = delta_view
            .as_ref()
            .and_then(|d| d.content.as_deref())
            .filter(|s| !s.is_empty())
        {
            out.push(StreamOutputChunk::with_event(
                serialize_stream_value(&crate::serde_json::json!({
                    "type": "content_block_delta",
                    "index": choice.map(|c| c.index).unwrap_or(0),
                    "delta": {
                        "type": "text_delta",
                        "text": content
                    }
                }))
                .unwrap_or_else(|_| Bytes::new()),
                "content_block_delta".to_string(),
            ));
        }
        out.append(&mut chunks);
        out.push(StreamOutputChunk::with_event(
            Bytes::from_static(br#"{"type":"message_stop"}"#),
            "message_stop".to_string(),
        ));
        return out;
    }

    if has_metadata && has_tool_calls && is_initial_tool_call && !anthropic_message_started {
        out.push(StreamOutputChunk::with_event(
            anthropic_message_start_bytes(universal),
            "message_start".to_string(),
        ));
        out.append(&mut chunks);
        return out;
    }

    chunks
}

fn anthropic_message_start_bytes(chunk: &UniversalStreamChunk) -> Bytes {
    serialize_stream_value(&crate::serde_json::json!({
        "type": "message_start",
        "message": {
            "id": chunk
                .id
                .clone()
                .unwrap_or_else(|| "msg_placeholder_id".to_string()),
            "type": "message",
            "role": "assistant",
            "model": chunk.model.as_deref().unwrap_or("claude-3-5-sonnet"),
            "content": [],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": match &chunk.usage {
                Some(usage) => usage.to_provider_value(ProviderFormat::Anthropic),
                None => crate::serde_json::json!({
                    "input_tokens": 0,
                    "output_tokens": 0
                }),
            }
        }
    }))
    .unwrap_or_else(|_| Bytes::new())
}

fn merge_delta_usage(
    finish_delta: StreamOutputChunk,
    usage_delta: StreamOutputChunk,
) -> StreamOutputChunk {
    let event_type = finish_delta.event_type.clone();
    let merged = (|| -> Option<StreamOutputChunk> {
        let mut finish: Value = crate::serde_json::from_slice(&finish_delta.data).ok()?;
        let usage_val: Value = crate::serde_json::from_slice(&usage_delta.data).ok()?;

        if let Some(usage) = usage_val.get("usage") {
            if let Some(obj) = finish.as_object_mut() {
                obj.insert("usage".into(), usage.clone());
            }
        }

        let serialized = crate::serde_json::to_vec(&finish).ok()?;
        Some(StreamOutputChunk {
            data: Bytes::from(serialized),
            event_type,
        })
    })();

    merged.unwrap_or(finish_delta)
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
            serialize_stream_value(&target_adapter.stream_from_universal(universal)?)?
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
    #[cfg(feature = "anthropic")]
    fn test_stream_session_merges_anthropic_finish_and_usage_deltas() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let finish_delta = to_bytes(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        }));
        let usage_delta = to_bytes(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "gpt-4",
            "choices": [],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 42,
                "total_tokens": 52
            }
        }));

        assert!(session.push(finish_delta).unwrap().is_empty());
        let out = session.push(usage_delta).unwrap();

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].event_type.as_deref(), Some("message_delta"));
        assert_eq!(out[1].event_type.as_deref(), Some("message_stop"));

        let merged: Value = crate::serde_json::from_slice(&out[0].data).unwrap();
        assert_eq!(
            merged
                .get("delta")
                .and_then(|d| d.get("stop_reason"))
                .and_then(Value::as_str),
            Some("end_turn")
        );
        assert_eq!(
            merged
                .get("usage")
                .and_then(|u| u.get("output_tokens"))
                .and_then(Value::as_i64),
            Some(42)
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_stream_session_flushes_buffered_anthropic_events_on_finish() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        assert!(session
            .push(to_bytes(&json!({
                "id": "chatcmpl-123",
                "object": "chat.completion.chunk",
                "created": 123,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            })))
            .unwrap()
            .is_empty());

        let out = session.finish();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].event_type.as_deref(), Some("message_delta"));
        assert_eq!(out[1].event_type.as_deref(), Some("message_stop"));
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_expands_multi_event_transform_output() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let openai_chunk = to_bytes(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": "" },
                "finish_reason": null
            }],
            "usage": null
        }));

        let out = session.push(openai_chunk).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].event_type.as_deref(), Some("message_start"));
        assert_eq!(out[1].event_type.as_deref(), Some("content_block_start"));
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_tool_call_continuations_do_not_repeat_message_start() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let tool_start = to_bytes(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": ""
                        }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let tool_args = to_bytes(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {
                            "arguments": "{\""
                        }
                    }]
                },
                "finish_reason": null
            }]
        }));

        let first = session.push(tool_start).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].event_type.as_deref(), Some("message_start"));
        assert_eq!(first[1].event_type.as_deref(), Some("content_block_start"));

        let second = session.push(tool_args).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].event_type.as_deref(), Some("content_block_delta"));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_stream_session_finish_sse_appends_done_marker_for_openai() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);
        let out = session.finish_sse();
        assert_eq!(out, vec![Bytes::from_static(b"data: [DONE]\n\n")]);
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
