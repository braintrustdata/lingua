//! Output-agnostic voice resolution for one trace. [`resolve`] turns voice spans into
//! [`ResolvedSpeech`] and [`assign_turns`] names each one's [`Owner`]. An output design adds
//! `emit(&Resolved, &[Owner])` here to write them into the trajectory.

use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, Speaker, VoiceKind};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(super) struct Resolved {
    pub speech: Vec<ResolvedSpeech>,
    pub warnings: Vec<VoiceWarning>,
}

#[derive(Debug, Serialize)]
pub(super) struct VoiceWarning {
    pub span_id: String,
    pub message: String,
}

/// A speech, STT, or TTS row.
#[derive(Debug, Serialize)]
pub(super) struct ResolvedSpeech {
    pub id: String,
    pub kind: VoiceKind,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub text: Option<String>,
    pub interrupted: Option<bool>,
    pub audio: Vec<ResolvedClip>,
    /// Row of the voice turn that `turn.id` names.
    pub turn_id: Option<String>,
}

/// Offsets into a ready recording.
#[derive(Debug, Serialize)]
pub(super) struct ResolvedClip {
    /// Row holding the attachment.
    pub recording_id: String,
    /// JSON pointer into that row.
    pub attachment_ref: String,
    pub channel_index: Option<u32>,
    pub start_offset_ms: f64,
    pub end_offset_ms: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub(super) enum Owner {
    /// A voice turn row.
    Turn(String),
    /// A model response row.
    Response(String),
    Session,
}

pub(super) fn resolve(mut spans: Vec<&PreparedSpan>) -> Resolved {
    spans.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.source.exec_counter.cmp(&b.source.exec_counter))
            .then(a.id.cmp(&b.id))
    });
    let by_span_id: HashMap<&str, &PreparedSpan> = spans
        .iter()
        .map(|span| (span.span_id.as_str(), *span))
        .collect();
    let mut speech_by_turn = HashMap::<&str, Vec<&PreparedSpan>>::new();
    for span in &spans {
        if let (Some(VoiceKind::Speech(Speaker::Agent)), Some(turn_id)) =
            (span.source.voice.kind, span.source.voice.turn_id.as_deref())
        {
            speech_by_turn.entry(turn_id).or_default().push(span);
        }
    }
    let mut warnings = Vec::new();
    let mut recordings = HashMap::new();
    for span in &spans {
        for error in &span.source.voice.errors {
            warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message: error.clone(),
            });
        }
        for recording in &span.source.voice.recordings {
            let attachment = audio_recording(recording, &by_span_id).unwrap_or_else(|message| {
                warnings.push(VoiceWarning {
                    span_id: span.id.clone(),
                    message: format!("Unsupported audio recording {}: {message}", recording.id),
                });
                None
            });
            recordings.insert((span.span_id.as_str(), recording.id.as_str()), attachment);
        }
    }
    let mut speech = Vec::new();
    for span in spans {
        let kind = match span.source.voice.kind {
            Some(VoiceKind::Turn(_)) | None => continue,
            Some(kind) => kind,
        };
        let mut warn = |message: String| {
            warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message,
            })
        };
        let turn_id = turn_id(span, &by_span_id, &mut warn);
        let mut audio = Vec::new();
        for selection in &span.source.voice.selections {
            let key = selection
                .recording_span_id
                .as_deref()
                .map(|span_id| (span_id, selection.recording_id.as_str()));
            match key.and_then(|key| recordings.get(&key)) {
                Some(Some((recording_id, attachment_ref))) => audio.push(ResolvedClip {
                    recording_id: recording_id.clone(),
                    attachment_ref: attachment_ref.clone(),
                    channel_index: selection.channel_index,
                    start_offset_ms: selection.start_offset_ms,
                    end_offset_ms: selection.end_offset_ms,
                }),
                Some(None) => {}
                None => warn(format!(
                    "Unresolved audio selection for recording {}: recording_span_id does not list it",
                    selection.recording_id
                )),
            }
        }
        let text = match kind {
            VoiceKind::Speech(speaker) => match speech_text(span, speaker) {
                Ok(Some(text)) if !text.is_empty() => Some(text),
                Ok(_) => playback_text(span, &by_span_id, &speech_by_turn),
                Err(message) => {
                    warn(message);
                    None
                }
            },
            _ => None,
        };
        speech.push(ResolvedSpeech {
            id: span.id.clone(),
            kind,
            start_time: span.start,
            end_time: span.source.end,
            text,
            interrupted: span.source.voice.interrupted,
            audio,
            turn_id,
        });
    }
    Resolved { speech, warnings }
}

