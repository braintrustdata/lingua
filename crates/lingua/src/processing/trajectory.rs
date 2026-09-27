#[cfg(test)]
mod legacy;
mod stream;
#[cfg(test)]
mod tests;
pub use stream::{TrajectoryCollector, TrajectoryEvent, TrajectoryStream};

use crate::processing::{import_messages_from_spans, message_dedup_hash, Span};
use crate::serde_json as json;
use crate::universal::trajectory::{
    Agent, AgentResponse, Compaction, LLMAnalysis, Scope, ToolResult, Trajectory, Turn, Work,
    WorkStep,
};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, ToolContentPart, ToolResultContentPart,
    UserContent, UserContentPart,
};
use crate::UniversalUsage;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize)]
pub struct ImportFailure {
    #[serde(skip)]
    root_span_id: String,
    span_id: String,
    message: String,
}

impl ImportFailure {
    pub fn from_row<'de, D: serde::Deserializer<'de>>(row: D, message: String) -> Result<Self> {
        #[derive(Deserialize)]
        struct Identity {
            id: String,
            root_span_id: String,
        }
        let identity = Identity::deserialize(row)
            .map_err(|_| format!("Cannot identify failed trajectory span: {message}"))?;
        Ok(Self {
            root_span_id: identity.root_span_id,
            span_id: identity.id,
            message,
        })
    }
}

