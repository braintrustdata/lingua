//! LiveKit agent sessions, as the Braintrust JS SDK traces them.

use super::{Kind, PreparedSpan, WorkKind};

pub(super) fn is_session(span: &PreparedSpan) -> bool {
    span.kind() == "task" && span.source.name.as_deref() == Some("session")
}

pub(super) fn task_kind(span: &PreparedSpan) -> Option<Kind> {
    match span.source.name.as_deref()? {
        "user_turn" => Some(Kind::UserTurn),
        "agent_turn" => Some(Kind::AgentTurn),
        "user_speech" => Some(Kind::UserSpeech),
        "agent_speech" => Some(Kind::Work(WorkKind::Speech)),
        "stt" | "tts" | "transcript_received" => Some(Kind::Work(WorkKind::SpeechTask)),
        _ => None,
    }
}

pub(super) fn interrupted(span: &PreparedSpan) -> bool {
    span.source.voice.interrupted == Some(true)
}
