//! LiveKit agent sessions as logged by the Braintrust TypeScript SDK.

use super::span::{Speaker, VoiceKind};

pub(super) fn voice_kind(kind: Option<&str>, name: Option<&str>) -> Option<VoiceKind> {
    if kind != Some("task") {
        return None;
    }
    match name? {
        "user_turn" => Some(VoiceKind::Turn(Speaker::User)),
        "agent_turn" => Some(VoiceKind::Turn(Speaker::Agent)),
        "user_speech" => Some(VoiceKind::Speech(Speaker::User)),
        "agent_speech" => Some(VoiceKind::Speech(Speaker::Agent)),
        "stt" => Some(VoiceKind::Stt),
        "tts" => Some(VoiceKind::Tts),
        _ => None,
    }
}
