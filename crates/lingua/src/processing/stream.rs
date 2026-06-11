use bytes::Bytes;
use std::collections::BTreeMap;

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::adapter_for_format;
use crate::processing::transform::{
    serialize_stream_value, transform_stream_chunk_step, TransformError, TransformResult,
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
    allow_full_response_fallback: bool,
    buffered_delta: Option<StreamOutputChunk>,
    buffered_stop: Option<StreamOutputChunk>,
    // Whether the target Anthropic stream has an open message. Some source
    // providers emit repeated metadata chunks; only the first should become
    // Anthropic `message_start`.
    anthropic_message_started: bool,
    // Whether the open Anthropic message has emitted a `tool_use` block. OpenAI
    // Responses may report a generic `stop`, but Anthropic clients need
    // `stop_reason: "tool_use"` once a tool block appeared.
    anthropic_tool_use_started: bool,
    // Anthropic streams require content blocks to be explicitly closed before a
    // new block or the terminal message delta. Some source providers do not have
    // an equivalent event, so the session synthesizes `content_block_stop`.
    anthropic_open_content_block_index: Option<u32>,
    bedrock_tool_call_indexes: BTreeMap<u32, u32>,
    next_bedrock_tool_call_index: u32,
}

impl StreamTransformSession {
    pub fn new(target_format: ProviderFormat) -> Self {
        Self::with_full_response_fallback(target_format, true)
    }

    pub fn with_full_response_fallback(
        target_format: ProviderFormat,
        allow_full_response_fallback: bool,
    ) -> Self {
        Self {
            target_format,
            allow_full_response_fallback,
            buffered_delta: None,
            buffered_stop: None,
            anthropic_message_started: false,
            anthropic_tool_use_started: false,
            anthropic_open_content_block_index: None,
            bedrock_tool_call_indexes: BTreeMap::new(),
            next_bedrock_tool_call_index: 0,
        }
    }

    pub fn target_format(&self) -> ProviderFormat {
        self.target_format
    }

    pub fn push(&mut self, input: Bytes) -> Result<Vec<StreamOutputChunk>, TransformError> {
        let step = transform_stream_chunk_step(
            input,
            self.target_format,
            self.allow_full_response_fallback,
        )?;

        if step.is_passthrough {
            return Ok(vec![StreamOutputChunk {
                data: step.result.into_bytes(),
                event_type: step.event_type,
            }]);
        }

        let result = self.normalize_bedrock_to_openai_stream_result(&step)?;

        let chunks = build_session_chunks(
            result,
            step.source_format,
            self.target_format,
            step.universal.as_ref(),
            step.source_is_native_stream,
            self.anthropic_message_started,
        )?;
        Ok(self.process_chunks(chunks))
    }

    fn normalize_bedrock_to_openai_stream_result(
        &mut self,
        step: &crate::processing::transform::StreamTransformStep,
    ) -> Result<TransformResult, TransformError> {
        if step.source_format != ProviderFormat::Converse
            || self.target_format != ProviderFormat::ChatCompletions
            || !step.source_is_native_stream
        {
            return Ok(step.result.clone());
        }

        let Some(mut universal) = step.universal.clone() else {
            return Ok(step.result.clone());
        };

        self.remap_bedrock_tool_call_indexes(&mut universal);

        let should_reset = universal
            .choices
            .iter()
            .any(|choice| choice.finish_reason.is_some());

        let target_adapter = adapter_for_format(self.target_format)
            .ok_or(TransformError::UnsupportedTargetFormat(self.target_format))?;
        let bytes = serialize_stream_value(&target_adapter.stream_from_universal(&universal)?)?;

        if should_reset {
            self.bedrock_tool_call_indexes.clear();
            self.next_bedrock_tool_call_index = 0;
        }

        Ok(TransformResult::Transformed {
            bytes,
            source_format: step.source_format,
            actual_target_format: self.target_format,
        })
    }

