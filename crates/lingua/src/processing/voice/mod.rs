//! Read one LiveKit trace into recordings, utterances, and text messages.

use crate::serde_json::Value;
use crate::universal::{AssistantContent, Message, UserContent};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

pub mod adapters;
use adapters::Adapter;

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
    let Some(adapter) = adapter_for(rows)? else {
        return Ok(VoiceCall::default());
    };
    let recordings = adapter.recordings(rows)?;
    let utterances = adapter.utterances(rows)?;
    let messages = utterances
        .iter()
        .filter_map(|utterance| {
            let text = utterance.text.clone()?;
            Some(match utterance.speaker {
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
    Ok(VoiceCall {
        recordings,
        utterances,
        messages,
    })
}

fn adapter_for(rows: &[VoiceSpan]) -> Result<Option<&'static dyn Adapter>, String> {
    let adapter = &adapters::livekit::Livekit;
    if adapter.matches(rows)? {
        Ok(Some(adapter))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