/// Cascade traces have voice turns, which own speech through `turn.id`. In other traces, agent
/// speech belongs to the latest model response that started before it and user speech to the
/// next one. Speech without an owner belongs to the session.
pub(super) fn assign_turns(spans: &[&PreparedSpan], speech: &[ResolvedSpeech]) -> Vec<Owner> {
    let cascade = spans
        .iter()
        .any(|span| matches!(span.source.voice.kind, Some(VoiceKind::Turn(_))));
    let mut responses: Vec<_> = spans.iter().filter(|span| span.kind() == "llm").collect();
    responses.sort_by_key(|span| span.start);
    speech
        .iter()
        .map(|speech| {
            if cascade {
                return speech.turn_id.clone().map_or(Owner::Session, Owner::Turn);
            }
            let next = responses.partition_point(|span| span.start <= speech.start_time);
            let response = match speech.kind {
                VoiceKind::Speech(Speaker::User) | VoiceKind::Stt => responses.get(next),
                _ => next.checked_sub(1).and_then(|index| responses.get(index)),
            };
            response.map_or(Owner::Session, |span| Owner::Response(span.id.clone()))
        })
        .collect()
}

// Only use the last playback transcript when each speech has a matching playback event.
fn playback_text(
    span: &PreparedSpan,
    by_span_id: &HashMap<&str, &PreparedSpan>,
    speech_by_turn: &HashMap<&str, Vec<&PreparedSpan>>,
) -> Option<String> {
    let turn_id = span.source.voice.turn_id.as_deref()?;
    let turn = by_span_id.get(turn_id)?;
    let speeches = speech_by_turn.get(turn_id)?;
    let playback = turn.source.voice.playbacks.last()?;
    if turn.source.voice.kind != Some(VoiceKind::Turn(Speaker::Agent))
        || speeches.last()?.id != span.id
        || speeches.len() != turn.source.voice.playbacks.len()
        || span.source.voice.interrupted != Some(true)
        || playback.interrupted != Some(true)
    {
        return None;
    }
    playback.transcript.clone()
}

/// The attachment row and pointer of a ready recording.
fn audio_recording(
    recording: &RecordingMetadata,
    by_span_id: &HashMap<&str, &PreparedSpan>,
) -> Result<Option<(String, String)>, String> {
    if recording
        .state
        .as_deref()
        .is_some_and(|state| state != "ready")
    {
        return Ok(None);
    }
    let attachment = recording.attachment.as_ref().ok_or("missing attachment")?;
    let row = by_span_id.get(attachment.span_id.as_str()).ok_or_else(|| {
        format!(
            "attachment span {} is not in this trace",
            attachment.span_id
        )
    })?;
    recording.timeline.as_ref().ok_or("missing timeline")?;
    let mut channels: Vec<_> = recording
        .sources
        .iter()
        .map(|source| source.channel_index.unwrap_or(0) as usize)
        .collect();
    channels.sort();
    if channels
        .iter()
        .enumerate()
        .any(|(index, channel)| index != *channel)
    {
        return Err("channel indexes must be 0 through n - 1".to_string());
    }
    Ok(Some((row.id.clone(), attachment.reference.clone())))
}

/// The row of the turn span that the span's `turn.id` names.
fn turn_id(
    span: &PreparedSpan,
    by_span_id: &HashMap<&str, &PreparedSpan>,
    warn: &mut impl FnMut(String),
) -> Option<String> {
    let turn_id = span.source.voice.turn_id.as_deref()?;
    match by_span_id.get(turn_id) {
        Some(turn) if matches!(turn.source.voice.kind, Some(VoiceKind::Turn(_))) => {
            Some(turn.id.clone())
        }
        _ => {
            warn(format!(
                "Unresolved voice turn.id {turn_id}: no voice turn has that span ID"
            ));
            None
        }
    }
}

/// The speech span's own recorded text: user input or agent output, never STT or model text.
fn speech_text(span: &PreparedSpan, speaker: Speaker) -> Result<Option<String>, String> {
    let texts: Vec<_> = match speaker {
        Speaker::User => span
            .input
            .iter()
            .filter_map(|input| match &input.message {
                Message::User { content } => Some(user_text(content)),
                _ => None,
            })
            .collect(),
        Speaker::Agent => span
            .output
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { content, .. } => Some(assistant_text(content)),
                _ => None,
            })
            .collect(),
    };
    match texts.as_slice() {
        [] => Ok(None),
        [Some(text)] => Ok(Some(text.clone())),
        _ => Err("Unsupported speech payload: expected one text message".to_string()),
    }
}

fn user_text(content: &UserContent) -> Option<String> {
    match content {
        UserContent::String(text) => Some(text.clone()),
        UserContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                UserContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
    }
}

fn assistant_text(content: &AssistantContent) -> Option<String> {
    match content {
        AssistantContent::String(text) => Some(text.clone()),
        AssistantContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                AssistantContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
    }
}
