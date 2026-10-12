use crate::{
    serde_json::{Map, Value},
    universal::{AssistantContent, ToolContent, UserContent},
    Message, UniversalParams, UniversalUsage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::vec::Vec;
use ts_rs::TS;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Trajectory {
    pub scope: Vec<Scope>,
    pub agent: Agent,
    #[ts(optional)]
    pub sections: Option<Vec<Section>>,
    #[ts(optional)]
    pub findings: Option<Vec<Finding>>,
    pub turns: Vec<Turn>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Map<String, Value>,
    #[ts(optional)]
    pub version: Option<String>,
    #[ts(optional)]
    pub voice_calls: Option<Vec<VoiceCall>>,
}

/// Audio of a voice call. Speech is ordered by span start and can overlap.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct VoiceCall {
    /// Id of the span that logged the recordings.
    pub id: String,
    pub recordings: Vec<AudioRecording>,
    pub speech: Vec<Speech>,
    /// The call's timeline events in `seq` order, when the integration logged a timeline.
    #[ts(optional)]
    pub events: Option<Vec<VoiceEvent>>,
}

/// One observation from a call timeline. `at_ms` is on the call clock: milliseconds since the
/// recordings' `origin_unix_ms`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct VoiceEvent {
    pub seq: u32,
    pub at_ms: f64,
    pub basis: TimeBasis,
    #[serde(flatten)]
    #[ts(flatten)]
    pub detail: VoiceEventDetail,
}

