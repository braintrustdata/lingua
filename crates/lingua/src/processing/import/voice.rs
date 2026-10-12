use crate::universal::trajectory::VoiceEvent;
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
    /// `metadata["voice.timeline_version"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_version: Option<u32>,
    /// `metadata["voice.pipeline"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<VoicePipeline>,
    /// `metadata["voice.timeline_origin_unix_ms"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_origin_unix_ms: Option<f64>,
    /// `metadata["voice.server_vad_silence_ms"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_vad_silence_ms: Option<f64>,
    /// `metadata["voice.timeline"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<LoggedTimelineChunk>,
    /// `metadata["voice.events"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<VoiceEvent>,
}

impl SpanVoice {
    pub fn is_empty(&self) -> bool {
        self.recordings.is_empty()
            && self.selections.is_empty()
            && self.timeline_version.is_none()
            && self.timeline.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoicePipeline {
    Cascaded,
    Realtime,
    HalfCascade,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedTimelineChunk {
    pub call_span_id: String,
    pub chunk_index: u32,
    #[serde(rename = "final")]
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedRecording {
    pub id: String,
    pub attachment: LoggedAttachment,
    pub duration_ms: Option<f64>,
    pub timeline: Option<LoggedTimeline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<RecordingState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<LoggedSource>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingState {
    Ready,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedSource {
    pub channel_index: u32,
    pub boundary: SourceBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceBoundary {
    SessionInput,
    TransportInput,
    ModelInput,
    LocalAudioOutput,
    TransportOutput,
    #[serde(other)]
    Other,
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
