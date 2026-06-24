use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::capabilities::ProviderFormat;
use crate::processing::adapters::adapter_for_format;
use crate::processing::transform::{
    serialize_stream_value, transform_stream_chunk_step, TransformError, TransformResult,
};
#[cfg(feature = "openai")]
use crate::providers::openai::responses_adapter::{
    responses_created_stream_event_from_universal,
    responses_stream_events_from_universal_with_output_index_offset,
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

struct SessionChunkState<'a> {
    anthropic_message_started: bool,
    responses_message_started: bool,
    responses_output_index_states: &'a mut BTreeMap<u32, ResponsesOutputIndexState>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ResponsesOutputIndexState {
    text_output_index: Option<u32>,
    tool_output_index_offset: u32,
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
    anthropic_open_content_block_kind: Option<AnthropicContentBlockKind>,
    anthropic_next_content_block_index: u32,
    anthropic_content_block_index_map: BTreeMap<(AnthropicContentBlockKind, u32), u32>,
    responses_message_started: bool,
    responses_output_index_states: BTreeMap<u32, ResponsesOutputIndexState>,
    responses_tool_call_indexes: BTreeMap<u32, u32>,
    next_responses_tool_call_index: u32,
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
            anthropic_open_content_block_kind: None,
            anthropic_next_content_block_index: 0,
            anthropic_content_block_index_map: BTreeMap::new(),
            responses_message_started: false,
            responses_output_index_states: BTreeMap::new(),
            responses_tool_call_indexes: BTreeMap::new(),
            next_responses_tool_call_index: 0,
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

        let result = self.normalize_source_stream_result(&step)?;

        let chunks = build_session_chunks(
            result,
            step.source_format,
            self.target_format,
            step.universal.as_ref(),
            step.source_is_native_stream,
            SessionChunkState {
                anthropic_message_started: self.anthropic_message_started,
                responses_message_started: self.responses_message_started,
                responses_output_index_states: &mut self.responses_output_index_states,
            },
        )?;
        self.process_chunks(chunks)
    }

    fn normalize_source_stream_result(
        &mut self,
        step: &crate::processing::transform::StreamTransformStep,
    ) -> Result<TransformResult, TransformError> {
        if step.source_format == ProviderFormat::Converse
            && self.target_format == ProviderFormat::ChatCompletions
            && step.source_is_native_stream
        {
            return self.normalize_bedrock_to_openai_stream_result(step);
        }

        if step.source_format == ProviderFormat::Responses
            && self.target_format != ProviderFormat::Responses
            && step.source_is_native_stream
        {
            return self.normalize_responses_tool_call_indexes_stream_result(step);
        }

        Ok(step.result.clone())
    }

    fn normalize_bedrock_to_openai_stream_result(
        &mut self,
        step: &crate::processing::transform::StreamTransformStep,
    ) -> Result<TransformResult, TransformError> {
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

    fn normalize_responses_tool_call_indexes_stream_result(
        &mut self,
        step: &crate::processing::transform::StreamTransformStep,
    ) -> Result<TransformResult, TransformError> {
        let Some(mut universal) = step.universal.clone() else {
            return Ok(step.result.clone());
        };

        self.remap_responses_tool_call_indexes(&mut universal);

        let should_reset = universal
            .choices
            .iter()
            .any(|choice| choice.finish_reason.is_some());

        let target_adapter = adapter_for_format(self.target_format)
            .ok_or(TransformError::UnsupportedTargetFormat(self.target_format))?;
        let bytes = serialize_stream_value(&target_adapter.stream_from_universal(&universal)?)?;

        if should_reset {
            self.responses_tool_call_indexes.clear();
            self.next_responses_tool_call_index = 0;
        }

        Ok(TransformResult::Transformed {
            bytes,
            source_format: step.source_format,
            actual_target_format: self.target_format,
        })
    }

    fn remap_responses_tool_call_indexes(&mut self, universal: &mut UniversalStreamChunk) {
        for choice in &mut universal.choices {
            let Some(mut delta) = choice.delta_view() else {
                continue;
            };

            if delta.tool_calls.is_empty() {
                continue;
            }

            for tool_call in &mut delta.tool_calls {
                let Some(responses_output_index) = tool_call.index else {
                    continue;
                };
                let tool_index = match self
                    .responses_tool_call_indexes
                    .get(&responses_output_index)
                {
                    Some(index) => *index,
                    None => {
                        let index = self.next_responses_tool_call_index;
                        self.next_responses_tool_call_index += 1;
                        self.responses_tool_call_indexes
                            .insert(responses_output_index, index);
                        index
                    }
                };
                tool_call.index = Some(tool_index);
            }

            choice.delta = Some(Value::from(delta));
        }
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

    fn process_chunks(
        &mut self,
        chunks: Vec<StreamOutputChunk>,
    ) -> Result<Vec<StreamOutputChunk>, TransformError> {
        if self.target_format == ProviderFormat::Responses {
            for chunk in &chunks {
                match chunk.event_type.as_deref() {
                    Some("response.created") => self.responses_message_started = true,
                    Some("response.completed") | Some("response.incomplete") => {
                        self.responses_message_started = false;
                    }
                    _ => {}
                }
            }
            return Ok(chunks);
        }

        // Anthropic currently needs stateful post-processing after universal -> provider
        // conversion: some inputs expand into multiple SSE events, and finish/usage data
        // may arrive as adjacent chunks that must be merged before emission. The adapter
        // boundary is still single-chunk and stateless, so that sequencing lives here.
        if self.target_format != ProviderFormat::Anthropic {
            return Ok(chunks);
        }

        let mut out = Vec::new();
        for chunk in chunks {
            out.extend(self.process_anthropic_chunk(chunk)?);
        }
        Ok(out)
    }

    fn process_anthropic_chunk(
        &mut self,
        chunk: StreamOutputChunk,
    ) -> Result<Vec<StreamOutputChunk>, TransformError> {
        // Enforce Anthropic event ordering after provider adapters have produced
        // Anthropic-shaped chunks: one message_start, balanced content blocks,
        // tool_use stop reasons, and merged finish/usage message_delta events.
        let is_message_delta = chunk.event_type.as_deref() == Some("message_delta");
        let is_stop = chunk.event_type.as_deref() == Some("message_stop");
        let is_start = chunk.event_type.as_deref() == Some("message_start");
        let is_content_block_start = chunk.event_type.as_deref() == Some("content_block_start");
        let is_content_block_stop = chunk.event_type.as_deref() == Some("content_block_stop");
        let content_block_start = parse_anthropic_content_block_start_event(&chunk)?;
        let mut content_block_start_index = content_block_start.as_ref().map(|event| event.index);
        let content_block_start_kind = content_block_start
            .as_ref()
            .and_then(AnthropicContentBlockStartEventView::block_kind);
        let content_block_delta = parse_anthropic_content_block_delta_event(&chunk)?;
        let content_block_delta_kind = content_block_delta
            .as_ref()
            .and_then(AnthropicContentBlockDeltaEventView::block_kind);
        let content_block_delta_index = content_block_delta.as_ref().map(|event| event.index);
        let is_tool_use_start = content_block_start
            .as_ref()
            .is_some_and(AnthropicContentBlockStartEventView::is_tool_use_start);
        let chunk = if is_message_delta && self.anthropic_tool_use_started {
            with_anthropic_tool_use_stop_reason(chunk)
        } else {
            chunk
        };
        let mut prefix = Vec::new();

        if is_start {
            self.anthropic_message_started = true;
            self.anthropic_tool_use_started = false;
            self.anthropic_open_content_block_index = None;
            self.anthropic_open_content_block_kind = None;
            self.anthropic_next_content_block_index = 0;
            self.anthropic_content_block_index_map.clear();
        }
        let mut chunk = chunk;
        let source_content_block_start_index = content_block_start_index;
        if let (Some(source_index), Some(start_kind)) =
            (source_content_block_start_index, content_block_start_kind)
        {
            if let Some(mapped_index) = self
                .anthropic_content_block_index_map
                .get(&(start_kind, source_index))
                .copied()
            {
                if mapped_index != source_index {
                    chunk = with_anthropic_content_block_start_index(chunk, mapped_index);
                    content_block_start_index = Some(mapped_index);
                }
            } else {
                let open_index = self.anthropic_open_content_block_index;
                let needs_fresh_index = source_index < self.anthropic_next_content_block_index
                    || open_index == Some(source_index)
                    || self
                        .anthropic_open_content_block_kind
                        .is_some_and(|open_kind| open_kind != start_kind);
                if needs_fresh_index {
                    let new_index = self
                        .anthropic_next_content_block_index
                        .max(open_index.map(|index| index + 1).unwrap_or(0));
                    self.anthropic_next_content_block_index = new_index + 1;
                    chunk = with_anthropic_content_block_start_index(chunk, new_index);
                    content_block_start_index = Some(new_index);
                }
            }
        }
        if let (Some(open_index), Some(open_kind), Some(delta_index), Some(delta_kind)) = (
            self.anthropic_open_content_block_index,
            self.anthropic_open_content_block_kind,
            content_block_delta_index,
            content_block_delta_kind,
        ) {
            if open_kind != delta_kind && delta_kind.can_synthesize_start() {
                let new_index = self.anthropic_next_content_block_index.max(open_index + 1);
                self.anthropic_next_content_block_index = new_index + 1;
                chunk = with_anthropic_content_block_delta_index(chunk, new_index);
                self.anthropic_open_content_block_index = Some(new_index);
                self.anthropic_open_content_block_kind = Some(delta_kind);
                self.anthropic_content_block_index_map
                    .insert((delta_kind, delta_index), new_index);
                prefix.push(anthropic_content_block_stop_chunk(open_index)?);
                prefix.push(anthropic_content_block_start_chunk(new_index, delta_kind));
            } else if open_kind == delta_kind {
                if let Some(mapped_index) = self
                    .anthropic_content_block_index_map
                    .get(&(delta_kind, delta_index))
                    .copied()
                    .filter(|mapped_index| *mapped_index != delta_index)
                {
                    chunk = with_anthropic_content_block_delta_index(chunk, mapped_index);
                }
            }
        }
        if self.anthropic_open_content_block_index.is_none()
            && self.anthropic_message_started
            && !is_content_block_start
        {
            if let (Some(delta_index), Some(delta_kind)) =
                (content_block_delta_index, content_block_delta_kind)
            {
                if delta_kind.can_synthesize_start() {
                    let target_index = self
                        .anthropic_content_block_index_map
                        .get(&(delta_kind, delta_index))
                        .copied()
                        .unwrap_or_else(|| {
                            let index = delta_index.max(self.anthropic_next_content_block_index);
                            self.anthropic_content_block_index_map
                                .insert((delta_kind, delta_index), index);
                            self.anthropic_next_content_block_index = index + 1;
                            index
                        });
                    if target_index != delta_index {
                        chunk = with_anthropic_content_block_delta_index(chunk, target_index);
                    }
                    self.anthropic_open_content_block_index = Some(target_index);
                    self.anthropic_open_content_block_kind = Some(delta_kind);
                    prefix.push(anthropic_content_block_start_chunk(
                        target_index,
                        delta_kind,
                    ));
                }
            }
        }
        // This closes blocks before explicit block starts and terminal message
        // deltas. Content-block deltas, including synthesized kind switches
        // above, must stay inside their open block until the delta is emitted.
        if (is_content_block_start || is_message_delta || is_stop)
            && self.anthropic_open_content_block_index.is_some()
        {
            if let Some(index) = self.anthropic_open_content_block_index.take() {
                prefix.push(anthropic_content_block_stop_chunk(index)?);
                self.anthropic_open_content_block_kind = None;
            }
        }
        if is_content_block_start {
            self.anthropic_open_content_block_index = content_block_start_index;
            self.anthropic_open_content_block_kind = content_block_start_kind;
            if let (Some(source_index), Some(target_index), Some(kind)) = (
                source_content_block_start_index,
                content_block_start_index,
                content_block_start_kind,
            ) {
                self.anthropic_content_block_index_map
                    .insert((kind, source_index), target_index);
            }
            if let Some(index) = content_block_start_index {
                self.anthropic_next_content_block_index =
                    self.anthropic_next_content_block_index.max(index + 1);
            }
        }
        if is_content_block_stop {
            self.anthropic_open_content_block_index = None;
            self.anthropic_open_content_block_kind = None;
        }
        if is_tool_use_start {
            self.anthropic_tool_use_started = true;
        }

        if is_message_delta && self.buffered_delta.is_some() {
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
                self.anthropic_open_content_block_kind = None;
                self.anthropic_next_content_block_index = 0;
                self.anthropic_content_block_index_map.clear();
                out.push(stop);
            }
            return Ok(out);
        }

        if is_message_delta {
            self.buffered_delta = Some(chunk);
            return Ok(prefix);
        }

        if is_stop && self.buffered_delta.is_some() {
            self.buffered_stop = Some(chunk);
            return Ok(prefix);
        }

        if is_stop {
            self.anthropic_message_started = false;
            self.anthropic_tool_use_started = false;
            self.anthropic_open_content_block_index = None;
            self.anthropic_open_content_block_kind = None;
            self.anthropic_next_content_block_index = 0;
            self.anthropic_content_block_index_map.clear();
        }

        let mut out = prefix;
        out.extend(self.flush_buffered());
        out.push(chunk);
        Ok(out)
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
            self.anthropic_open_content_block_kind = None;
            self.anthropic_next_content_block_index = 0;
            self.anthropic_content_block_index_map.clear();
            out.push(stop);
        }
        out
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlockStartEventView {
    #[serde(rename = "type")]
    _event_type: AnthropicStreamEventTypeView,
    index: u32,
    content_block: Option<AnthropicContentBlockView>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnthropicContentBlockStartEvent {
    #[serde(rename = "type")]
    event_type: String,
    index: u32,
    content_block: Value,
}

impl AnthropicContentBlockStartEventView {
    fn is_tool_use_start(&self) -> bool {
        self.content_block
            .as_ref()
            .and_then(|block| block.block_type.as_deref())
            == Some("tool_use")
    }

    fn block_kind(&self) -> Option<AnthropicContentBlockKind> {
        self.content_block
            .as_ref()
            .and_then(|block| block.block_type.as_deref())
            .and_then(AnthropicContentBlockKind::from_content_block_type)
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlockDeltaEventView {
    #[serde(rename = "type")]
    _event_type: AnthropicStreamEventTypeView,
    index: u32,
    delta: Option<AnthropicContentBlockDeltaView>,
}

impl AnthropicContentBlockDeltaEventView {
    fn block_kind(&self) -> Option<AnthropicContentBlockKind> {
        self.delta
            .as_ref()
            .and_then(|delta| delta.delta_type.as_deref())
            .and_then(AnthropicContentBlockKind::from_delta_type)
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlockDeltaView {
    #[serde(rename = "type")]
    delta_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnthropicContentBlockDeltaEvent {
    #[serde(rename = "type")]
    event_type: String,
    index: u32,
    delta: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AnthropicContentBlockKind {
    Text,
    Thinking,
    ToolUse,
}

impl AnthropicContentBlockKind {
    fn from_content_block_type(block_type: &str) -> Option<Self> {
        match block_type {
            "text" => Some(Self::Text),
            "thinking" => Some(Self::Thinking),
            "tool_use" => Some(Self::ToolUse),
            _ => None,
        }
    }

    fn from_delta_type(delta_type: &str) -> Option<Self> {
        match delta_type {
            "text_delta" => Some(Self::Text),
            "thinking_delta" | "signature_delta" => Some(Self::Thinking),
            "input_json_delta" => Some(Self::ToolUse),
            _ => None,
        }
    }

    fn can_synthesize_start(self) -> bool {
        matches!(self, Self::Text | Self::Thinking)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AnthropicStreamEventTypeView {
    ContentBlockStart,
    ContentBlockDelta,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlockView {
    #[serde(rename = "type")]
    block_type: Option<String>,
}

fn parse_anthropic_content_block_start_event(
    chunk: &StreamOutputChunk,
) -> Result<Option<AnthropicContentBlockStartEventView>, TransformError> {
    if chunk.event_type.as_deref() != Some("content_block_start") {
        return Ok(None);
    }

    crate::serde_json::from_slice::<AnthropicContentBlockStartEventView>(&chunk.data)
        .map(Some)
        .map_err(|e| {
            TransformError::DeserializationFailed(format!(
                "Anthropic content_block_start event: {}",
                e
            ))
        })
}

fn parse_anthropic_content_block_delta_event(
    chunk: &StreamOutputChunk,
) -> Result<Option<AnthropicContentBlockDeltaEventView>, TransformError> {
    if chunk.event_type.as_deref() != Some("content_block_delta") {
        return Ok(None);
    }

    crate::serde_json::from_slice::<AnthropicContentBlockDeltaEventView>(&chunk.data)
        .map(Some)
        .map_err(|e| {
            TransformError::DeserializationFailed(format!(
                "Anthropic content_block_delta event: {}",
                e
            ))
        })
}

fn anthropic_content_block_stop_chunk(index: u32) -> Result<StreamOutputChunk, TransformError> {
    Ok(StreamOutputChunk::with_event(
        serialize_stream_value(&crate::serde_json::json!({
            "type": "content_block_stop",
            "index": index
        }))?,
        "content_block_stop".to_string(),
    ))
}

fn anthropic_content_block_start_chunk(
    index: u32,
    kind: AnthropicContentBlockKind,
) -> StreamOutputChunk {
    let data = match kind {
        AnthropicContentBlockKind::Text => Bytes::from(format!(
            r#"{{"type":"content_block_start","index":{index},"content_block":{{"type":"text","text":""}}}}"#
        )),
        AnthropicContentBlockKind::Thinking => Bytes::from(format!(
            r#"{{"type":"content_block_start","index":{index},"content_block":{{"type":"thinking","thinking":""}}}}"#
        )),
        AnthropicContentBlockKind::ToolUse => unreachable!("tool_use starts need provider IDs"),
    };
    StreamOutputChunk::with_event(data, "content_block_start".to_string())
}

fn with_anthropic_content_block_delta_index(
    chunk: StreamOutputChunk,
    index: u32,
) -> StreamOutputChunk {
    let Ok(mut event) =
        crate::serde_json::from_slice::<AnthropicContentBlockDeltaEvent>(&chunk.data)
    else {
        return chunk;
    };
    event.index = index;
    let Ok(data) = crate::serde_json::to_vec(&event).map(Bytes::from) else {
        return chunk;
    };
    StreamOutputChunk {
        data,
        event_type: chunk.event_type,
    }
}

fn with_anthropic_content_block_start_index(
    chunk: StreamOutputChunk,
    index: u32,
) -> StreamOutputChunk {
    let Ok(mut event) =
        crate::serde_json::from_slice::<AnthropicContentBlockStartEvent>(&chunk.data)
    else {
        return chunk;
    };
    event.index = index;
    let Ok(data) = crate::serde_json::to_vec(&event).map(Bytes::from) else {
        return chunk;
    };
    StreamOutputChunk {
        data,
        event_type: chunk.event_type,
    }
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
    state: SessionChunkState<'_>,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    let mut chunks = expand_transform_result(result)?;
    if target_format == ProviderFormat::Anthropic {
        return expand_anthropic_session_chunks(
            chunks,
            source_format,
            universal,
            source_is_native_stream,
            state.anthropic_message_started,
        );
    }
    if target_format == ProviderFormat::Responses {
        #[cfg(feature = "openai")]
        return expand_responses_session_chunks(
            chunks,
            universal,
            state.responses_message_started,
            state.responses_output_index_states,
        );
        #[cfg(not(feature = "openai"))]
        return Ok(std::mem::take(&mut chunks));
    }
    Ok(std::mem::take(&mut chunks))
}

fn expand_responses_session_chunks(
    chunks: Vec<StreamOutputChunk>,
    universal: Option<&UniversalStreamChunk>,
    responses_message_started: bool,
    responses_output_index_states: &mut BTreeMap<u32, ResponsesOutputIndexState>,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    let Some(universal) = universal else {
        return Ok(chunks);
    };

    let (choice_index, has_reasoning, has_content, has_finish) = universal
        .choices
        .first()
        .map(|choice| {
            let (has_reasoning, has_content) = choice
                .delta_view()
                .map(|delta| {
                    let has_reasoning = delta.reasoning.iter().any(|r| {
                        r.content
                            .as_deref()
                            .is_some_and(|content| !content.is_empty())
                    });
                    let has_content = delta
                        .content
                        .as_deref()
                        .is_some_and(|content| !content.is_empty());
                    (has_reasoning, has_content)
                })
                .unwrap_or((false, false));
            (
                choice.index,
                has_reasoning,
                has_content,
                choice.finish_reason.is_some(),
            )
        })
        .unwrap_or((0, false, false, false));
    let output_index_state = responses_output_index_states
        .get(&choice_index)
        .copied()
        .unwrap_or_default();

    let mut events = responses_stream_events_from_universal_with_output_index_offset(
        universal,
        output_index_state.text_output_index,
        output_index_state.tool_output_index_offset,
    );
    let mut next_output_index_state = output_index_state;
    if has_reasoning {
        next_output_index_state.tool_output_index_offset = next_output_index_state
            .tool_output_index_offset
            .max(choice_index + 1);
    }
    if has_content {
        let text_output_index = *next_output_index_state
            .text_output_index
            .get_or_insert_with(|| {
                choice_index.max(next_output_index_state.tool_output_index_offset)
            });
        next_output_index_state.tool_output_index_offset = next_output_index_state
            .tool_output_index_offset
            .max(text_output_index + 1);
    }
    if has_finish {
        responses_output_index_states.remove(&choice_index);
    } else if next_output_index_state.text_output_index.is_some()
        || next_output_index_state.tool_output_index_offset > 0
    {
        responses_output_index_states.insert(choice_index, next_output_index_state);
    }
    let has_metadata =
        universal.model.is_some() || universal.id.is_some() || universal.usage.is_some();
    if has_metadata
        && !responses_message_started
        && !events.is_empty()
        && events
            .first()
            .and_then(|event| event.get("type"))
            .and_then(Value::as_str)
            != Some("response.created")
    {
        events.insert(0, responses_created_stream_event_from_universal(universal));
    }

    let out = events
        .into_iter()
        .map(|event| {
            let event_type = extract_event_type(&event);
            Ok(StreamOutputChunk {
                data: serialize_stream_value(&event)?,
                event_type,
            })
        })
        .collect::<Result<Vec<_>, TransformError>>()?;

    if out.is_empty() {
        Ok(chunks)
    } else {
        Ok(out)
    }
}

fn expand_anthropic_session_chunks(
    mut chunks: Vec<StreamOutputChunk>,
    source_format: ProviderFormat,
    universal: Option<&UniversalStreamChunk>,
    source_is_native_stream: bool,
    anthropic_message_started: bool,
) -> Result<Vec<StreamOutputChunk>, TransformError> {
    if source_format == ProviderFormat::Anthropic && source_is_native_stream {
        return Ok(chunks);
    }

    let Some(universal) = universal else {
        return Ok(chunks);
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
    let reasoning_text = delta_view.as_ref().and_then(|d| {
        let text = d
            .reasoning
            .iter()
            .filter_map(|r| r.content.as_deref())
            .collect::<String>();
        (!text.is_empty()).then_some(text)
    });
    let reasoning_signature = delta_view
        .as_ref()
        .and_then(|d| d.reasoning_signature.as_deref())
        .filter(|signature| !signature.is_empty());
    let has_reasoning = reasoning_text.is_some() || reasoning_signature.is_some();
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
        && !has_reasoning
        && !universal.choices.is_empty()
        && delta_view
            .as_ref()
            .is_none_or(|d| d.content.as_deref().is_none_or(str::is_empty));

    let mut out = Vec::new();

    if is_initial_metadata && anthropic_message_started {
        return Ok(out);
    }

    if has_reasoning {
        let reasoning_index = choice.map(|c| c.index).unwrap_or(0);
        let content = delta_view
            .as_ref()
            .and_then(|d| d.content.as_deref())
            .filter(|s| !s.is_empty());
        if !anthropic_message_started {
            out.push(StreamOutputChunk::with_event(
                anthropic_message_start_bytes(universal)?,
                "message_start".to_string(),
            ));
            out.push(StreamOutputChunk::with_event(
                Bytes::from(format!(
                    r#"{{"type":"content_block_start","index":{reasoning_index},"content_block":{{"type":"thinking","thinking":""}}}}"#
                )),
                "content_block_start".to_string(),
            ));
        }
        if let Some(thinking) = reasoning_text {
            out.push(StreamOutputChunk::with_event(
                serialize_stream_value(&crate::serde_json::json!({
                    "type": "content_block_delta",
                    "index": reasoning_index,
                    "delta": {
                        "type": "thinking_delta",
                        "thinking": thinking
                    }
                }))?,
                "content_block_delta".to_string(),
            ));
        }
        if let Some(signature) = reasoning_signature {
            out.push(StreamOutputChunk::with_event(
                serialize_stream_value(&crate::serde_json::json!({
                    "type": "content_block_delta",
                    "index": reasoning_index,
                    "delta": {
                        "type": "signature_delta",
                        "signature": signature
                    }
                }))?,
                "content_block_delta".to_string(),
            ));
        }
        if let Some(content) = content {
            out.push(StreamOutputChunk::with_event(
                serialize_stream_value(&crate::serde_json::json!({
                    "type": "content_block_delta",
                    "index": reasoning_index,
                    "delta": {
                        "type": "text_delta",
                        "text": content
                    }
                }))?,
                "content_block_delta".to_string(),
            ));
        }
        if let Some(delta_view) = delta_view.as_ref() {
            for tool_call in &delta_view.tool_calls {
                let tool_index = tool_call.index.unwrap_or(reasoning_index);
                let function = tool_call.function.clone().unwrap_or_default();
                let tool_name = function.name.unwrap_or_default();
                let tool_id = tool_call.id.clone().unwrap_or_default();
                if !tool_name.is_empty() || !tool_id.is_empty() {
                    out.push(StreamOutputChunk::with_event(
                        serialize_stream_value(&crate::serde_json::json!({
                            "type": "content_block_start",
                            "index": tool_index,
                            "content_block": {
                                "type": "tool_use",
                                "id": tool_id,
                                "name": tool_name,
                                "input": {}
                            }
                        }))?,
                        "content_block_start".to_string(),
                    ));
                }
                if let Some(arguments) =
                    function.arguments.filter(|arguments| !arguments.is_empty())
                {
                    out.push(StreamOutputChunk::with_event(
                        serialize_stream_value(&crate::serde_json::json!({
                            "type": "content_block_delta",
                            "index": tool_index,
                            "delta": {
                                "type": "input_json_delta",
                                "partial_json": arguments
                            }
                        }))?,
                        "content_block_delta".to_string(),
                    ));
                }
            }
        }
        if has_finish {
            out.append(&mut chunks);
            out.push(StreamOutputChunk::with_event(
                Bytes::from_static(br#"{"type":"message_stop"}"#),
                "message_stop".to_string(),
            ));
        }
        return Ok(out);
    }

    if is_initial_metadata && !anthropic_message_started {
        if let Some(message_start) = chunks.first() {
            out.push(message_start.clone());
        }
        return Ok(out);
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
                anthropic_message_start_bytes(universal)?,
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
                }))?,
                "content_block_delta".to_string(),
            ));
            if needs_message_start {
                out.push(StreamOutputChunk::with_event(
                    serialize_stream_value(&crate::serde_json::json!({
                        "type": "content_block_stop",
                        "index": choice.map(|c| c.index).unwrap_or(0),
                    }))?,
                    "content_block_stop".to_string(),
                ));
            }
        }
        out.append(&mut chunks);
        out.push(StreamOutputChunk::with_event(
            Bytes::from_static(br#"{"type":"message_stop"}"#),
            "message_stop".to_string(),
        ));
        return Ok(out);
    }

    if has_metadata && has_tool_calls && is_initial_tool_call && !anthropic_message_started {
        out.push(StreamOutputChunk::with_event(
            anthropic_message_start_bytes(universal)?,
            "message_start".to_string(),
        ));
        out.append(&mut chunks);
        return Ok(out);
    }

    // Any other content-bearing chunk that arrives before the Anthropic message has
    // been opened must still emit message_start first. Some OpenAI-compatible providers
    // (e.g. GLM/zai) bundle id/model + role + the first text delta into a single chunk,
    // so there is no separate metadata-only or role-only chunk to open the message.
    // Without message_start the downstream ordering pass never synthesizes content block
    // starts/stops, so text and tool_use collide on index 0 and tool arguments are lost.
    if !anthropic_message_started && !chunks.is_empty() {
        out.push(StreamOutputChunk::with_event(
            anthropic_message_start_bytes(universal)?,
            "message_start".to_string(),
        ));
        out.append(&mut chunks);
        return Ok(out);
    }

    Ok(chunks)
}

fn anthropic_message_start_bytes(chunk: &UniversalStreamChunk) -> Result<Bytes, TransformError> {
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
    fn test_anthropic_content_block_start_requires_typed_index() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let malformed_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "content_block": {
                    "type": "text",
                    "text": ""
                }
            })),
            "content_block_start".to_string(),
        );

        let err = session.process_chunks(vec![malformed_start]).unwrap_err();
        assert!(matches!(err, TransformError::DeserializationFailed(_)));
        assert!(err.to_string().contains("Anthropic content_block_start"));
        assert!(err.to_string().contains("missing field `index`"));
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_stream_session_preserves_new_same_kind_anthropic_block_index() {
        #[derive(Deserialize)]
        struct ContentBlockStopEvent {
            index: u32,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let first_tool_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "lookup",
                    "input": {}
                }
            })),
            "content_block_start".to_string(),
        );
        let second_tool_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "index": 1,
                "content_block": {
                    "type": "tool_use",
                    "id": "toolu_2",
                    "name": "lookup",
                    "input": {}
                }
            })),
            "content_block_start".to_string(),
        );
        let first_tool_arguments = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": "{\"first\":true}"
                }
            })),
            "content_block_delta".to_string(),
        );

        let first = session.process_chunks(vec![first_tool_start]).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_start")]
        );

        let second = session.process_chunks(vec![second_tool_start]).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_stop"), Some("content_block_start")]
        );

        let stop: ContentBlockStopEvent = crate::serde_json::from_slice(&second[0].data)
            .expect("content_block_stop should parse");
        assert_eq!(stop.index, 0);

        let start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&second[1].data)
                .expect("content_block_start should parse");
        assert_eq!(start.index, 1);
        assert_eq!(start.block_kind(), Some(AnthropicContentBlockKind::ToolUse));

        let arguments = session.process_chunks(vec![first_tool_arguments]).unwrap();
        assert_eq!(
            arguments
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_delta")]
        );
        let arguments_delta: AnthropicContentBlockDeltaEventView =
            crate::serde_json::from_slice(&arguments[0].data)
                .expect("content_block_delta should parse");
        assert_eq!(arguments_delta.index, 0);
        assert_eq!(
            arguments_delta.block_kind(),
            Some(AnthropicContentBlockKind::ToolUse)
        );
    }

    #[test]
    #[cfg(feature = "anthropic")]
    fn test_stream_session_allocates_unique_same_kind_tool_start_after_remap() {
        #[derive(Deserialize)]
        struct ContentBlockStopEvent {
            index: u32,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let thinking_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {
                    "type": "thinking",
                    "thinking": ""
                }
            })),
            "content_block_start".to_string(),
        );
        let first_tool_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "index": 0,
                "content_block": {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "lookup",
                    "input": {}
                }
            })),
            "content_block_start".to_string(),
        );
        let second_tool_start = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_start",
                "index": 1,
                "content_block": {
                    "type": "tool_use",
                    "id": "toolu_2",
                    "name": "lookup",
                    "input": {}
                }
            })),
            "content_block_start".to_string(),
        );
        let first_tool_arguments = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_delta",
                "index": 0,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": "{\"first\":true}"
                }
            })),
            "content_block_delta".to_string(),
        );
        let second_tool_arguments = StreamOutputChunk::with_event(
            to_bytes(&json!({
                "type": "content_block_delta",
                "index": 1,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": "{\"second\":true}"
                }
            })),
            "content_block_delta".to_string(),
        );

        let thinking = session.process_chunks(vec![thinking_start]).unwrap();
        assert_eq!(
            thinking
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_start")]
        );

        let first_tool = session.process_chunks(vec![first_tool_start]).unwrap();
        assert_eq!(
            first_tool
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_stop"), Some("content_block_start")]
        );
        let thinking_stop: ContentBlockStopEvent =
            crate::serde_json::from_slice(&first_tool[0].data).unwrap();
        assert_eq!(thinking_stop.index, 0);
        let first_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&first_tool[1].data).unwrap();
        assert_eq!(first_start.index, 1);
        assert_eq!(
            first_start.block_kind(),
            Some(AnthropicContentBlockKind::ToolUse)
        );

        let second_tool = session.process_chunks(vec![second_tool_start]).unwrap();
        assert_eq!(
            second_tool
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_stop"), Some("content_block_start")]
        );
        let first_tool_stop: ContentBlockStopEvent =
            crate::serde_json::from_slice(&second_tool[0].data).unwrap();
        assert_eq!(first_tool_stop.index, 1);
        let second_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&second_tool[1].data).unwrap();
        assert_eq!(second_start.index, 2);
        assert_eq!(
            second_start.block_kind(),
            Some(AnthropicContentBlockKind::ToolUse)
        );

        let first_arguments = session.process_chunks(vec![first_tool_arguments]).unwrap();
        let first_delta: AnthropicContentBlockDeltaEventView =
            crate::serde_json::from_slice(&first_arguments[0].data).unwrap();
        assert_eq!(first_delta.index, 1);

        let second_arguments = session.process_chunks(vec![second_tool_arguments]).unwrap();
        let second_delta: AnthropicContentBlockDeltaEventView =
            crate::serde_json::from_slice(&second_arguments[0].data).unwrap();
        assert_eq!(second_delta.index, 2);
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
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event_type.as_deref(), Some("message_start"));
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
            vec![Some("message_start")]
        );

        let second = session.push(in_progress).unwrap();
        assert!(second.is_empty());

        let third = session.push(tool_start).unwrap();
        assert_eq!(
            third
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_start")]
        );

        let tool_block: Value = crate::serde_json::from_slice(&third[0].data).unwrap();
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
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_responses_reasoning_after_metadata_starts_thinking_block() {
        #[derive(Deserialize)]
        struct ThinkingDeltaEvent {
            index: u32,
            delta: ThinkingDelta,
        }

        #[derive(Deserialize)]
        struct ThinkingDelta {
            thinking: String,
        }

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
        let reasoning = to_bytes(&json!({
            "type": "response.reasoning_summary_text.delta",
            "output_index": 0,
            "summary_index": 0,
            "delta": "thinking after metadata"
        }));

        let first = session.push(created).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_start")]
        );

        let second = session.push(reasoning).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_start"), Some("content_block_delta")]
        );

        let thinking_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&second[0].data).unwrap();
        assert_eq!(
            thinking_start.block_kind(),
            Some(AnthropicContentBlockKind::Thinking)
        );
        assert_eq!(thinking_start.index, 0);

        let thinking_delta: ThinkingDeltaEvent =
            crate::serde_json::from_slice(&second[1].data).unwrap();
        assert_eq!(thinking_delta.index, 0);
        assert_eq!(thinking_delta.delta.thinking, "thinking after metadata");
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_thought_after_metadata_starts_thinking_block() {
        #[derive(Deserialize)]
        struct ThinkingDeltaEvent {
            index: u32,
            delta: ThinkingDelta,
        }

        #[derive(Deserialize)]
        struct ThinkingDelta {
            thinking: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let metadata = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": []
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "totalTokenCount": 1
            }
        }));
        let thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "thinking after metadata",
                        "thought": true
                    }]
                }
            }]
        }));

        let first = session.push(metadata).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_start")]
        );

        let second = session.push(thought).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_start"), Some("content_block_delta")]
        );

        let thinking_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&second[0].data).unwrap();
        assert_eq!(
            thinking_start.block_kind(),
            Some(AnthropicContentBlockKind::Thinking)
        );
        assert_eq!(thinking_start.index, 0);

        let thinking_delta: ThinkingDeltaEvent =
            crate::serde_json::from_slice(&second[1].data).unwrap();
        assert_eq!(thinking_delta.index, 0);
        assert_eq!(thinking_delta.delta.thinking, "thinking after metadata");
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
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_thought_only_chunks_are_anthropic_thinking_deltas() {
        #[derive(Deserialize)]
        struct ThinkingDeltaEvent {
            delta: ThinkingDelta,
        }

        #[derive(Deserialize)]
        struct ThinkingDelta {
            thinking: String,
        }

        #[derive(Deserialize)]
        struct SignatureDeltaEvent {
            index: u32,
            delta: SignatureDelta,
        }

        #[derive(Deserialize)]
        struct SignatureDelta {
            signature: String,
        }

        #[derive(Deserialize)]
        struct TextDeltaEvent {
            index: u32,
            delta: TextDelta,
        }

        #[derive(Deserialize)]
        struct TextDelta {
            text: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let first_thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "thinking before visible text",
                        "thought": true
                    }]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "thoughtsTokenCount": 2,
                "totalTokenCount": 3
            }
        }));
        let later_thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "later thinking",
                        "thought": true
                    }]
                }
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "thoughtsTokenCount": 4,
                "totalTokenCount": 5
            }
        }));
        let visible_text = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "The visible answer.",
                        "thoughtSignature": "sig_on_text_part"
                    }]
                }
            }]
        }));

        let first = session.push(first_thought).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta")
            ]
        );

        let first_delta: ThinkingDeltaEvent =
            crate::serde_json::from_slice(&first[2].data).unwrap();
        assert_eq!(first_delta.delta.thinking, "thinking before visible text");

        let second = session.push(later_thought).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("content_block_delta")]
        );

        let second_delta: ThinkingDeltaEvent =
            crate::serde_json::from_slice(&second[0].data).unwrap();
        assert_eq!(second_delta.delta.thinking, "later thinking");

        let third = session.push(visible_text).unwrap();
        assert_eq!(
            third
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("content_block_delta"),
                Some("content_block_stop"),
                Some("content_block_start"),
                Some("content_block_delta")
            ]
        );

        let signature_delta: SignatureDeltaEvent =
            crate::serde_json::from_slice(&third[0].data).unwrap();
        assert_eq!(signature_delta.index, 0);
        assert_eq!(signature_delta.delta.signature, "sig_on_text_part");

        let text_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&third[2].data).unwrap();
        assert_eq!(
            text_start.block_kind(),
            Some(AnthropicContentBlockKind::Text)
        );
        assert_eq!(text_start.index, 1);

        let text_delta: TextDeltaEvent = crate::serde_json::from_slice(&third[3].data).unwrap();
        assert_eq!(text_delta.index, 1);
        assert_eq!(text_delta.delta.text, "The visible answer.");
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_nonzero_candidate_thinking_starts_matching_block() {
        #[derive(Deserialize)]
        struct ThinkingDeltaEvent {
            index: u32,
            delta: ThinkingDelta,
        }

        #[derive(Deserialize)]
        struct ThinkingDelta {
            thinking: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 1,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "candidate one thinking",
                        "thought": true
                    }]
                }
            }]
        }));

        let out = session.push(thought).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta")
            ]
        );

        let thinking_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&out[1].data).unwrap();
        assert_eq!(
            thinking_start.block_kind(),
            Some(AnthropicContentBlockKind::Thinking)
        );
        assert_eq!(thinking_start.index, 1);

        let thinking_delta: ThinkingDeltaEvent =
            crate::serde_json::from_slice(&out[2].data).unwrap();
        assert_eq!(thinking_delta.index, 1);
        assert_eq!(thinking_delta.delta.thinking, "candidate one thinking");
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_mixed_thought_and_text_chunk_preserves_both_blocks() {
        #[derive(Deserialize)]
        struct ContentBlockStopEvent {
            index: u32,
        }

        #[derive(Deserialize)]
        struct TextDeltaEvent {
            index: u32,
            delta: TextDelta,
        }

        #[derive(Deserialize)]
        struct TextDelta {
            text: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let mixed_final = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "thinking in the same candidate",
                            "thought": true
                        },
                        {
                            "text": "{\"answer\":\"visible json\"}"
                        }
                    ]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "candidatesTokenCount": 4,
                "thoughtsTokenCount": 2,
                "totalTokenCount": 7
            }
        }));

        let out = session.push(mixed_final).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta"),
                Some("content_block_stop"),
                Some("content_block_start"),
                Some("content_block_delta"),
                Some("content_block_stop")
            ]
        );

        let text_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&out[4].data).unwrap();
        assert_eq!(text_start.index, 1);
        assert_eq!(
            text_start.block_kind(),
            Some(AnthropicContentBlockKind::Text)
        );

        let text_delta: TextDeltaEvent = crate::serde_json::from_slice(&out[5].data).unwrap();
        assert_eq!(text_delta.index, 1);
        assert_eq!(text_delta.delta.text, "{\"answer\":\"visible json\"}");

        let text_stop: ContentBlockStopEvent = crate::serde_json::from_slice(&out[6].data).unwrap();
        assert_eq!(text_stop.index, 1);

        let final_chunks = session.finish();
        assert_eq!(
            final_chunks
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("message_delta"), Some("message_stop")]
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "anthropic"))]
    fn test_stream_session_google_mixed_thought_and_tool_call_preserves_tool_use() {
        #[derive(Deserialize)]
        struct SignatureDeltaEvent {
            index: u32,
            delta: SignatureDelta,
        }

        #[derive(Deserialize)]
        struct SignatureDelta {
            signature: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);
        let mixed_tool = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "thinking before the tool",
                            "thought": true,
                            "thoughtSignature": "sig_abc123"
                        },
                        {
                            "functionCall": {
                                "name": "lookup_creator",
                                "args": {
                                    "query": "microphone comparison"
                                }
                            }
                        }
                    ]
                }
            }]
        }));

        let out = session.push(mixed_tool).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("message_start"),
                Some("content_block_start"),
                Some("content_block_delta"),
                Some("content_block_delta"),
                Some("content_block_stop"),
                Some("content_block_start"),
                Some("content_block_delta")
            ]
        );

        let signature_delta: SignatureDeltaEvent =
            crate::serde_json::from_slice(&out[3].data).unwrap();
        assert_eq!(signature_delta.index, 0);
        assert_eq!(signature_delta.delta.signature, "sig_abc123");

        let tool_start: AnthropicContentBlockStartEventView =
            crate::serde_json::from_slice(&out[5].data).unwrap();
        assert_eq!(tool_start.index, 1);
        assert_eq!(
            tool_start.block_kind(),
            Some(AnthropicContentBlockKind::ToolUse)
        );

        let tool_delta: AnthropicContentBlockDeltaEventView =
            crate::serde_json::from_slice(&out[6].data).unwrap();
        assert_eq!(tool_delta.index, 1);
        assert_eq!(
            tool_delta.block_kind(),
            Some(AnthropicContentBlockKind::ToolUse)
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "openai"))]
    fn test_stream_session_google_mixed_thought_and_text_expands_to_responses_events() {
        let mut session = StreamTransformSession::new(ProviderFormat::Responses);
        let mixed_final = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "thinking in the same candidate",
                            "thought": true
                        },
                        {
                            "text": "{\"answer\":\"visible json\"}"
                        }
                    ]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 1,
                "candidatesTokenCount": 4,
                "thoughtsTokenCount": 2,
                "totalTokenCount": 7
            }
        }));

        let out = session.push(mixed_final).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.created"),
                Some("response.reasoning_summary_text.delta"),
                Some("response.output_text.delta"),
                Some("response.completed")
            ]
        );

        let created: Value = crate::serde_json::from_slice(&out[0].data).unwrap();
        assert_eq!(
            created
                .get("response")
                .and_then(|response| response.get("id"))
                .and_then(Value::as_str),
            Some("response_123")
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "openai"))]
    fn test_stream_session_google_mixed_thought_and_tool_expands_to_responses_events() {
        let mut session = StreamTransformSession::new(ProviderFormat::Responses);
        let mixed_tool = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "thinking before the tool",
                            "thought": true
                        },
                        {
                            "functionCall": {
                                "name": "lookup_creator",
                                "args": {
                                    "query": "microphone comparison"
                                }
                            }
                        }
                    ]
                }
            }]
        }));

        let out = session.push(mixed_tool).unwrap();
        assert_eq!(
            out.iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.created"),
                Some("response.reasoning_summary_text.delta"),
                Some("response.output_item.added"),
                Some("response.function_call_arguments.delta")
            ]
        );
    }

    #[test]
    #[cfg(all(feature = "google", feature = "openai"))]
    fn test_stream_session_google_response_metadata_created_once_for_responses() {
        #[derive(Deserialize)]
        struct ReasoningDelta {
            output_index: u32,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Responses);
        let first_thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "first thought",
                        "thought": true
                    }]
                }
            }]
        }));
        let second_thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "second thought",
                        "thought": true
                    }]
                }
            }]
        }));

        let first = session.push(first_thought).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.created"),
                Some("response.reasoning_summary_text.delta")
            ]
        );

        let second = session.push(second_thought).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("response.reasoning_summary_text.delta")]
        );
        let second_reasoning: ReasoningDelta =
            crate::serde_json::from_slice(&second[0].data).unwrap();
        assert_eq!(second_reasoning.output_index, 0);
    }

    #[test]
    #[cfg(all(feature = "google", feature = "openai"))]
    fn test_stream_session_google_split_thought_then_text_offsets_responses_text_index() {
        #[derive(Deserialize)]
        struct OutputTextDelta {
            output_index: u32,
            delta: String,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Responses);
        let thought = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "thinking first",
                        "thought": true
                    }]
                }
            }]
        }));
        let visible_text = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "{\"answer\":\"visible json\"}"
                    }]
                },
                "finishReason": "STOP"
            }]
        }));

        let first = session.push(thought).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.created"),
                Some("response.reasoning_summary_text.delta")
            ]
        );

        let second = session.push(visible_text).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.output_text.delta"),
                Some("response.completed")
            ]
        );

        let text_delta: OutputTextDelta = crate::serde_json::from_slice(&second[0].data).unwrap();
        assert_eq!(text_delta.output_index, 1);
        assert_eq!(text_delta.delta, "{\"answer\":\"visible json\"}");
    }

    #[test]
    #[cfg(all(feature = "google", feature = "openai"))]
    fn test_stream_session_google_text_then_tool_uses_next_responses_output_index() {
        #[derive(Deserialize)]
        struct OutputEvent {
            output_index: u32,
        }

        let mut session = StreamTransformSession::new(ProviderFormat::Responses);
        let thought_and_text = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [
                        {
                            "text": "thinking first",
                            "thought": true
                        },
                        {
                            "text": "{\"answer\":\"visible json\"}"
                        }
                    ]
                }
            }]
        }));
        let tool_call = to_bytes(&json!({
            "responseId": "response_123",
            "modelVersion": "gemini-2.5-flash",
            "candidates": [{
                "index": 0,
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": {
                            "name": "lookup_creator",
                            "args": {
                                "query": "microphone comparison"
                            }
                        }
                    }]
                },
                "finishReason": "STOP"
            }]
        }));

        let first = session.push(thought_and_text).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.created"),
                Some("response.reasoning_summary_text.delta"),
                Some("response.output_text.delta")
            ]
        );
        let text_delta: OutputEvent = crate::serde_json::from_slice(&first[2].data).unwrap();
        assert_eq!(text_delta.output_index, 1);

        let second = session.push(tool_call).unwrap();
        assert_eq!(
            second
                .iter()
                .map(|chunk| chunk.event_type.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("response.output_item.added"),
                Some("response.function_call_arguments.delta"),
                Some("response.completed")
            ]
        );
        let tool_start: OutputEvent = crate::serde_json::from_slice(&second[0].data).unwrap();
        let tool_args: OutputEvent = crate::serde_json::from_slice(&second[1].data).unwrap();
        assert_eq!(tool_start.output_index, 2);
        assert_eq!(tool_args.output_index, 2);
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
    #[cfg(feature = "openai")]
    fn test_stream_session_maps_responses_output_item_indexes_to_tool_call_indexes() {
        let mut session = StreamTransformSession::new(ProviderFormat::ChatCompletions);

        let reasoning = to_bytes(&json!({
            "type": "response.reasoning_summary_text.delta",
            "output_index": 0,
            "summary_index": 0,
            "delta": "thinking before tool"
        }));
        let reasoning_out = session.push(reasoning).unwrap();
        assert_eq!(reasoning_out.len(), 1);
        let reasoning_chunk: Value = crate::serde_json::from_slice(&reasoning_out[0].data).unwrap();
        assert_eq!(
            reasoning_chunk["choices"][0]["index"],
            json!(0),
            "reasoning output item should remain one assistant choice"
        );

        let tool_start = to_bytes(&json!({
            "type": "response.output_item.added",
            "output_index": 1,
            "item": {
                "type": "function_call",
                "status": "in_progress",
                "call_id": "call_lookup",
                "name": "lookup_creator",
                "arguments": ""
            }
        }));
        let start_out = session.push(tool_start).unwrap();
        assert_eq!(start_out.len(), 1);
        let start_chunk: Value = crate::serde_json::from_slice(&start_out[0].data).unwrap();
        assert_eq!(start_chunk["choices"][0]["index"], json!(0));
        assert_eq!(
            start_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );

        let tool_args = to_bytes(&json!({
            "type": "response.function_call_arguments.delta",
            "output_index": 1,
            "delta": "{\"query\":\"microphone comparison\"}"
        }));
        let args_out = session.push(tool_args).unwrap();
        assert_eq!(args_out.len(), 1);
        let args_chunk: Value = crate::serde_json::from_slice(&args_out[0].data).unwrap();
        assert_eq!(args_chunk["choices"][0]["index"], json!(0));
        assert_eq!(
            args_chunk["choices"][0]["delta"]["tool_calls"][0]["index"],
            json!(0)
        );
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

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_openai_text_then_tool_call_emits_anthropic_framing() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        // First chunk carries metadata + role + the first text delta together.
        let text_start = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "created": 123,
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": "Sure" },
                "finish_reason": null
            }]
        }));
        let text_more = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": { "content": "! Let me check the weather." },
                "finish_reason": null
            }]
        }));
        let tool_start = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "chatcmpl-tool-abc",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let tool_args = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": { "arguments": "{\"city\": \"San Francisco\"}" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let finish = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }]
        }));
        let usage = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
        }));

        let mut events: Vec<StreamOutputChunk> = Vec::new();
        for chunk in [text_start, text_more, tool_start, tool_args, finish, usage] {
            events.extend(session.push(chunk).unwrap());
        }
        events.extend(session.finish());

        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|chunk| chunk.event_type.as_deref())
            .collect();

        assert_eq!(
            kinds,
            vec![
                "message_start",
                "content_block_start",
                "content_block_delta",
                "content_block_delta",
                "content_block_stop",
                "content_block_start",
                "content_block_delta",
                "content_block_stop",
                "message_delta",
                "message_stop",
            ],
            "unexpected Anthropic event sequence: {kinds:?}"
        );

        let parsed: Vec<Value> = events
            .iter()
            .map(|chunk| crate::serde_json::from_slice(&chunk.data).unwrap())
            .collect();

        // Text block opens at index 0 as a text block.
        assert_eq!(parsed[1]["index"], json!(0));
        assert_eq!(parsed[1]["content_block"]["type"], json!("text"));

        // Text deltas stay on index 0.
        assert_eq!(parsed[2]["index"], json!(0));
        assert_eq!(parsed[2]["delta"]["text"], json!("Sure"));
        assert_eq!(parsed[3]["index"], json!(0));

        // Text block is closed at index 0.
        assert_eq!(parsed[4]["index"], json!(0));

        // Tool_use opens a NEW block index (1), not colliding with the text block.
        assert_eq!(parsed[5]["index"], json!(1));
        assert_eq!(parsed[5]["content_block"]["type"], json!("tool_use"));
        assert_eq!(parsed[5]["content_block"]["name"], json!("get_weather"));

        // Arguments are streamed as input_json_delta on the tool_use index.
        assert_eq!(parsed[6]["index"], json!(1));
        assert_eq!(parsed[6]["delta"]["type"], json!("input_json_delta"));
        assert_eq!(
            parsed[6]["delta"]["partial_json"],
            json!("{\"city\": \"San Francisco\"}")
        );

        // Tool block closed at index 1.
        assert_eq!(parsed[7]["index"], json!(1));

        // Terminators.
        assert_eq!(parsed[8]["delta"]["stop_reason"], json!("tool_use"));
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_openai_clean_text_then_tool_call_emits_anthropic_framing() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        // Role-only first chunk with empty content (native OpenAI shape).
        let role_only = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": "" },
                "finish_reason": null
            }]
        }));
        let text = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [{ "index": 0, "delta": { "content": "Sure" }, "finish_reason": null }]
        }));
        let tool_start = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_abc",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let tool_args = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": { "arguments": "{\"city\": \"San Francisco\"}" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let finish = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }]
        }));
        let usage = to_bytes(&json!({
            "id": "chatcmpl-oai",
            "object": "chat.completion.chunk",
            "model": "gpt-5-nano",
            "choices": [],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
        }));

        let mut events: Vec<StreamOutputChunk> = Vec::new();
        for chunk in [role_only, text, tool_start, tool_args, finish, usage] {
            events.extend(session.push(chunk).unwrap());
        }
        events.extend(session.finish());

        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|chunk| chunk.event_type.as_deref())
            .collect();

        assert_eq!(
            kinds,
            vec![
                "message_start",
                "content_block_start",
                "content_block_delta",
                "content_block_stop",
                "content_block_start",
                "content_block_delta",
                "content_block_stop",
                "message_delta",
                "message_stop",
            ],
            "unexpected Anthropic event sequence: {kinds:?}"
        );
    }

    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_glm_bundled_tool_call_streams_all_arguments() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let text = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "I" }, "finish_reason": null }]
        }));
        // Opening tool delta bundles name + first argument fragment.
        let tool_open = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "chatcmpl-tool-abc",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "{" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        // Continuation delta repeats the id and carries only arguments.
        let tool_cont = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "chatcmpl-tool-abc",
                        "function": { "arguments": "\"location\": \"San Francisco\"" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        // Final argument fragment rides on the finish delta.
        let finish = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "}" } }] },
                "finish_reason": "tool_calls"
            }]
        }));
        let usage = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
        }));

        let mut events: Vec<StreamOutputChunk> = Vec::new();
        for chunk in [text, tool_open, tool_cont, finish, usage] {
            events.extend(session.push(chunk).unwrap());
        }
        events.extend(session.finish());

        let parsed: Vec<Value> = events
            .iter()
            .map(|chunk| crate::serde_json::from_slice(&chunk.data).unwrap())
            .collect();
        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|chunk| chunk.event_type.as_deref())
            .collect();

        assert_eq!(
            kinds,
            vec![
                "message_start",
                "content_block_start",
                "content_block_delta",
                "content_block_stop",
                "content_block_start",
                "content_block_delta",
                "content_block_delta",
                "content_block_delta",
                "content_block_stop",
                "message_delta",
                "message_stop",
            ],
            "unexpected Anthropic event sequence: {kinds:?}"
        );

        // Exactly one tool_use block is opened, named, at index 1.
        let tool_starts: Vec<&Value> = parsed
            .iter()
            .filter(|event| {
                event["type"] == json!("content_block_start")
                    && event["content_block"]["type"] == json!("tool_use")
            })
            .collect();
        assert_eq!(tool_starts.len(), 1);
        assert_eq!(tool_starts[0]["index"], json!(1));
        assert_eq!(
            tool_starts[0]["content_block"]["name"],
            json!("get_weather")
        );

        // Every argument fragment is streamed as input_json_delta on index 1 and they
        // reconstruct the full tool input.
        let partial_json: String = parsed
            .iter()
            .filter(|event| event["delta"]["type"] == json!("input_json_delta"))
            .map(|event| {
                assert_eq!(event["index"], json!(1));
                event["delta"]["partial_json"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        assert_eq!(partial_json, "{\"location\": \"San Francisco\"}");
        assert_eq!(
            serde_json::from_str::<Value>(&partial_json).unwrap(),
            json!({ "location": "San Francisco" })
        );
    }

    // Regression: a single OpenAI-compatible delta may carry BOTH assistant text and the
    // opening of a tool call (GLM/zai bundles the final text fragment onto the tool-call
    // chunk). The text must not be dropped — it stays on the text block before the
    // tool_use block opens. See glm-bug.md.
    #[test]
    #[cfg(all(feature = "openai", feature = "anthropic"))]
    fn test_stream_session_bundled_text_and_tool_call_preserves_text() {
        let mut session = StreamTransformSession::new(ProviderFormat::Anthropic);

        let text = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "Sure" }, "finish_reason": null }]
        }));
        // This delta carries the trailing text "." AND opens the tool call.
        let text_and_tool = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": ".",
                    "tool_calls": [{
                        "index": 0,
                        "id": "chatcmpl-tool-abc",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "{\"city\": \"SF\"}" }
                    }]
                },
                "finish_reason": null
            }]
        }));
        let finish = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }]
        }));
        let usage = to_bytes(&json!({
            "id": "chatcmpl-glm",
            "object": "chat.completion.chunk",
            "model": "zai-org/GLM-5.2",
            "choices": [],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
        }));

        let mut events: Vec<StreamOutputChunk> = Vec::new();
        for chunk in [text, text_and_tool, finish, usage] {
            events.extend(session.push(chunk).unwrap());
        }
        events.extend(session.finish());

        let parsed: Vec<Value> = events
            .iter()
            .map(|chunk| crate::serde_json::from_slice(&chunk.data).unwrap())
            .collect();

        // Both text fragments are preserved on the text block (index 0).
        let text: String = parsed
            .iter()
            .filter(|event| event["delta"]["type"] == json!("text_delta"))
            .map(|event| {
                assert_eq!(event["index"], json!(0));
                event["delta"]["text"].as_str().unwrap_or("").to_string()
            })
            .collect();
        assert_eq!(text, "Sure.");

        // The tool_use block still opens (on a new index) with streamed arguments.
        let tool_start = parsed.iter().find(|event| {
            event["type"] == json!("content_block_start")
                && event["content_block"]["type"] == json!("tool_use")
        });
        assert_eq!(
            tool_start.map(|event| event["content_block"]["name"].clone()),
            Some(json!("get_weather"))
        );
        let partial_json: String = parsed
            .iter()
            .filter(|event| event["delta"]["type"] == json!("input_json_delta"))
            .map(|event| event["delta"]["partial_json"].as_str().unwrap_or("").to_string())
            .collect();
        assert_eq!(partial_json, "{\"city\": \"SF\"}");
    }
}
