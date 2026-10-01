//! Read one LiveKit trace into recordings, utterances, and text messages.

use crate::serde_json::Value;
use crate::universal::{AssistantContent, Message, UserContent};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{BTreeMap, HashMap};
use ts_rs::TS;

#[derive(Debug, Deserialize)]
pub struct VoiceSpan {
    pub span_id: String,
    pub span_parents: Option<Vec<String>>,
    span_attributes: Option<Attributes>,
    metrics: Option<Metrics>,
    metadata: Option<Value>,
    input: Option<Value>,
    output: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct Attributes {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Metrics {
    start: Option<f64>,
    end: Option<f64>,
}

#[derive(Default, Deserialize)]
struct Metadata {
    #[serde(rename = "lk.pii.user_transcript")]
    user_text: Option<String>,
    #[serde(rename = "lk.pii.response.text")]
    agent_text: Option<String>,
    interrupted: Option<bool>,
    #[serde(rename = "lk.interrupted")]
    turn_interrupted: Option<bool>,
}

#[derive(Default, Deserialize)]
struct ContentFields {
    audio: Option<Value>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct TraceMessage {
    role: TraceRole,
    content: Option<UserContent>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TraceRole {
    User,
    Assistant,
    System,
    Developer,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum VoiceAttachment {
    #[serde(rename = "braintrust_attachment")]
    Braintrust {
        filename: String,
        content_type: String,
        key: String,
    },
    #[serde(rename = "external_attachment")]
    External {
        filename: String,
        content_type: String,
        url: String,
    },
}

#[skip_serializing_none]
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Recording {
    pub span_id: String,
    pub attachment: VoiceAttachment,
    #[ts(optional)]
    pub start_ms: Option<f64>,
    #[ts(optional)]
    pub end_ms: Option<f64>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct RecordingRange {
    pub recording_span_id: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Speaker {
    User,
    Agent,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Utterance {
    pub span_id: String,
    pub speaker: Speaker,
    #[ts(optional)]
    pub text: Option<String>,
    #[ts(optional)]
    pub clip: Option<VoiceAttachment>,
    #[ts(optional)]
    pub start_ms: Option<f64>,
    #[ts(optional)]
    pub end_ms: Option<f64>,
    #[ts(optional)]
    pub interrupted: Option<bool>,
    #[ts(optional)]
    pub range: Option<RecordingRange>,
}

#[derive(Debug, Default, Serialize, TS)]
#[ts(export)]
pub struct VoiceCall {
    pub recordings: Vec<Recording>,
    pub utterances: Vec<Utterance>,
    pub messages: Vec<Message>,
}

impl VoiceSpan {
    fn name(&self) -> &str {
        self.span_attributes
            .as_ref()
            .and_then(|a| a.name.as_deref())
            .unwrap_or("")
    }

    fn parent(&self) -> Option<&str> {
        self.span_parents
            .as_ref()
            .and_then(|parents| parents.first())
            .map(String::as_str)
    }

    fn start(&self) -> Option<f64> {
        self.metrics.as_ref().and_then(|m| m.start)
    }

    fn end(&self) -> Option<f64> {
        self.metrics.as_ref().and_then(|m| m.end)
    }
}

/// Times are epoch milliseconds; ranges are milliseconds from recording start.
/// Missing trace data stays absent. Unrecognized traces return empty results.
pub fn import_voice_call(rows: &[VoiceSpan]) -> Result<VoiceCall, String> {
    let has_session = rows
        .iter()
        .any(|row| matches!(row.name(), "agent_session" | "livekit_agent_session"));
    let mut has_livekit_metadata = false;
    for row in rows {
        if let Some(value) = &row.metadata {
            let keys = BTreeMap::<String, Value>::deserialize(value).map_err(|error| {
                format!("Invalid voice metadata in span {}: {error}", row.span_id)
            })?;
            has_livekit_metadata |= keys.keys().any(|key| key.starts_with("lk."));
        }
    }
    if !has_session && !has_livekit_metadata {
        return Ok(VoiceCall::default());
    }
    let metadata: Vec<Metadata> = rows
        .iter()
        .map(|row| {
            row.metadata
                .as_ref()
                .map_or_else(|| Ok(Metadata::default()), Metadata::deserialize)
                .map_err(|error| format!("Invalid voice metadata in span {}: {error}", row.span_id))
        })
        .collect::<Result<_, _>>()?;
    let mut call = VoiceCall::default();
    for row in rows {
        let attachment = if row.name() == "call_recording" {
            audio(row.output.as_ref())?
        } else if row.parent().is_none() {
            audio(fields(row.input.as_ref())?.audio.as_ref())?
                .or(audio(fields(row.output.as_ref())?.audio.as_ref())?)
        } else {
            None
        };
        if let Some(attachment) = attachment {
            let bounded = row.name() == "call_recording";
            call.recordings.push(Recording {
                span_id: row.span_id.clone(),
                attachment,
                start_ms: bounded.then(|| row.start()).flatten().map(ms),
                end_ms: bounded.then(|| row.end()).flatten().map(ms),
            });
        }
    }
    let by_id: HashMap<&str, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| (row.span_id.as_str(), i))
        .collect();
    let only_child = |parent: &VoiceSpan, name: &str| {
        let mut children = rows
            .iter()
            .filter(|row| row.parent() == Some(parent.span_id.as_str()) && row.name() == name);
        children.next().is_some() && children.next().is_none()
    };
    let mut bounded = call
        .recordings
        .iter()
        .filter(|r| r.start_ms.is_some() && r.end_ms.is_some());
    let recording = bounded.next().filter(|_| bounded.next().is_none());
    let mut found = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let speaker = match row.name() {
            "user_speaking" | "user_turn" => Speaker::User,
            "agent_speaking" => Speaker::Agent,
            _ => continue,
        };
        let speaking = row.name() != "user_turn";
        let turn = if speaking {
            row.parent()
                .and_then(|id| by_id.get(id))
                .copied()
                .filter(|&i| {
                    rows[i].name()
                        == if speaker == Speaker::User {
                            "user_turn"
                        } else {
                            "agent_turn"
                        }
                        && only_child(&rows[i], row.name())
                })
        } else {
            Some(index)
        };
        let content = fields(if speaker == Speaker::User {
            row.input.as_ref()
        } else {
            row.output.as_ref()
        })?;
        let mut text = if speaker == Speaker::Agent {
            clean(content.text.as_deref())
        } else {
            None
        };
        if text.is_none() {
            if let Some(i) = turn {
                text = if speaker == Speaker::User {
                    role_text(rows[i].input.as_ref(), TraceRole::User)?
                        .or_else(|| clean(metadata[i].user_text.as_deref()))
                } else {
                    role_text(rows[i].output.as_ref(), TraceRole::Assistant)?
                        .or_else(|| clean(metadata[i].agent_text.as_deref()))
                };
            }
        }
        if !speaking && (text.is_none() || only_child(row, "user_speaking")) {
            continue;
        }
        let start_ms = speaking.then(|| row.start()).flatten().map(ms);
        let end_ms = speaking.then(|| row.end()).flatten().map(ms);
        let range = recording.and_then(|r| {
            let recording_start = r.start_ms?;
            let start = start_ms?.max(recording_start);
            let end = end_ms?.min(r.end_ms?);
            (end > start).then(|| RecordingRange {
                recording_span_id: r.span_id.clone(),
                start_ms: start - recording_start,
                end_ms: end - recording_start,
            })
        });
        found.push((
            row.start(),
            Utterance {
                span_id: row.span_id.clone(),
                speaker,
                text,
                clip: if speaking {
                    audio(content.audio.as_ref())?
                } else {
                    None
                },
                start_ms,
                end_ms,
                range,
                interrupted: if speaker == Speaker::Agent {
                    metadata[index]
                        .interrupted
                        .or_else(|| turn.and_then(|i| metadata[i].turn_interrupted))
                } else {
                    None
                },
            },
        ));
    }
    found.sort_by(|(a, _), (b, _)| {
        a.unwrap_or(f64::INFINITY)
            .total_cmp(&b.unwrap_or(f64::INFINITY))
    });
    call.utterances = found.into_iter().map(|(_, utterance)| utterance).collect();
    call.messages = call
        .utterances
        .iter()
        .filter_map(|u| {
            let text = u.text.clone()?;
            Some(match u.speaker {
                Speaker::User => Message::User {
                    content: UserContent::String(text),
                },
                Speaker::Agent => Message::Assistant {
                    content: AssistantContent::String(text),
                    id: None,
                },
            })
        })
        .collect();
    Ok(call)
}

fn ms(seconds: f64) -> f64 {
    seconds * 1000.0
}

fn clean(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn fields(value: Option<&Value>) -> Result<ContentFields, String> {
    match value.filter(|value| value.is_object()) {
        Some(value) => ContentFields::deserialize(value)
            .map_err(|error| format!("Invalid voice content: {error}")),
        None => Ok(ContentFields::default()),
    }
}

fn role_text(value: Option<&Value>, role: TraceRole) -> Result<Option<String>, String> {
    let Some(value) = value.filter(|value| value.is_array()) else {
        return Ok(None);
    };
    let messages = Vec::<TraceMessage>::deserialize(value)
        .map_err(|error| format!("Invalid voice turn messages: {error}"))?;
    let text = messages
        .into_iter()
        .filter(|message| message.role == role)
        .filter_map(|message| match message.content {
            Some(UserContent::String(text)) => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok(clean(Some(&text)))
}

fn audio(value: Option<&Value>) -> Result<Option<VoiceAttachment>, String> {
    #[derive(Deserialize)]
    struct AttachmentKind {
        #[serde(rename = "type")]
        kind: Option<String>,
        content_type: Option<String>,
    }
    let Some(value) = value.filter(|value| value.is_object()) else {
        return Ok(None);
    };
    let kind = AttachmentKind::deserialize(value)
        .map_err(|error| format!("Invalid voice attachment: {error}"))?;
    if !matches!(
        kind.kind.as_deref(),
        Some("braintrust_attachment" | "external_attachment")
    ) || !kind
        .content_type
        .is_some_and(|mime| mime.starts_with("audio/"))
    {
        return Ok(None);
    }
    VoiceAttachment::deserialize(value)
        .map(Some)
        .map_err(|error| format!("Invalid voice audio attachment: {error}"))
}

#[cfg(test)]
mod tests;
