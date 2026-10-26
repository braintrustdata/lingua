//! LiveKit agent sessions as logged by the Braintrust TypeScript SDK.

use super::span::{Playback, Speaker, VoiceKind};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct PlaybackEvent {
    name: String,
    attributes: PlaybackAttributes,
}

#[derive(Deserialize)]
struct PlaybackAttributes {
    interrupted: Option<bool>,
    #[serde(rename = "synchronizedTranscript")]
    synchronized_transcript: Option<String>,
}

/// The `playbackFinished` events LiveKit records on an agent turn, in order.
pub(super) fn playbacks(events: Vec<PlaybackEvent>) -> Vec<Playback> {
    events
        .into_iter()
        .filter(|event| event.name == "playbackFinished")
        .map(|event| Playback {
            interrupted: event.attributes.interrupted,
            transcript: event.attributes.synchronized_transcript,
        })
        .collect()
}

pub(super) fn voice_kind(kind: Option<&str>, name: Option<&str>) -> Option<VoiceKind> {
    if kind != Some("task") {
        return None;
    }
    match name? {
        "user_turn" => Some(VoiceKind::Turn(Speaker::User)),
        "agent_turn" => Some(VoiceKind::Turn(Speaker::Agent)),
        "user_speech" => Some(VoiceKind::Speech(Speaker::User)),
        "agent_speech" => Some(VoiceKind::Speech(Speaker::Agent)),
        _ => None,
    }
}
