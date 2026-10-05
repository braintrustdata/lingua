use super::{import_span_messages, is_instruction, ImportOptions, OpaqueItem, Span};
use crate::serde_json as json;
use crate::universal::trajectory::{Compaction, ToolResult};
use crate::universal::{
    Message, ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use crate::UniversalUsage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

type Result<T> = std::result::Result<T, String>;

pub const TRAJECTORY_METADATA_FIELDS: &[&str] = &[
    "turn_id",
    "tool_call_id",
    "model",
    "trajectory_role",
    "request_kind",
    "compaction",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    pub id: Option<String>,
    pub root_span_id: Option<String>,
    pub span_id: Option<String>,
    #[serde(default, alias = "span__parents", deserialize_with = "null_default")]
    pub span_parents: Vec<String>,
    #[serde(default)]
    pub kind: String,
    pub name: Option<String>,
    #[serde(default)]
    pub exec_counter: i64,
    #[serde(default)]
    pub scorer: bool,
    pub model: Option<String>,
    pub error: Option<json::Value>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub turn: Option<String>,
    #[serde(default)]
    pub analysis: bool,
    pub compaction: Option<Compaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedSpan {
    pub header: SpanContext,
    #[serde(default)]
    pub input: Vec<Message>,
    #[serde(default)]
    pub output: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque_input: Vec<OpaqueItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque_output: Vec<OpaqueItem>,
    pub usage: Option<UniversalUsage>,
    pub tool_result: Option<ToolResult>,
    #[serde(default)]
    pub context_messages: Vec<usize>,
    #[serde(default)]
    pub interruption_messages: Vec<usize>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl ImportedSpan {
    pub fn header_only(&self) -> Self {
        Self {
            header: self.header.clone(),
            input: Vec::new(),
            output: Vec::new(),
            opaque_input: Vec::new(),
            opaque_output: Vec::new(),
            usage: None,
            tool_result: None,
            context_messages: Vec::new(),
            interruption_messages: Vec::new(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }
}

fn null_default<'de, T: Deserialize<'de> + Default, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<T, D::Error> {
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Deserialize)]
struct SpanFields {
    #[serde(flatten)]
    header: SpanContext,
    created: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "null_default")]
    metrics: Metrics,
    #[serde(default, deserialize_with = "null_default")]
    span_attributes: Attributes,
    metadata: Option<json::Value>,
    #[serde(default, deserialize_with = "null_default")]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Metrics {
    start: Option<f64>,
    end: Option<f64>,
    #[serde(flatten)]
    usage: Option<UniversalUsage>,
    tokens: Option<i64>,
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
    tool_call_id: Option<json::Value>,
    model: Option<json::Value>,
    trajectory_role: Option<json::Value>,
    request_kind: Option<json::Value>,
    compaction: Option<json::Value>,
}

fn metadata_field<T: serde::de::DeserializeOwned>(
    value: Option<json::Value>,
    field: &str,
    errors: &mut Vec<String>,
) -> Option<T> {
    match value.map(T::deserialize).transpose() {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("Invalid metadata.{field}: {error}"));
            None
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CompactionHint {
    Flag(bool),
    Details { replaced_message_count: usize },
}

pub fn import_span(span: Span) -> Result<ImportedSpan> {
    import_span_with_options(span, ImportOptions::default())
}

pub fn import_span_with_options(mut span: Span, options: ImportOptions) -> Result<ImportedSpan> {
    let mut source = SpanFields::deserialize(&json::Value::Object(std::mem::take(&mut span.other)))
        .map_err(|error| format!("Invalid span fields: {error}"))?;
    let mut errors = Vec::new();
    let metadata: Metadata = source
        .metadata
        .as_ref()
        .map_or_else(|| Ok(Metadata::default()), Metadata::deserialize)
        .unwrap_or_else(|err| {
            errors.push(format!("Invalid span metadata: {err}"));
            Metadata::default()
        });
    let tool_call_id = metadata_field::<String>(metadata.tool_call_id, "tool_call_id", &mut errors);
    let model = metadata_field::<String>(metadata.model, "model", &mut errors);
    let trajectory_role =
        metadata_field::<String>(metadata.trajectory_role, "trajectory_role", &mut errors);
    let request_kind = metadata_field::<String>(metadata.request_kind, "request_kind", &mut errors);
    let analysis = trajectory_role.as_deref() == Some("analysis")
        || request_kind.as_deref() == Some("reviewer");
    let compaction =
        metadata_field::<CompactionHint>(metadata.compaction, "compaction", &mut errors);
    let start = source.metrics.start.and_then(timestamp).or(source.created);
    let end = source
        .metrics
        .end
        .and_then(timestamp)
        .filter(|end| start.is_none_or(|start| *end >= start));
    let tool_result = if source.span_attributes.kind.as_deref() == Some("tool") {
        let mut output = span.output.take();
        let content = tool_call_id.and_then(|tool_call_id| {
            output.take().map(|output| {
                vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id,
                    // Match provider imports: an empty name means unknown; the call ID still pairs the result.
                    tool_name: source.span_attributes.name.clone().unwrap_or_default(),
                    output,
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })]
            })
        });
        Some(ToolResult {
            input: span.input.take(),
            output,
            content,
        })
    } else {
        None
    };
    let is_compaction = matches!(
        compaction,
        Some(CompactionHint::Flag(true) | CompactionHint::Details { .. })
    ) || source.tags.iter().any(|tag| tag == "compaction")
        || (source.span_attributes.kind.as_deref() == Some("task")
            && source.span_attributes.name.as_deref() == Some("compaction"));
    let (compaction_input, compaction_output) = if options.preserve_unsupported && is_compaction {
        (span.input.take(), span.output.take())
    } else {
        (None, None)
    };
    let super::SpanMessages {
        input,
        output,
        mut opaque_input,
        mut opaque_output,
        errors: message_errors,
        warnings,
    } = import_span_messages(
        span.input,
        span.output,
        source.metadata.as_ref(),
        source.span_attributes.kind.as_deref() == Some("llm"),
        options,
    );
    opaque_input.extend(compaction_input.map(|value| OpaqueItem { index: None, value }));
    opaque_output.extend(compaction_output.map(|value| OpaqueItem { index: None, value }));
    errors.extend(message_errors);
    let context_messages = input
        .iter()
        .enumerate()
        .filter_map(|(index, message)| is_context(message).then_some(index))
        .collect();
    let interruption_messages = input
        .iter()
        .enumerate()
        .filter_map(|(index, message)| is_interruption(message).then_some(index))
        .collect();
    let turn = metadata_field::<String>(metadata.turn_id, "turn_id", &mut errors)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            (source.span_attributes.kind.as_deref() == Some("task")
                && source
                    .span_attributes
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with("turn: ")))
            .then(|| source.header.id.clone())
            .flatten()
        });
    let compaction = source
        .header
        .id
        .clone()
        .filter(|_| is_compaction)
        .map(|id| Compaction {
            id,
            replaced_message_count: match compaction {
                Some(CompactionHint::Details {
                    replaced_message_count,
                }) => Some(replaced_message_count),
                _ => None,
            },
        });
    source.header.model = model.or(source.header.model);
    source.header.start = start;
    source.header.end = end;
    source.header.turn = turn;
    source.header.analysis = analysis;
    source.header.compaction = compaction;
    source.header.kind = source
        .span_attributes
        .kind
        .unwrap_or_else(|| "task".to_string());
    source.header.name = source.span_attributes.name;
    source.header.exec_counter = source.span_attributes.exec_counter;
    source.header.scorer = source.span_attributes.purpose.as_deref() == Some("scorer");
    let mut usage = source.metrics.usage.take();
    if let Some(usage) = &mut usage {
        usage.total_tokens = source.metrics.tokens.or(usage.total_tokens);
    }
    Ok(ImportedSpan {
        header: source.header,
        input,
        output,
        opaque_input,
        opaque_output,
        usage: usage.filter(|usage| usage != &UniversalUsage::default()),
        tool_result,
        context_messages,
        interruption_messages,
        errors,
        warnings,
    })
}

fn timestamp(value: f64) -> Option<DateTime<Utc>> {
    if !value.is_finite() {
        return None;
    }
    DateTime::from_timestamp_micros((value * 1_000_000.0) as i64)
}

fn user_text(content: &UserContent) -> Option<Cow<'_, str>> {
    match content {
        UserContent::String(text) => Some(Cow::Borrowed(text)),
        UserContent::Array(parts) => {
            let mut text = Cow::Borrowed("");
            for part in parts {
                let UserContentPart::Text(part) = part else {
                    return None;
                };
                if text.is_empty() {
                    text = Cow::Borrowed(part.text.as_str());
                } else {
                    text.to_mut().push_str(&part.text);
                }
            }
            Some(text)
        }
    }
}

fn is_context(message: &Message) -> bool {
    match message {
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
        _ => is_instruction(message),
    }
}

fn is_interruption(message: &Message) -> bool {
    let (Message::System { content } | Message::Developer { content }) = message else {
        return false;
    };
    user_text(content).is_some_and(|text| {
        text.trim()
            .strip_prefix("<turn_aborted>")
            .and_then(|text| text.strip_suffix("</turn_aborted>"))
            .is_some_and(|text| !text.is_empty())
    })
}
