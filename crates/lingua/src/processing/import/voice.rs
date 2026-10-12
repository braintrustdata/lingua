use crate::universal::trajectory::{
    VoiceMessageState, VoiceTextSource, VoiceTextStatus, VoiceTimingSource,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// `metadata["voice.origin_unix_ms"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_unix_ms: Option<f64>,
    /// `metadata["voice.messages"]`, without messages that failed validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<BTreeMap<String, LoggedVoiceMessage>>,
    /// `metadata["voice.context_message_ids"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_message_ids: Option<Vec<Option<String>>>,
    /// `metadata["voice.output_message_id"]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_message_id: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggedRole {
    User,
    Assistant,
}

/// One merged entry of `metadata["voice.messages"]`. `turn_span_id` and `source_ids` are ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedVoiceMessage {
    pub role: LoggedRole,
    pub seq: u64,
    pub committed: bool,
    pub state: VoiceMessageState,
    pub text: Option<String>,
    pub text_status: VoiceTextStatus,
    pub text_source: Option<VoiceTextSource>,
    pub generated_text: Option<String>,
    pub start_ms: Option<f64>,
    pub end_ms: Option<f64>,
    pub timing_source: Option<VoiceTimingSource>,
    pub end_of_turn_ms: Option<f64>,
    pub transcript_final_ms: Option<f64>,
    pub interrupted_at_ms: Option<f64>,
    pub provider_audio_end_ms: Option<f64>,
    pub selections: Vec<LoggedSelection>,
    pub produced_by_span_id: Option<String>,
}

impl LoggedVoiceMessage {
    /// Checks one merged entry of a call span. Returns warnings, or the reason to drop it.
    pub(super) fn validate(
        &self,
        call_span_id: Option<&str>,
        recordings: &[LoggedRecording],
    ) -> Result<Vec<String>, String> {
        use VoiceMessageState::{Complete, Interrupted, Unplayed};
        use VoiceTextStatus::{Final, Missing, NotCaptured, Partial};
        let user = self.role == LoggedRole::User;
        match (&self.text, self.text_status) {
            (Some(_), Missing | NotCaptured) => {
                return Err("has text but its text_status says there is none".to_string())
            }
            (None, Final | Partial) => {
                return Err("has no text but its text_status says there is".to_string())
            }
            _ => {}
        }
        let channel = if user { 0 } else { 1 };
        for selection in &self.selections {
            if selection.channel_index != Some(channel) {
                return Err(format!("has a selection off channel {channel}"));
            }
            if Some(selection.recording_span_id.as_str()) != call_span_id
                || !recordings
                    .iter()
                    .any(|recording| recording.id == selection.recording_id)
            {
                return Err("refers to a missing recording".to_string());
            }
        }
        if !self.committed && user && (self.state != Complete || self.text.is_some()) {
            return Err("is uncommitted user speech that is not complete or has text".to_string());
        }
        if self.state == Unplayed
            && (!self.selections.is_empty()
                || self.interrupted_at_ms.is_some()
                || (self.text.is_none() && self.text_status != NotCaptured))
        {
            return Err(
                "is unplayed but has audio, an interruption, or no generated text".to_string(),
            );
        }
        let mut warnings = Vec::new();
        let mismatched = if user {
            vec![
                ("generated_text", self.generated_text.is_some()),
                ("interrupted_at_ms", self.interrupted_at_ms.is_some()),
                (
                    "provider_audio_end_ms",
                    self.provider_audio_end_ms.is_some(),
                ),
                ("produced_by_span_id", self.produced_by_span_id.is_some()),
            ]
        } else {
            vec![
                ("end_of_turn_ms", self.end_of_turn_ms.is_some()),
                ("transcript_final_ms", self.transcript_final_ms.is_some()),
            ]
        };
        for (field, present) in mismatched {
            if present {
                warnings.push(format!("has {field}, which does not apply to its role"));
            }
        }
        if user && matches!(self.state, Interrupted | Unplayed) {
            warnings.push("is user speech with an agent-only state".to_string());
        }
        if !user && self.state == Interrupted && self.interrupted_at_ms.is_none() {
            warnings.push("is interrupted without interrupted_at_ms".to_string());
        }
        if let (Some(start), Some(end)) = (self.start_ms, self.end_ms) {
            if end < start {
                warnings.push("ends before it starts".to_string());
            }
        }
        Ok(warnings)
    }
}
