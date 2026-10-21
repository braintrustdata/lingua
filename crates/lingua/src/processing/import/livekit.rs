//! LiveKit agent sessions as logged by the Braintrust TypeScript SDK.

use super::span::{Speaker, VoiceKind};
use crate::serde_json as json;
use serde::Deserialize;

/// Metadata keys this adapter reads, projected into span headers with the core keys.
pub(super) const INTERRUPTED_FIELD: &str = "contrib.livekit.interrupted";

#[derive(Default, Deserialize)]
pub(super) struct Metadata {
    #[serde(rename = "contrib.livekit.interrupted")]
    interrupted: Option<json::Value>,
}

/// LiveKit's voice facts, before core import combines them with Braintrust's own keys.
pub(super) struct Voice {
    pub kind: Option<VoiceKind>,
    pub interrupted: Option<bool>,
}

pub(super) fn voice(
    kind: Option<&str>,
    name: Option<&str>,
    metadata: Metadata,
    errors: &mut Vec<String>,
) -> Voice {
    let kind = voice_kind(kind, name);
    let interrupted = match kind {
        Some(VoiceKind::Speech(_)) => {
            super::span::metadata_field(metadata.interrupted, INTERRUPTED_FIELD, errors)
        }
        _ => None,
    };
    Voice { kind, interrupted }
}

fn voice_kind(kind: Option<&str>, name: Option<&str>) -> Option<VoiceKind> {
    if kind.is_some_and(|kind| kind != "task") {
        return None;
    }
    match name? {
        "user_turn" => Some(VoiceKind::Turn(Speaker::User)),
        "agent_turn" => Some(VoiceKind::Turn(Speaker::Agent)),
        "user_speech" => Some(VoiceKind::Speech(Speaker::User)),
        "agent_speech" => Some(VoiceKind::Speech(Speaker::Agent)),
        "stt" | "tts" => Some(VoiceKind::Operation),
        _ => None,
    }
}
