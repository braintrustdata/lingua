use crate::universal::trajectory::{Speaker, SpeechState, TimingSource, TranscriptSource};
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
    /// `metadata["voice.speech"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech: Option<LoggedSpeech>,
}

/// One utterance, logged on its own span. Span ids refer to spans in the same trace and offsets
/// are milliseconds on the call's recording timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedSpeech {
    pub speaker: Speaker,
    pub turn_id: Option<String>,
    pub state: SpeechState,
    pub timing_source: TimingSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heard_by: Vec<String>,
    pub produced_by: Option<String>,
    pub interrupted_by: Option<String>,
    pub played_ms: Option<f64>,
    pub provider_played_ms: Option<f64>,
    pub end_of_turn_ms: Option<f64>,
    pub transcript: Option<LoggedTranscript>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedTranscript {
    pub source: TranscriptSource,
    #[serde(rename = "final")]
    pub is_final: bool,
    pub available_ms: Option<f64>,
    pub confidence: Option<f64>,
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
