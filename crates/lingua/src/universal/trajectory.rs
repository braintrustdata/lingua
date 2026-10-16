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
    /// Call recordings logged on the root span, in logged order. Root scope only.
    #[ts(optional)]
    pub recordings: Option<Vec<AudioRecording>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AudioRecording {
    pub id: String,
    /// Span holding the audio attachment; absent while the recording is pending or omitted.
    #[ts(optional)]
    pub attachment_span_id: Option<String>,
    /// JSON pointer to the attachment in that span, like `/input/audio/call-0000`.
    #[ts(optional)]
    pub attachment_pointer: Option<String>,
    /// Segments of one recording share a group id.
    #[ts(optional)]
    pub group_id: Option<String>,
    #[ts(optional)]
    pub mime_type: Option<String>,
    #[ts(optional)]
    pub duration_ms: Option<f64>,
    #[ts(optional)]
    pub channel_count: Option<u32>,
    /// What each channel carries, like `session_input` or `local_audio_output`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<AudioChannel>>")]
    pub channels: Vec<AudioChannel>,
    /// Unix milliseconds of offset 0, when the recording places itself on the call clock.
    #[ts(optional)]
    pub origin_unix_ms: Option<f64>,
    /// `pending`, `ready`, or `omitted`.
    #[ts(optional)]
    pub state: Option<String>,
    #[ts(optional)]
    pub truncated: Option<bool>,
    #[ts(optional)]
    pub reason: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AudioChannel {
    #[ts(optional)]
    pub index: Option<u32>,
    pub boundary: String,
}

/// A stretch of one recording, in milliseconds from the start of that recording.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AudioSelection {
    pub recording_id: String,
    pub recording_span_id: String,
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
