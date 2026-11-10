//! Voice metadata in audio trace schema v3. The `audio.*` maps come from payloads because
//! Brainstore headers can't carry maps of objects. Parse errors stay out of the span's
//! errors because only voice traces report them.

use super::span::metadata_field;
use crate::serde_json as json;
use crate::universal::trajectory::{
    AudioSource, Channel, Recording, RecordingSegment, SegmentState,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VoiceContext {
    /// Kept even when invalid, because any version marks the span as a voice session.
    pub schema_version: Option<Result<String, String>>,
    pub turn_id: Option<String>,
    pub reply_to: Option<String>,
    pub interrupted: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoicePayload {
    pub recording: Option<Recording>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Deserialize)]
struct AudioSourceEntry {
    label: Option<String>,
}

#[derive(Deserialize)]
struct SegmentEntry {
    state: SegmentState,
    start_offset_ms: u64,
    duration_ms: Option<u64>,
    attachment: Option<AttachmentEntry>,
    mime_type: Option<String>,
    #[serde(default)]
    channels: BTreeMap<String, ChannelEntry>,
    reason: Option<String>,
}

#[derive(Deserialize)]
struct AttachmentEntry {
    #[serde(rename = "ref")]
    reference: String,
}

#[derive(Deserialize)]
struct ChannelEntry {
    sources: Vec<String>,
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

pub(super) fn parse(
    span_id: Option<&str>,
    metadata: Option<&json::Value>,
) -> (VoiceContext, VoicePayload) {
    let mut header_errors = Vec::new();
    let mut errors = Vec::new();
    let metadata = match metadata.map(Metadata::deserialize).transpose() {
        Ok(metadata) => metadata.unwrap_or_default(),
        Err(error) => {
            errors.push(format!("Invalid voice metadata: {error}"));
            Metadata::default()
        }
    };
    let header = VoiceContext {
        schema_version: metadata.schema_version.map(|value| {
            String::deserialize(value)
                .map_err(|error| format!("Invalid metadata.audio.schema.version: {error}"))
        }),
        turn_id: metadata_field(metadata.turn_id, "turn.id", &mut header_errors),
        reply_to: metadata_field(metadata.reply_to, "turn.reply_to", &mut header_errors),
        interrupted: metadata_field(
            metadata.interrupted,
            "contrib.livekit.interrupted",
            &mut header_errors,
        ),
        errors: header_errors,
    };
    let recording = match (span_id, &header.schema_version) {
        (Some(span_id), Some(_)) => recording(
            span_id,
            metadata.timeline,
            metadata.sources,
            metadata.segments,
            &mut errors,
        ),
        _ => None,
    };
    let payload = VoicePayload { recording, errors };
    (header, payload)
}

fn recording(
    span_id: &str,
    timeline: Option<json::Value>,
    sources: Option<json::Value>,
    segments: Option<json::Value>,
    errors: &mut Vec<String>,
) -> Option<Recording> {
    if timeline.is_none() && sources.is_none() && segments.is_none() {
        return None;
    }
    let origin = metadata_field::<Timeline>(timeline, "audio.timeline", errors)
        .map(|timeline| timeline.origin_unix_ms)
        .filter(|ms| ms.is_finite())
        .and_then(|ms| DateTime::from_timestamp_micros((ms * 1000.0).round() as i64));
    if origin.is_none() {
        errors.push("Voice session has no valid audio.timeline.origin_unix_ms".to_string());
    }
    let sources: BTreeMap<String, AudioSourceEntry> =
        metadata_field(sources, "audio.sources", errors).unwrap_or_default();
    let entries: BTreeMap<String, SegmentEntry> =
        metadata_field(segments, "audio.segments", errors).unwrap_or_default();
    let mut segments = Vec::new();
    for (id, entry) in entries {
        let mut channels = Vec::new();
        for (index, channel) in entry.channels {
            let Ok(index) = index.parse() else {
                errors.push(format!(
                    "Invalid channel index {index} in audio segment {id}"
                ));
                continue;
            };
            for source in &channel.sources {
                if !sources.contains_key(source) {
                    errors.push(format!("Unknown audio source {source} in segment {id}"));
                }
            }
            channels.push(Channel {
                index,
                sources: channel.sources,
            });
        }
        channels.sort_by_key(|channel| channel.index);
        let state = entry.state;
        if let SegmentState::Unknown(value) = &state {
            errors.push(format!("Audio segment {id} has an unknown state: {value}"));
        }
        if state == SegmentState::Ready && entry.attachment.is_none() {
            errors.push(format!("Ready audio segment {id} has no attachment"));
        }
        segments.push(RecordingSegment {
            id,
            state,
            start_offset_ms: entry.start_offset_ms,
            duration_ms: entry.duration_ms,
            attachment_ref: entry.attachment.map(|attachment| attachment.reference),
            mime_type: entry.mime_type,
            channels,
            reason: entry.reason,
        });
    }
    segments.sort_by(|a, b| {
        a.start_offset_ms
            .cmp(&b.start_offset_ms)
            .then(a.id.cmp(&b.id))
    });
    origin.map(|origin| Recording {
        id: span_id.to_string(),
        origin,
        sources: sources
            .into_iter()
            .map(|(id, source)| AudioSource {
                id,
                label: source.label,
            })
            .collect(),
        segments,
    })
}