fn null_default<'de, T: Deserialize<'de> + Default, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<T, D::Error> {
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceSpan {
    id: String,
    root_span_id: String,
    #[serde(default)]
    span_id: Option<String>,
    #[serde(default, alias = "span__parents", deserialize_with = "null_default")]
    span_parents: Vec<String>,
    created: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "null_default")]
    metrics: Metrics,
    #[serde(default, deserialize_with = "null_default")]
    span_attributes: Attributes,
    metadata: Option<json::Value>,
    input: Option<json::Value>,
    output: Option<json::Value>,
    error: Option<json::Value>,
    model: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    tags: Vec<String>,
    #[serde(skip)]
    skipped: bool,
    #[serde(skip)]
    normalized: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Metrics {
    start: Option<f64>,
    end: Option<f64>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    tokens: Option<i64>,
    prompt_cached_tokens: Option<i64>,
    prompt_cache_creation_tokens: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Attributes {
    #[serde(rename = "type")]
    kind: Option<String>,
    name: Option<String>,
    purpose: Option<String>,
    #[serde(default)]
    exec_counter: i64,
}

#[derive(Default, Deserialize)]
struct Metadata {
    turn_id: Option<json::Value>,
    #[serde(default)]
    model: MetadataHint<String>,
    #[serde(default)]
    trajectory_role: MetadataHint<String>,
    #[serde(default)]
    request_kind: MetadataHint<String>,
    #[serde(default)]
    compaction: MetadataHint<CompactionMetadata>,
    #[serde(default)]
    tool_call_id: MetadataHint<String>,
}

struct MetadataHint<T>(std::result::Result<Option<T>, String>);

impl<T> Default for MetadataHint<T> {
    fn default() -> Self {
        Self(Ok(None))
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for MetadataHint<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = json::Value::deserialize(deserializer)?;
        Ok(Self(
            Option::<T>::deserialize(value).map_err(|err| err.to_string()),
        ))
    }
}

impl<T> MetadataHint<T> {
    fn read(self, field: &str, errors: &mut Vec<String>) -> Option<T> {
        match self.0 {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("Invalid metadata.{field}: {error}"));
                None
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CompactionMetadata {
    Flag(bool),
    Details {
        replaced_message_count: Option<usize>,
    },
}

#[derive(Deserialize)]
struct NormalizedSpan {
    id: String,
    root_span_id: String,
    input: Option<json::Value>,
    output: Option<json::Value>,
    error: Option<json::Value>,
    metadata: Option<json::Value>,
    span_attributes: Option<Attributes>,
}

impl SourceSpan {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn parse<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Self> {
        Self::deserialize(value)
            .map_err(|err| format!("Invalid trajectory source span: {err}").into())
    }

    pub fn normalize(mut self, value: Value) -> Result<Self> {
        if value.is_null() {
            self.input = None;
            self.output = None;
            self.skipped = true;
            return Ok(self);
        }
        let normalized: NormalizedSpan = serde_json::from_value(value).map_err(|err| {
            format!("Trajectory preprocessors must return a span record or null: {err}")
        })?;
        if normalized.id != self.id || normalized.root_span_id != self.root_span_id {
            return Err(
                format!("Trajectory preprocessors must preserve id and root_span_id").into(),
            );
        }
        self.input = normalized.input;
        self.output = normalized.output;
        self.error = normalized.error;
        self.metadata = normalized.metadata;
        self.normalized = true;
        if let Some(attributes) = normalized.span_attributes {
            self.span_attributes.kind = attributes.kind.or(self.span_attributes.kind);
            self.span_attributes.name = attributes.name.or(self.span_attributes.name);
            self.span_attributes.purpose = attributes.purpose.or(self.span_attributes.purpose);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct TrajectorySpan {
    source: SourceSpan,
    input: Vec<Message>,
    input_keys: Vec<u64>,
    output: Vec<Message>,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    turn: Option<String>,
    analysis: bool,
    compaction: Option<Compaction>,
    tool_result: Option<ToolResult>,
    analysis_messages: Option<Vec<Message>>,
    failure: Option<ImportFailure>,
}

fn timestamp(value: f64) -> Option<DateTime<Utc>> {
    if !value.is_finite() {
        return None;
    }
    DateTime::from_timestamp_micros((value * 1_000_000.0) as i64)
}

impl TrajectorySpan {
    pub fn new(mut source: SourceSpan) -> Result<Self> {
        let mut errors = Vec::new();
        let metadata: Metadata = source
            .metadata
            .as_ref()
            .map_or_else(
                || Ok(Metadata::default()),
                |value| Metadata::deserialize(value),
            )
            .unwrap_or_else(|err| {
                errors.push(format!("Invalid trajectory metadata: {err}"));
                Metadata::default()
            });
        let model = metadata.model.read("model", &mut errors);
        let trajectory_role = metadata
            .trajectory_role
            .read("trajectory_role", &mut errors);
        let request_kind = metadata.request_kind.read("request_kind", &mut errors);
        let analysis = trajectory_role.as_deref() == Some("analysis")
            || request_kind.as_deref() == Some("reviewer");
        let compaction = metadata.compaction.read("compaction", &mut errors);
        let tool_call_id = metadata.tool_call_id.read("tool_call_id", &mut errors);
        let start = source
            .metrics
            .start
            .and_then(timestamp)
            .or(source.created)
            .ok_or_else(|| {
                format!(
                    "Missing or invalid timestamp for trajectory span {}",
                    source.id
                )
            })?;
        let end = source
            .metrics
            .end
            .and_then(timestamp)
            .filter(|end| *end >= start);
        let import = |input: Option<json::Value>, output: Option<json::Value>| {
            let nonempty = |value: &json::Value| match value {
                json::Value::Array(items) => !items.is_empty(),
                json::Value::Object(fields) => !fields.is_empty(),
                _ => true,
            };
            import_messages_from_spans(vec![Span {
                input: input.filter(nonempty),
                output: output.filter(nonempty),
                other: json::Map::new(),
            }])
        };
        let tool_result = if source.span_attributes.kind.as_deref() == Some("tool") {
            let input = source.input.take();
            let output = source.output.take();
            if source.normalized {
                Some(ToolResult {
                    input,
                    content: Some(
                        output
                            .into_iter()
                            .map(|output| {
                                ToolContentPart::ToolResult(ToolResultContentPart {
                                    tool_call_id: tool_call_id
                                        .clone()
                                        .unwrap_or_else(|| source.id.clone()),
                                    tool_name: source
                                        .span_attributes
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| source.id.clone()),
                                    output,
                                    custom_tool_call: None,
                                    caller: None,
                                    provider_options: None,
                                })
                            })
                            .collect(),
                    ),
                })
            } else {
                None
            }
        } else {
            None
        };
        let input = import(source.input.take(), None);
        let output = import(None, source.output.take());
        let analysis_messages =
            (source.normalized && analysis).then(|| input.iter().chain(&output).cloned().collect());
        let input_keys = message_keys(&input);
        let input = current_input(&input).to_vec();
        let turn = metadata
            .turn_id
            .and_then(|value| match value {
                json::Value::String(value) if !value.is_empty() => Some(value),
                _ => None,
            })
            .or_else(|| {
                (source.span_attributes.kind.as_deref() == Some("task")
                    && source
                        .span_attributes
                        .name
                        .as_deref()
                        .is_some_and(|name| name.starts_with("turn: ")))
                .then(|| source.id.clone())
            });
        let is_compaction = matches!(
            compaction,
            Some(CompactionMetadata::Flag(true) | CompactionMetadata::Details { .. })
        ) || source.tags.iter().any(|tag| tag == "compaction")
            || (source.span_attributes.kind.as_deref() == Some("task")
                && source.span_attributes.name.as_deref() == Some("compaction"));
        let compaction = is_compaction.then(|| Compaction {
            id: source.id.clone(),
            replaced_message_count: match compaction {
                Some(CompactionMetadata::Details {
                    replaced_message_count,
                }) => replaced_message_count,
                _ => None,
            },
        });
        source.model = model.or(source.model);
        source.metadata = None;
        let failure = (!errors.is_empty()).then(|| ImportFailure {
            root_span_id: source.root_span_id.clone(),
            span_id: source.id.clone(),
            message: errors.join("; "),
        });
        Ok(Self {
            source,
            input,
            input_keys,
            output,
            start,
            end,
            turn,
            analysis,
            compaction,
            tool_result,
            analysis_messages,
            failure,
        })
    }

    fn kind(&self) -> &str {
        self.source
            .span_attributes
            .kind
            .as_deref()
            .unwrap_or("task")
    }

    fn is_scorer(&self) -> bool {
        self.kind() == "score" || self.source.span_attributes.purpose.as_deref() == Some("scorer")
    }

    fn usage(&self) -> Option<UniversalUsage> {
        let metrics = &self.source.metrics;
        if metrics.prompt_tokens.is_none()
            && metrics.completion_tokens.is_none()
            && metrics.tokens.is_none()
        {
            return None;
        }
        Some(UniversalUsage {
            prompt_tokens: metrics.prompt_tokens,
            completion_tokens: metrics.completion_tokens,
            total_tokens: metrics.tokens,
            prompt_cached_tokens: metrics.prompt_cached_tokens,
            prompt_cache_creation_tokens: metrics.prompt_cache_creation_tokens,
            ..Default::default()
        })
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
            end_time: self.end,
        }
    }

    fn can_finish_turn(&self) -> bool {
        matches!(self.kind(), "llm" | "task") && !self.analysis && self.end.is_some() && self.source.error.is_none()
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

fn user_text(content: &UserContent) -> Option<String> {
    match content {
        UserContent::String(text) => Some(text.clone()),
        UserContent::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                let UserContentPart::Text(part) = part else {
                    return None;
                };
                text.push_str(&part.text);
            }
            Some(text)
        }
    }
}

fn is_context(message: &Message) -> bool {
    match message {
        Message::System { .. } | Message::Developer { .. } | Message::AdditionalTools { .. } => {
            true
        }
        Message::User { content } => {
            let Some(text) = user_text(content) else {
                return false;
            };
            let text = text.trim();
            let Some((tag, _)) = text.strip_prefix('<').and_then(|text| text.split_once('>'))
            else {
                return false;
            };
            let is_context_tag = matches!(tag, "environment_context" | "braintrust.runtime")
                || tag.starts_with("external_braintrust.")
                    && tag
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '.');
            is_context_tag && text.ends_with(&format!("</{tag}>"))
        }
        _ => false,
    }
}

fn current_input(input: &[Message]) -> &[Message] {
    let start = input
        .iter()
        .rposition(|message| matches!(message, Message::Assistant { .. } | Message::Tool { .. }))
        .map_or(0, |index| index + 1);
    &input[start..]
}

fn interrupts_previous_turn(input: &[Message]) -> bool {
    input.iter().any(|message| {
        let (Message::System { content } | Message::Developer { content }) = message else {
            return false;
        };
        user_text(content).is_some_and(|text| {
            text.trim()
                .strip_prefix("<turn_aborted>")
                .and_then(|text| text.strip_suffix("</turn_aborted>"))
                .is_some_and(|text| !text.is_empty())
        })
    })
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
    spans: &[TrajectorySpan],
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
            spans[index].source.id
        )
        .into());
    }
    let span = &spans[index];
    let mut result = Ownership::default();
    if let Some(parent) =
        span.source.span_parents.first().and_then(|parent| {
            by_span_id.get(&(span.source.root_span_id.as_str(), parent.as_str()))
        })
    {
        result = ownership(*parent, spans, by_span_id, resolved, visiting)?;
        if spans[*parent].kind() == "tool" && !spans[*parent].source.skipped {
            result.tool = Some(*parent);
        }
    }
    if span.turn.is_some() {
        result.turn.clone_from(&span.turn);
    }
    if span.compaction.is_some() && !span.source.skipped {
        result.compaction = Some(index);
    }
    result.skipped |= span.is_scorer();
    visiting.remove(&index);
    resolved.insert(index, result.clone());
    Ok(result)
}

pub fn assemble(
    spans: &[TrajectorySpan],
    failures: &[ImportFailure],
    exclude_system: bool,
) -> Result<Vec<Trajectory>> {
    let mut stream =
        TrajectoryStream::from_normalized(spans.to_vec(), failures.to_vec(), exclude_system)?;
    let mut collector = TrajectoryCollector::default();
    for event in stream.finish()? {
        collector.push(event)?;
    }
    collector.snapshot()
}