    fn remap_bedrock_tool_call_indexes(&mut self, universal: &mut UniversalStreamChunk) {
        for choice in &mut universal.choices {
            let Some(mut delta) = choice.delta_view() else {
                continue;
            };

            if delta.tool_calls.is_empty() {
                continue;
            }

            for tool_call in &mut delta.tool_calls {
                let Some(bedrock_block_index) = tool_call.index else {
                    continue;
                };
                let openai_tool_index =
                    match self.bedrock_tool_call_indexes.get(&bedrock_block_index) {
                        Some(index) => *index,
                        None => {
                            let index = self.next_bedrock_tool_call_index;
                            self.next_bedrock_tool_call_index += 1;
                            self.bedrock_tool_call_indexes
                                .insert(bedrock_block_index, index);
                            index
                        }
                    };
                tool_call.index = Some(openai_tool_index);
            }

            choice.delta = Some(Value::from(delta));
        }
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
        // Enforce Anthropic event ordering after provider adapters have produced
        // Anthropic-shaped chunks: one message_start, balanced content blocks,
        // tool_use stop reasons, and merged finish/usage message_delta events.
        let is_delta = chunk.event_type.as_deref() == Some("message_delta");
        let is_stop = chunk.event_type.as_deref() == Some("message_stop");
        let is_start = chunk.event_type.as_deref() == Some("message_start");
        let is_content_block_start = chunk.event_type.as_deref() == Some("content_block_start");
        let is_content_block_stop = chunk.event_type.as_deref() == Some("content_block_stop");
        let content_block_start_index = content_block_index(&chunk);
        let is_tool_use_start = is_anthropic_tool_use_start(&chunk);
        let chunk = if is_delta && self.anthropic_tool_use_started {
            with_anthropic_tool_use_stop_reason(chunk)
        } else {
            chunk
        };
        let mut prefix = Vec::new();

        if is_start {
            self.anthropic_message_started = true;
            self.anthropic_tool_use_started = false;
            self.anthropic_open_content_block_index = None;
        }
        if (is_content_block_start || is_delta || is_stop)
            && self.anthropic_open_content_block_index.is_some()
        {
            if let Some(index) = self.anthropic_open_content_block_index.take() {
                prefix.push(anthropic_content_block_stop_chunk(index));
            }
        }
        if is_content_block_start {
            self.anthropic_open_content_block_index = content_block_start_index;
        }
        if is_content_block_stop {
            self.anthropic_open_content_block_index = None;
        }
        if is_tool_use_start {
            self.anthropic_tool_use_started = true;
        }

        if is_delta && self.buffered_delta.is_some() {
            let merged = merge_delta_usage(
                self.buffered_delta
                    .take()
                    .unwrap_or_else(|| StreamOutputChunk::data(Bytes::new())),
                chunk,
            );
            let mut out = prefix;
            out.push(merged);
            if let Some(stop) = self.buffered_stop.take() {
                self.anthropic_message_started = false;
                self.anthropic_tool_use_started = false;
                self.anthropic_open_content_block_index = None;
                out.push(stop);
            }
            return out;
        }

        if is_delta {
            self.buffered_delta = Some(chunk);
            return prefix;
        }

        if is_stop && self.buffered_delta.is_some() {
            self.buffered_stop = Some(chunk);
            return prefix;
        }

        if is_stop {
            self.anthropic_message_started = false;
            self.anthropic_tool_use_started = false;
            self.anthropic_open_content_block_index = None;
        }

        let mut out = prefix;
        out.extend(self.flush_buffered());
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
            self.anthropic_tool_use_started = false;
            self.anthropic_open_content_block_index = None;
            out.push(stop);
        }
        out
    }
}

fn content_block_index(chunk: &StreamOutputChunk) -> Option<u32> {
    let data = crate::serde_json::from_slice::<Value>(&chunk.data).ok()?;
    data.get("index")
        .and_then(Value::as_u64)
        .and_then(|index| u32::try_from(index).ok())
}

fn anthropic_content_block_stop_chunk(index: u32) -> StreamOutputChunk {
    StreamOutputChunk::with_event(
        serialize_stream_value(&crate::serde_json::json!({
            "type": "content_block_stop",
            "index": index
        }))
        .unwrap_or_else(|_| Bytes::new()),
        "content_block_stop".to_string(),
    )
}

fn is_anthropic_tool_use_start(chunk: &StreamOutputChunk) -> bool {
    if chunk.event_type.as_deref() != Some("content_block_start") {
        return false;
    }

    let Ok(data) = crate::serde_json::from_slice::<Value>(&chunk.data) else {
        return false;
    };

    data.get("content_block")
        .and_then(|block| block.get("type"))
        .and_then(Value::as_str)
        == Some("tool_use")
}

