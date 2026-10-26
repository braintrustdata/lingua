//! Design-agnostic voice walk. Output designs implement `VoiceSink`.

use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, Speaker, VoiceKind};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::collections::HashMap;

pub(crate) trait VoiceSink<'a> {
    fn recording(&mut self, recording: Recording<'a>);
    /// User speech.
    fn stt(&mut self, speech: Speech<'a>);
    /// Agent speech.
    fn tts(&mut self, speech: Speech<'a>);
    fn warning(&mut self, span_id: &'a str, message: Cow<'a, str>);
}

/// A ready `audio.recordings` entry, identified by its attachment row and `name`.
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct Recording<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub attachment_ref: &'a str,
    pub start_time: DateTime<Utc>,
    pub mime_type: Option<&'a str>,
    pub duration_ms: Option<f64>,
    /// Boundary for each channel index.
    pub channels: Vec<&'a str>,
}

#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct Speech<'a> {
    pub id: &'a str,
    /// Cascade: the `turn.id` turn row. Duplex: the latest model response row, else the root row.
    pub owner: Option<&'a str>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub text: Option<Cow<'a, str>>,
    pub interrupted: Option<bool>,
    pub audio: Vec<Clip<'a>>,
}

/// Offsets into a ready recording.
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct Clip<'a> {
    pub recording_id: &'a str,
    pub recording_name: &'a str,
    pub attachment_ref: &'a str,
    pub channel_index: Option<u32>,
    pub start_offset_ms: f64,
    pub end_offset_ms: f64,
}

/// Walks one trace's spans in start order. Skips traces without voice spans or recordings.
pub(crate) fn walk<'a>(mut spans: Vec<&'a PreparedSpan>, sink: &mut impl VoiceSink<'a>) {
    if !spans
        .iter()
        .any(|span| span.source.voice.kind.is_some() || !span.source.voice.recordings.is_empty())
    {
        return;
    }
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
    let mut response = spans
        .iter()
        .find(|span| span.span_id == span.root_span_id)
        .map(|span| span.id.as_str());
    for span in spans {
        let id = span.id.as_str();
        for error in &span.source.voice.errors {
            sink.warning(id, Cow::Borrowed(error));
        }
        for recording in &span.source.voice.recordings {
            match ready_recording(recording, &by_span_id) {
                Ok(Some(recording)) => sink.recording(recording),
                Ok(None) => {}
                Err(message) => sink.warning(
                    id,
                    Cow::Owned(format!(
                        "Unsupported audio recording {}: {message}",
                        recording.id
                    )),
                ),
            }
        }
        let speaker = match span.source.voice.kind {
            Some(VoiceKind::Speech(speaker)) => speaker,
            None if span.kind() == "llm" && span.source.voice.turn_id.is_none() => {
                response = Some(id);
                continue;
            }
            _ => continue,
        };
        let owner = match span.source.voice.turn_id.as_deref() {
            Some(turn_id) => match by_span_id.get(turn_id) {
                Some(turn) if matches!(turn.source.voice.kind, Some(VoiceKind::Turn(_))) => {
                    Some(turn.id.as_str())
                }
                _ => {
                    sink.warning(
                        id,
                        Cow::Owned(format!(
                            "Unresolved voice turn.id {turn_id}: no voice turn has that span ID"
                        )),
                    );
                    None
                }
            },
            None => response,
        };
        let mut audio = Vec::new();
        for selection in &span.source.voice.selections {
            let recording = selection
                .recording_span_id
                .as_deref()
                .and_then(|span_id| by_span_id.get(span_id))
                .and_then(|row| {
                    row.source
                        .voice
                        .recordings
                        .iter()
                        .find(|recording| recording.id == selection.recording_id)
                });
            let Some(recording) = recording else {
                sink.warning(
                    id,
                    Cow::Owned(format!(
                        "Unresolved audio selection for recording {}: recording_span_id is not in this trace",
                        selection.recording_id
                    )),
                );
                continue;
            };
            if let Ok(Some(recording)) = ready_recording(recording, &by_span_id) {
                audio.push(Clip {
                    recording_id: recording.id,
                    recording_name: recording.name,
                    attachment_ref: recording.attachment_ref,
                    channel_index: selection.channel_index,
                    start_offset_ms: selection.start_offset_ms,
                    end_offset_ms: selection.end_offset_ms,
                });
            }
        }
        let text = match speech_text(span, speaker) {
            Ok(Some(text)) if !text.is_empty() => Some(text),
            Ok(_) => playback_text(span, &by_span_id, &speech_by_turn).map(Cow::Borrowed),
            Err(message) => {
                sink.warning(id, Cow::Borrowed(message));
                None
            }
        };
        let speech = Speech {
            id,
            owner,
            start_time: span.start,
            end_time: span.source.end,
            text,
            interrupted: span.source.voice.interrupted,
            audio,
        };
        match speaker {
            Speaker::User => sink.stt(speech),
            Speaker::Agent => sink.tts(speech),
        }
    }
}

// Only use the last playback transcript when each speech has a matching playback event.
fn playback_text<'a>(
    span: &'a PreparedSpan,
    by_span_id: &HashMap<&str, &'a PreparedSpan>,
    speech_by_turn: &HashMap<&str, Vec<&PreparedSpan>>,
) -> Option<&'a str> {
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
    playback.transcript.as_deref()
}

fn ready_recording<'a>(
    recording: &'a RecordingMetadata,
    by_span_id: &HashMap<&str, &'a PreparedSpan>,
) -> Result<Option<Recording<'a>>, String> {
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
    let timeline = recording.timeline.as_ref().ok_or("missing timeline")?;
    let start_ms = timeline.origin_unix_ms + timeline.recording_start_offset_ms;
    let start_time = DateTime::from_timestamp_micros((start_ms * 1000.0).round() as i64)
        .ok_or("timeline is out of range")?;
    let mut sources: Vec<_> = recording.sources.iter().collect();
    sources.sort_by_key(|source| source.channel_index);
    let contiguous = sources
        .iter()
        .enumerate()
        .all(|(index, source)| source.channel_index.unwrap_or(0) as usize == index);
    if !contiguous {
        return Err("channel indexes must be 0 through n - 1".to_string());
    }
    Ok(Some(Recording {
        id: &row.id,
        name: &recording.id,
        attachment_ref: &attachment.reference,
        start_time,
        mime_type: recording.mime_type.as_deref(),
        duration_ms: recording.duration_ms,
        channels: sources
            .iter()
            .map(|source| source.boundary.as_str())
            .collect(),
    }))
}

/// The speech span's own recorded text: user input or agent output, never STT or model text.
fn speech_text(
    span: &PreparedSpan,
    speaker: Speaker,
) -> Result<Option<Cow<'_, str>>, &'static str> {
    let mut texts: Vec<_> = match speaker {
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
    match texts.as_mut_slice() {
        [] => Ok(None),
        [text @ Some(_)] => Ok(text.take()),
        _ => Err("Unsupported speech payload: expected one text message"),
    }
}

fn user_text(content: &UserContent) -> Option<Cow<'_, str>> {
    match content {
        UserContent::String(text) => Some(Cow::Borrowed(text)),
        UserContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                UserContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect::<Option<String>>()
            .map(Cow::Owned),
    }
}

fn assistant_text(content: &AssistantContent) -> Option<Cow<'_, str>> {
    match content {
        AssistantContent::String(text) => Some(Cow::Borrowed(text)),
        AssistantContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                AssistantContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect::<Option<String>>()
            .map(Cow::Owned),
    }
}
