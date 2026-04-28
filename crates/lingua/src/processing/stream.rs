use bytes::Bytes;

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::adapter_for_format;
use crate::processing::transform::{
    serialize_stream_value, transform_stream_chunk_step, TransformError,
};
use crate::serde_json::Value;
use crate::universal::UniversalStreamChunk;

static EMPTY_JSON: Bytes = Bytes::from_static(b"{}");
static SSE_DATA_PREFIX: &[u8] = b"data: ";
static SSE_EVENT_PREFIX: &[u8] = b"event: ";
static SSE_EVENT_SUFFIX: &[u8] = b"\n\n";
static SSE_DONE_MARKER_BYTES: Bytes = Bytes::from_static(b"data: [DONE]\n\n");
static SSE_COMMENT_BYTES: Bytes = Bytes::from_static(b":\n\n");
static KEEP_ALIVE_BYTES: &[u8] = b"{\"_keep_alive\":true}";

/// A single provider-formatted stream event emitted by a stream transform session.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamOutputChunk {
    /// Serialized JSON payload for a single provider event.
    pub data: Bytes,
    /// Optional SSE `event:` label for this output chunk.
    ///
    /// For passthrough chunks this usually matches the source provider event name.
    /// For transformed chunks it is the target-provider event name to emit.
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

    pub fn target_format(&self) -> ProviderFormat {
        self.target_format
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
        // Anthropic currently needs stateful post-processing after universal -> provider
        // conversion: some inputs expand into multiple SSE events, and finish/usage data
        // may arrive as adjacent chunks that must be merged before emission. The adapter
        // boundary is still single-chunk and stateless, so that sequencing lives here.
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

fn expand_transform_result(
    result: crate::processing::transform::TransformResult,
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

fn build_session_chunks(
    result: crate::processing::transform::TransformResult,
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
        None => (true, false),
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
        None => EMPTY_JSON.clone(),
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
}