fn with_anthropic_tool_use_stop_reason(chunk: StreamOutputChunk) -> StreamOutputChunk {
    let Ok(mut data) = crate::serde_json::from_slice::<Value>(&chunk.data) else {
        return chunk;
    };

    let Some(delta) = data.get_mut("delta").and_then(Value::as_object_mut) else {
        return chunk;
    };
    let Some(stop_reason) = delta.get_mut("stop_reason") else {
        return chunk;
    };
    if !matches!(stop_reason.as_str(), Some("end_turn" | "stop")) {
        return chunk;
    }

    *stop_reason = Value::String("tool_use".to_string());
    StreamOutputChunk {
        data: serialize_stream_value(&data).unwrap_or(chunk.data),
        event_type: chunk.event_type,
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
    source_is_native_stream: bool,
    anthropic_message_started: bool,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    let mut chunks = expand_transform_result(result)?;
    if target_format == ProviderFormat::Anthropic {
        return Ok(expand_anthropic_session_chunks(
            chunks,
            source_format,
            universal,
            source_is_native_stream,
            anthropic_message_started,
        ));
    }
    Ok(std::mem::take(&mut chunks))
}

fn expand_anthropic_session_chunks(
    mut chunks: Vec<StreamOutputChunk>,
    source_format: ProviderFormat,
    universal: Option<&UniversalStreamChunk>,
    source_is_native_stream: bool,
    anthropic_message_started: bool,
) -> Vec<StreamOutputChunk> {
    if source_format == ProviderFormat::Anthropic && source_is_native_stream {
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

    if is_initial_metadata && anthropic_message_started {
        return out;
    }

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
        let content = delta_view
            .as_ref()
            .and_then(|d| d.content.as_deref())
            .filter(|s| !s.is_empty());
        let needs_message_start = !anthropic_message_started
            && (source_format != ProviderFormat::Anthropic || !source_is_native_stream);

        if needs_message_start {
            out.push(StreamOutputChunk::with_event(
                anthropic_message_start_bytes(universal),
                "message_start".to_string(),
            ));
            if content.is_some() {
                out.push(StreamOutputChunk::with_event(
                    Bytes::from_static(
                        br#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
                    ),
                    "content_block_start".to_string(),
                ));
            }
        }

        if let Some(content) = content {
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
            if needs_message_start {
                out.push(StreamOutputChunk::with_event(
                    serialize_stream_value(&crate::serde_json::json!({
                        "type": "content_block_stop",
                        "index": choice.map(|c| c.index).unwrap_or(0),
                    }))
                    .unwrap_or_else(|_| Bytes::new()),
                    "content_block_stop".to_string(),
                ));
            }
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

        let start = session.push(finish_delta).unwrap();
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].event_type.as_deref(), Some("message_start"));

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
        let start = session
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
            .unwrap();
        assert_eq!(start.len(), 1);
        assert_eq!(start[0].event_type.as_deref(), Some("message_start"));

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
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_responses_tool_call_after_metadata_does_not_repeat_message_start() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let created = to_bytes(&json!({
            "type": "response.created",
            "response": {
                "id": "resp_123",
                "model": "gpt-5.5-2026-04-23",
                "usage": {
                    "input_tokens": 0,
                    "output_tokens": 0
                }
            }
        }));
        let in_progress = to_bytes(&json!({
            "type": "response.in_progress",
            "response": {
                "id": "resp_123",
                "model": "gpt-5.5-2026-04-23"
            }
        }));
        let tool_start = to_bytes(&json!({
            "type": "response.output_item.added",
            "output_index": 1,
            "item": {
                "type": "function_call",
                "call_id": "call_123",
                "name": "mcp__braintrust__list_recent_objects"
            }
        }));
        let completed = to_bytes(&json!({
            "type": "response.completed",
            "response": {
                "id": "resp_123",
                "model": "gpt-5.5-2026-04-23",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 20
                }
            }
        }));

        let first = session.push(created).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_start"), Some("content_block_start")]
        );

        let second = session.push(in_progress).unwrap();
        assert!(second.is_empty());

        let third = session.push(tool_start).unwrap();
        assert_eq!(
            third
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_stop"), Some("content_block_start")]
        );

        let tool_block: Value = crate::serde_json::from_slice(&third[1].data).unwrap();
        assert_eq!(
            tool_block
                .get("content_block")
                .and_then(|block| block.get("type"))
                .and_then(Value::as_str),
            Some("tool_use")
        );

        let fourth = session.push(completed).unwrap();
        assert_eq!(
            fourth
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_stop")]
        );

        let final_chunks = session.finish();
        assert_eq!(
            final_chunks
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_delta"), Some("message_stop")]
        );

        let message_delta: Value = crate::serde_json::from_slice(&final_chunks[0].data).unwrap();
        assert_eq!(
            message_delta
                .get("delta")
                .and_then(|delta| delta.get("stop_reason"))
                .and_then(Value::as_str),
            Some("tool_use")
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_final_text_chunk_opens_anthropic_message() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let final_text = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-3.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "There are 100 projects."
                    }]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }));

        let out = session.push(final_text).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta"),
                Some("content_block_stop")
            ]
        );

        let final_chunks = session.finish();
        assert_eq!(
            final_chunks
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_delta"), Some("message_stop")]
        );

        let message_delta: Value = crate::serde_json::from_slice(&final_chunks[0].data).unwrap();
        assert_eq!(
            message_delta
                .get("delta")
                .and_then(|delta| delta.get("stop_reason"))
                .and_then(Value::as_str),
            Some("end_turn")
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_stream_session_expands_full_anthropic_response_for_anthropic_target() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let full_response = to_bytes(&json!({
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
        }));

        let mut out = session.push(full_response).unwrap();
        out.extend(session.finish());

        let event_types = out
            .iter()
            .map(|chunk| chunk.event_type.as_deref())
            .collect::<Vec<_>>();

        assert_eq!(
            event_types,
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta"),
                Some("content_block_stop"),
                Some("message_delta"),
                Some("message_stop"),
            ]
        );

        let delta: Value = crate::serde_json::from_slice(&out[2].data).unwrap();
        assert_eq!(
            delta
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(Value::as_str),
            Some("Hello from Vertex")
        );
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_stream_session_can_disable_full_response_fallback() {
        let mut session = StreamTransformSession::with_full_response_fallback(
            ProviderFormat::ChatCompletions,
            false,
        );
        let full_response = to_bytes(&json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 123,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello from a fake stream"
                },
                "finish_reason": "stop"
            }]
        }));

        assert!(matches!(
            session.push(full_response),
            Err(TransformError::UnableToDetectStreamFormat)
        ));
    }

    #[test]
    #[cfg(feature = "openai")]
    fn test_stream_session_finish_sse_appends_done_marker_for_openai() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);
        let out = session.finish_sse();
        assert_eq!(out, vec![Bytes::from_static(b"data: [DONE]\n\n")]);
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "bedrock"))]
    fn test_stream_session_converts_bedrock_tool_events_to_openai_chunks() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);

        let message_start = to_bytes(&json!({
            "messageStart": {
                "role": "assistant"
            }
        }));
        let message_start_out = session.push(message_start).unwrap();
        assert_eq!(message_start_out.len(), 1);
        let message_start_chunk: Value =
            crate::serde_json::from_slice(&message_start_out[0].data).unwrap();
        assert_eq!(
            message_start_chunk,
            json!({
                "object": "chat.completion.chunk",
                "choices": []
            })
        );

        let tool_start = to_bytes(&json!({
            "contentBlockStart": {
                "contentBlockIndex": 0,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_123",
                        "name": "list_campaigns",
                        "type": "tool_use"
                    }
                }
            }
        }));
        let start_out = session.push(tool_start).unwrap();
        assert_eq!(start_out.len(), 1);
        let start_chunk: Value = crate::serde_json::from_slice(&start_out[0].data).unwrap();
        assert_eq!(
            start_chunk,
            json!({
                "object": "chat.completion.chunk",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "index": 0,
                            "id": "tooluse_123",
                            "type": "function",
                            "function": {
                                "name": "list_campaigns",
                                "arguments": ""
                            }
                        }]
                    },
                    "finish_reason": null
                }]
            })
        );

        let tool_args = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "toolUse": {
                        "input": "{\"campaign"
                    }
                }
            }
        }));
        let args_out = session.push(tool_args).unwrap();
        assert_eq!(args_out.len(), 1);
        let args_chunk: Value = crate::serde_json::from_slice(&args_out[0].data).unwrap();
        assert_eq!(
            args_chunk,
            json!({
                "object": "chat.completion.chunk",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "content": null,
                        "tool_calls": [{
                            "index": 0,
                            "function": {
                                "arguments": "{\"campaign"
                            }
                        }]
                    },
                    "finish_reason": null
                }]
            })
        );

        let stop = to_bytes(&json!({
            "messageStop": {
                "stopReason": "tool_use"
            }
        }));
        let stop_out = session.push(stop).unwrap();
        assert_eq!(stop_out.len(), 1);
        let stop_chunk: Value = crate::serde_json::from_slice(&stop_out[0].data).unwrap();
        assert_eq!(
            stop_chunk,
            json!({
                "object": "chat.completion.chunk",
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "tool_calls"
                }]
            })
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "bedrock"))]
    fn test_stream_session_keeps_single_choice_index_for_bedrock_reasoning_and_text_blocks() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);

        let reasoning = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "reasoningContent": {
                        "text": "Thinking"
                    }
                }
            }
        }));
        let reasoning_out = session.push(reasoning).unwrap();
        assert_eq!(reasoning_out.len(), 1);
        let reasoning_chunk: Value = crate::serde_json::from_slice(&reasoning_out[0].data).unwrap();
        assert_eq!(
            reasoning_chunk
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("index"))
                .and_then(Value::as_u64),
            Some(0)
        );

        let text = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 1,
                "delta": {
                    "text": "Final answer"
                }
            }
        }));
        let text_out = session.push(text).unwrap();
        assert_eq!(text_out.len(), 1);
        let text_chunk: Value = crate::serde_json::from_slice(&text_out[0].data).unwrap();
        let text_choice = text_chunk
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .unwrap();

        assert_eq!(text_choice.get("index").and_then(Value::as_u64), Some(0));
        assert_eq!(
            text_choice
                .get("delta")
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str),
            Some("Final answer")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "bedrock"))]
    fn test_stream_session_maps_bedrock_tool_block_to_sequential_openai_tool_index() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);

        let reasoning = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 0,
                "delta": {
                    "reasoningContent": {
                        "text": "Thinking"
                    }
                }
            }
        }));
        assert_eq!(session.push(reasoning).unwrap().len(), 1);

        let tool_start = to_bytes(&json!({
            "contentBlockStart": {
                "contentBlockIndex": 1,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_123",
                        "name": "list_campaigns"
                    }
                }
            }
        }));
        let start_out = session.push(tool_start).unwrap();
        let start_chunk: Value = crate::serde_json::from_slice(&start_out[0].data).unwrap();
        assert_eq!(
            start_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );

        let tool_args = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 1,
                "delta": {
                    "toolUse": {
                        "input": "{\"campaign"
                    }
                }
            }
        }));
        let args_out = session.push(tool_args).unwrap();
        let args_chunk: Value = crate::serde_json::from_slice(&args_out[0].data).unwrap();
        assert_eq!(
            args_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );
        assert_eq!(
            args_chunk["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"],
            json!("{\"campaign")
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "bedrock"))]
    fn test_stream_session_maps_parallel_bedrock_tool_blocks_to_sequential_openai_tool_indexes() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);

        let first_tool_start = to_bytes(&json!({
            "contentBlockStart": {
                "contentBlockIndex": 2,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_2",
                        "name": "first_tool"
                    }
                }
            }
        }));
        let first_out = session.push(first_tool_start).unwrap();
        let first_chunk: Value = crate::serde_json::from_slice(&first_out[0].data).unwrap();
        assert_eq!(
            first_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );

        let second_tool_start = to_bytes(&json!({
            "contentBlockStart": {
                "contentBlockIndex": 4,
                "start": {
                    "toolUse": {
                        "toolUseId": "tooluse_4",
                        "name": "second_tool"
                    }
                }
            }
        }));
        let second_out = session.push(second_tool_start).unwrap();
        let second_chunk: Value = crate::serde_json::from_slice(&second_out[0].data).unwrap();
        assert_eq!(
            second_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(1)
        );

        let first_args = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 2,
                "delta": {
                    "toolUse": {
                        "input": "{\"first\":"
                    }
                }
            }
        }));
        let first_args_out = session.push(first_args).unwrap();
        let first_args_chunk: Value =
            crate::serde_json::from_slice(&first_args_out[0].data).unwrap();
        assert_eq!(
            first_args_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );

        let second_args = to_bytes(&json!({
            "contentBlockDelta": {
                "contentBlockIndex": 4,
                "delta": {
                    "toolUse": {
                        "input": "{\"second\":"
                    }
                }
            }
        }));
        let second_args_out = session.push(second_args).unwrap();
        let second_args_chunk: Value =
            crate::serde_json::from_slice(&second_args_out[0].data).unwrap();
        assert_eq!(
            second_args_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(1)
        );
    }
}