/// How a timeline timestamp was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TimeBasis {
    /// From sample positions on the recording.
    Audio,
    /// From the local wall clock when a hook ran.
    Local,
    /// From a provider-reported time converted to the call clock.
    Provider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptSource {
    Stt,
    Provider,
    Playout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MissingEvidence {
    Transcript,
    SpeechEnd,
    Audio,
    ToolResult,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VoiceEventDetail {
    SpeechStart {
        speaker: Speaker,
        utterance_id: String,
        #[ts(optional)]
        response_id: Option<String>,
    },
    SpeechEnd {
        speaker: Speaker,
        utterance_id: String,
        #[ts(optional)]
        interrupted: Option<bool>,
        #[ts(optional)]
        played_ms: Option<f64>,
    },
    Transcript {
        speaker: Speaker,
        utterance_id: String,
        #[serde(rename = "final")]
        is_final: bool,
        source: TranscriptSource,
        #[ts(optional)]
        text: Option<String>,
        #[ts(optional)]
        message_id: Option<String>,
    },
    EndOfTurn {
        utterance_ids: Vec<String>,
    },
    ResponseStart {
        response_id: String,
        span_id: String,
        #[ts(optional)]
        in_reply_to: Option<Vec<String>>,
    },
    ToolCall {
        response_id: String,
        tool_call_id: String,
        name: String,
        span_id: String,
    },
    ToolResult {
        tool_call_id: String,
        span_id: String,
        outcome: ToolOutcome,
    },
    Truncate {
        utterance_id: String,
        audio_end_ms: f64,
    },
    Missing {
        evidence: MissingEvidence,
        reason: String,
        #[ts(optional)]
        utterance_id: Option<String>,
    },
    CallEnd {
        reason: String,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AudioRecording {
    pub id: String,
    /// Id of the span holding the audio attachment.
    pub attachment_span_id: String,
    /// JSON pointer to the attachment in that span, like `/input/audio/call-0000`.
    pub attachment_pointer: String,
    #[ts(optional)]
    pub start_time: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub duration_ms: Option<f64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Speech {
    /// Id of the span that logged this speech.
    pub id: String,
    pub speaker: Speaker,
    /// Ids of the trajectory steps that this speech belongs to: for one utterance, the LLM call
    /// that produced agent speech or answered user speech.
    pub step_ids: Vec<String>,
    pub selections: Vec<AudioSelection>,
    #[ts(optional)]
    pub interrupted: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    User,
    Agent,
}

/// A stretch of one recording. Offsets are from the start of that recording.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AudioSelection {
    pub recording_id: String,
    #[ts(optional)]
    pub channel_index: Option<u32>,
    pub start_offset_ms: f64,
    pub end_offset_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scope {
    Span { id: String },
    Trace { trace_id: String },
    Thread { key: String, value: String },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Agent {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub version: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Map<String, Value>,
    #[ts(optional)]
    pub instructions: Option<UserContent>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Section {
    pub name: String,
    pub description: String,
    pub step_ids: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Finding {
    #[ts(optional)]
    pub title: Option<String>,
    #[ts(optional)]
    pub severity: Option<FindingSeverity>,
    #[ts(optional)]
    pub kind: Option<String>,
    #[ts(optional)]
    pub hypothesis: Option<String>,
    #[ts(optional)]
    pub root_cause: Option<String>,
    #[ts(optional)]
    pub next_steps: Option<Vec<String>>,
    #[ts(optional)]
    pub evidence: Option<Vec<FindingEvidence>>,
    #[ts(optional)]
    pub pattern_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FindingEvidence {
    pub step_id: String,
    #[ts(optional)]
    pub field: Option<EvidenceField>,
    #[ts(optional)]
    pub part: Option<EvidencePart>,
    #[ts(optional)]
    pub explanation: Option<String>,
    #[ts(optional)]
    pub quote: Option<String>,
    #[ts(optional)]
    pub highlight_terms: Option<Vec<String>>,
    #[ts(optional)]
    pub leading_text: Option<String>,
    #[ts(optional)]
    pub trailing_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceField {
    Input,
    Output,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePart {
    Reasoning,
    Arguments,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Turn {
    // Each of these is the id of the corresponding span
    pub request_id: String,
    #[ts(optional)]
    pub response_id: Option<String>,

    // Step definition
    #[ts(optional)]
    pub request: Option<Vec<Message>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
    pub opaque_request: Vec<OpaqueItem>,
    #[ts(optional)]
    pub response: Option<AgentResponse>,
    pub work: Vec<WorkStep>,

    #[ts(optional)]
    pub model: Option<String>,
    #[ts(optional)]
    pub params: Option<UniversalParams>,

    pub start_time: DateTime<Utc>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub interrupted: Option<bool>,
    #[ts(optional)]
    pub compaction: Option<Compaction>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Compaction {
    pub id: String,
    #[ts(optional)]
    pub replaced_message_count: Option<usize>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentResponse {
    #[ts(optional)]
    pub response: Option<AssistantContent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
    pub opaque_input: Vec<OpaqueItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
    pub opaque_output: Vec<OpaqueItem>,

    #[ts(optional)]
    pub usage: Option<UniversalUsage>,
    #[ts(optional)]
    pub start_time: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ToolResult {
    #[ts(optional, type = "unknown")]
    pub input: Option<Value>,
    #[ts(optional, type = "unknown")]
    pub output: Option<Value>,
    #[ts(optional)]
    pub content: Option<ToolContent>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct LLMAnalysis {
    #[ts(optional)]
    pub work: Option<Vec<Message>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
    pub opaque_input: Vec<OpaqueItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
    pub opaque_output: Vec<OpaqueItem>,
    #[ts(optional)]
    pub model: Option<String>,
    #[ts(optional)]
    pub params: Option<UniversalParams>,
    #[ts(optional)]
    pub usage: Option<UniversalUsage>,
}

/// Source data retained without interpreting it as a conversational message.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct OpaqueItem {
    /// Position in the source array, or None when the payload itself is opaque.
    pub index: Option<usize>,
    #[ts(type = "unknown")]
    pub value: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub is_metadata: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WorkStep {
    pub id: String,        // span's id
    pub span_type: String, // span type
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional, type = "unknown")]
    pub error: Option<Value>,

    pub start_time: DateTime<Utc>,
    #[ts(optional)]
    pub end_time: Option<DateTime<Utc>>,

    pub work: Work,
    #[ts(optional)]
    pub sub_agent: Option<Box<Trajectory>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Work {
    AgentResponse(Box<AgentResponse>),
    ToolResult(Box<ToolResult>),
    #[serde(rename = "llm_analysis")]
    LLMAnalysis(Box<LLMAnalysis>),
}
