use serde::{Deserialize, Serialize};

/// Voice metadata logged on a span, kept on its header so the trajectory can place speech on audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpanVoice {
    /// `metadata["audio.recordings"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recordings: Vec<LoggedRecording>,
    /// `metadata["audio.selections"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selections: Vec<LoggedSelection>,
    /// `metadata["contrib.livekit.interrupted"]`
    pub interrupted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedRecording {
    pub id: String,
    pub attachment: LoggedAttachment,
    pub duration_ms: Option<f64>,
    pub timeline: Option<LoggedTimeline>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedAttachment {
    #[serde(rename = "ref")]
    pub pointer: String,
    pub span_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedTimeline {
    pub origin_unix_ms: f64,
    pub recording_start_offset_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedSelection {
    pub recording_id: String,
    pub recording_span_id: String,
    pub channel_index: Option<u32>,
    pub start_offset_ms: f64,
    pub end_offset_ms: f64,
}
