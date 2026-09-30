mod stream;
#[cfg(test)]
mod tests;
pub use stream::{TrajectoryCollector, TrajectoryEvent, TrajectoryStream};

use crate::processing::import::{ImportedSpan, SpanContext};
use crate::processing::message_dedup_hash;
use crate::serde_json as json;
use crate::universal::trajectory::{
    Agent, AgentResponse, LLMAnalysis, Scope, ToolResult, Trajectory, Turn, Work, WorkStep,
};
use crate::universal::{AssistantContent, AssistantContentPart, Message};
use crate::UniversalUsage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailure {
    pub root_span_id: String,
    pub span_id: String,
    pub message: String,
}

impl ImportFailure {
    fn from_context(context: &SpanContext, message: String) -> Result<Self> {
        Ok(Self {
            root_span_id: context
                .root_span_id
                .clone()
                .ok_or_else(|| format!("Cannot identify failed span: {message}"))?,
            span_id: context
                .id
                .clone()
                .ok_or_else(|| format!("Cannot identify failed span: {message}"))?,
            message,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PreparedSpan {
    id: String,
    root_span_id: String,
    span_id: String,
    start: DateTime<Utc>,
    source: SpanContext,
    input: Vec<Message>,
    input_keys: Vec<u64>,
    output: Vec<Message>,
    context_messages: HashSet<usize>,
    interruption_offsets: Vec<usize>,
    usage: Option<UniversalUsage>,
    tool_result: Option<ToolResult>,
    failure: Option<ImportFailure>,
}

impl PreparedSpan {
    pub fn new(mut span: ImportedSpan) -> Result<Self> {
        let mut source = span.header;
        let id = source.id.take().ok_or("Missing trajectory span id")?;
        let root_span_id = source
            .root_span_id
            .take()
            .ok_or("Missing trajectory root span id")?;
        let span_id = source.span_id.take().unwrap_or_else(|| id.clone());
        let start = source
            .start
            .take()
            .ok_or_else(|| format!("Missing or invalid timestamp for trajectory span {id}"))?;
        let mut context_messages: HashSet<_> = span.context_messages.into_iter().collect();
        let current_start = span
            .input
            .iter()
            .rposition(|message| {
                matches!(message, Message::Assistant { .. } | Message::Tool { .. })
            })
            .map_or(0, |index| index + 1);
        let mut input_keys = Vec::new();
        let mut interruption_offsets = Vec::new();
        for (index, message) in span.input.iter().enumerate() {
            if index >= current_start && span.interruption_messages.contains(&index) {
                interruption_offsets.push(input_keys.len());
            }
            if !context_messages.contains(&index) && !is_context(message) {
                input_keys.push(message_dedup_hash(message));
            }
        }
        if !source.analysis {
            span.input.drain(..current_start);
            context_messages = context_messages
                .into_iter()
                .filter_map(|index| index.checked_sub(current_start))
                .collect();
        }
        let failure = (!span.errors.is_empty()).then(|| ImportFailure {
            root_span_id: root_span_id.clone(),
            span_id: id.clone(),
            message: span.errors.join("; "),
        });
        Ok(Self {
            id,
            root_span_id,
            span_id,
            start,
            source,
            input: span.input,
            output: span.output,
            input_keys,
            context_messages,
            interruption_offsets,
            usage: span.usage,
            tool_result: span.tool_result,
            failure,
        })
    }

    fn current_input(&self) -> impl Iterator<Item = (usize, &Message)> {
        let start = self
            .input
            .iter()
            .rposition(|message| {
                matches!(message, Message::Assistant { .. } | Message::Tool { .. })
            })
            .map_or(0, |index| index + 1);
        self.input.iter().enumerate().skip(start)
    }

    fn is_context(&self, index: usize, message: &Message) -> bool {
        self.context_messages.contains(&index) || is_context(message)
    }

    fn kind(&self) -> &str {
        &self.source.kind
    }
    fn is_scorer(&self) -> bool {
        self.kind() == "score" || self.source.scorer
    }
    fn usage(&self) -> Option<UniversalUsage> {
        self.usage.clone()
    }
    fn response(&self) -> AgentResponse {
        let mut parts = Vec::new();
        for message in &self.output {
            if let Message::Assistant { content, .. } = message {
                match content {
                    AssistantContent::String(text) => parts.push(AssistantContentPart::Text(
                        crate::universal::TextContentPart {
                            text: text.clone(),
                            encrypted_content: None,
                            cache_control: None,
                            provider_options: None,
                        },
                    )),
                    AssistantContent::Array(content) => parts.extend(content.iter().cloned()),
                }
            }
        }
        AgentResponse {
            response: Some(AssistantContent::Array(parts)),
            usage: self.usage(),
            start_time: Some(self.start),
            end_time: self.source.end,
        }
    }

    fn can_finish_turn(&self) -> bool {
        matches!(self.kind(), "llm" | "task") && !self.source.analysis && self.source.end.is_some() && self.source.error.is_none()
            && self.output.iter().any(|message| match message {
                Message::Assistant { content: AssistantContent::String(text), .. } => !text.trim().is_empty(),
                Message::Assistant { content: AssistantContent::Array(parts), .. } => !parts.is_empty(),
                _ => false,
            })
            && !self.output.iter().any(|message| matches!(message,
                Message::Assistant { content: AssistantContent::Array(parts), .. }
                    if parts.iter().any(|part| matches!(part,
                        AssistantContentPart::ToolCall { .. } | AssistantContentPart::ToolDiscoveryCall { .. }))))
    }
}

fn is_context(message: &Message) -> bool {
    matches!(
        message,
        Message::System { .. } | Message::Developer { .. } | Message::AdditionalTools { .. }
    )
}

fn message_keys(messages: &[Message]) -> Vec<u64> {
    messages
        .iter()
        .filter(|message| !is_context(message))
        .map(message_dedup_hash)
        .collect()
}

#[derive(Clone, Default)]
struct Ownership {
    turn: Option<String>,
    tool: Option<usize>,
    compaction: Option<usize>,
    skipped: bool,
}

fn ownership(
    index: usize,
    spans: &[PreparedSpan],
    by_span_id: &HashMap<(&str, &str), usize>,
    resolved: &mut HashMap<usize, Ownership>,
    visiting: &mut HashSet<usize>,
) -> Result<Ownership> {
    if let Some(value) = resolved.get(&index) {
        return Ok(value.clone());
    }
    if !visiting.insert(index) {
        return Err(format!(
            "Cycle in trajectory span parents at {}",
            spans[index].id
        ));
    }
    let span = &spans[index];
    let mut result = Ownership::default();
    if let Some(parent) = span
        .source
        .span_parents
        .first()
        .and_then(|parent| by_span_id.get(&(span.root_span_id.as_str(), parent.as_str())))
    {
        result = ownership(*parent, spans, by_span_id, resolved, visiting)?;
        if spans[*parent].kind() == "tool" {
            result.tool = Some(*parent);
        }
    }
    if span.source.turn.is_some() {
        result.turn.clone_from(&span.source.turn);
    }
    if span.source.compaction.is_some() {
        result.compaction = Some(index);
    }
    result.skipped |= span.is_scorer();
    visiting.remove(&index);
    resolved.insert(index, result.clone());
    Ok(result)
}

#[cfg(test)]
fn assemble(
    spans: &[ImportedSpan],
    failures: &[ImportFailure],
    exclude_system: bool,
) -> Result<Vec<Trajectory>> {
    let mut stream = TrajectoryStream::with_failures(
        spans.iter().map(ImportedSpan::header_only).collect(),
        failures.to_vec(),
        exclude_system,
    )?;
    let mut collector = TrajectoryCollector::default();
    while let Some(id) = stream.pending_ids(1).first() {
        let span = spans
            .iter()
            .find(|span| span.header.id.as_ref() == Some(id))
            .unwrap();
        for event in stream.push(span.clone())? {
            collector.push(event)?;
        }
    }
    for event in stream.finish()? {
        collector.push(event)?;
    }
    collector.snapshot()
}
