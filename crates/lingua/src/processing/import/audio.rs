//! Voice metadata in audio trace schema v3, as the Braintrust JS SDK logs voice sessions.
//!
//! Scalar fields, including framework keys such as LiveKit's interruption flag, are read
//! from span headers. The session's `audio.*` maps are read from payloads, because
//! Brainstore headers can't carry maps of objects. Which framework traced a session is
//! decided by the trajectory builder. Parse errors are kept here and reported only for
//! voice traces.

use super::span::metadata_field;
use crate::serde_json as json;
use crate::universal::trajectory::SegmentState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Voice fields that can be read from a span header. Turn references are span ids.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VoiceHeader {
    pub schema_version: Option<String>,
    pub turn_id: Option<String>,
    pub reply_to: Option<String>,
    /// `contrib.livekit.interrupted`.
    pub livekit_interrupted: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Voice fields that are read from a span payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoicePayload {
    pub session: Option<SessionAudio>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionAudio {
    pub origin_unix_ms: Option<f64>,
    pub sources: BTreeMap<String, AudioSourceEntry>,
    pub segments: BTreeMap<String, SegmentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSourceEntry {
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentEntry {
    pub state: SegmentState,
    pub start_offset_ms: u64,
    pub duration_ms: Option<u64>,
    pub attachment: Option<AttachmentEntry>,
    pub mime_type: Option<String>,
    #[serde(default)]
    pub channels: BTreeMap<String, ChannelEntry>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentEntry {
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEntry {
    pub sources: Vec<String>,
}

#[derive(Default, Deserialize)]
struct Metadata {
    #[serde(rename = "audio.schema.version")]
    schema_version: Option<json::Value>,
    #[serde(rename = "turn.id")]
    turn_id: Option<json::Value>,
    #[serde(rename = "turn.reply_to")]
    reply_to: Option<json::Value>,
    #[serde(rename = "audio.timeline")]
    timeline: Option<json::Value>,
    #[serde(rename = "audio.sources")]
    sources: Option<json::Value>,
    #[serde(rename = "audio.segments")]
    segments: Option<json::Value>,
    #[serde(rename = "contrib.livekit.interrupted")]
    interrupted: Option<json::Value>,
}

#[derive(Deserialize)]
struct Timeline {
    origin_unix_ms: f64,
}

pub(super) fn parse(metadata: Option<&json::Value>) -> (VoiceHeader, VoicePayload) {
    let mut header_errors = Vec::new();
    let mut errors = Vec::new();
    let metadata = match metadata.map(Metadata::deserialize).transpose() {
        Ok(metadata) => metadata.unwrap_or_default(),
        Err(error) => {
            errors.push(format!("Invalid voice metadata: {error}"));
            Metadata::default()
        }
    };
    let header = VoiceHeader {
        schema_version: metadata_field(
            metadata.schema_version,
            "audio.schema.version",
            &mut header_errors,
        ),
        turn_id: metadata_field(metadata.turn_id, "turn.id", &mut header_errors),
        reply_to: metadata_field(metadata.reply_to, "turn.reply_to", &mut header_errors),
        livekit_interrupted: metadata_field(
            metadata.interrupted,
            "contrib.livekit.interrupted",
            &mut header_errors,
        ),
        errors: header_errors,
    };
    let session = (metadata.timeline.is_some()
        || metadata.sources.is_some()
        || metadata.segments.is_some())
    .then(|| SessionAudio {
        origin_unix_ms: metadata_field::<Timeline>(
            metadata.timeline,
            "audio.timeline",
            &mut errors,
        )
        .map(|timeline| timeline.origin_unix_ms),
        sources: metadata_field(metadata.sources, "audio.sources", &mut errors).unwrap_or_default(),
        segments: metadata_field(metadata.segments, "audio.segments", &mut errors)
            .unwrap_or_default(),
    });
    let payload = VoicePayload { session, errors };
    (header, payload)
}
