//! LiveKit agent sessions, as the Braintrust JS SDK traces them.

use super::{PreparedSpan, SpanKind, WorkKind};

pub(super) fn is_session(span: &PreparedSpan) -> bool {
    span.kind() == "task" && span.source.name.as_deref() == Some("session")
}

pub(super) fn task_kind(span: &PreparedSpan) -> Option<SpanKind> {
    match span.source.name.as_deref()? {
        "user_turn" => Some(SpanKind::UserTurn),
        "agent_turn" => Some(SpanKind::AgentTurn),
        "user_speech" => Some(SpanKind::UserSpeech),
        "agent_speech" => Some(SpanKind::Work(WorkKind::Speech)),
        "stt" | "tts" | "transcript_received" => Some(SpanKind::Work(WorkKind::SpeechTask)),
        _ => None,
    }
}
