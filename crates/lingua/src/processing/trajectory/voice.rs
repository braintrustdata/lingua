//! Shared helpers for voice trajectory builders. `spans` arguments are one trace's spans, by start.

use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, SelectionMetadata, Speaker, VoiceKind};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A ready `audio.recordings` entry, identified by the row holding it and its `name`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioRecording {
    pub id: String,
    pub name: String,
    pub attachment_ref: String,
    pub start_time: DateTime<Utc>,
    pub mime_type: Option<String>,
    pub duration_ms: Option<f64>,
    pub channels: Vec<String>,
}

/// `"stt"`, `"tts"`, `"llm"`, or `"tool"` for spans that did voice or model work.
pub(crate) fn step_kind(span: &PreparedSpan) -> Option<&'static str> {
    match span.source.voice.kind {
        Some(VoiceKind::Stt) => Some("stt"),
        Some(VoiceKind::Tts) => Some("tts"),
        Some(_) => None,
        None => match span.kind() {
            "llm" => Some("llm"),
            "tool" => Some("tool"),
            _ => None,
        },
    }
}

pub(crate) fn audio_recording(
    recording: &RecordingMetadata,
    by_span_id: &HashMap<&str, &PreparedSpan>,
) -> Result<Option<AudioRecording>, String> {
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
    Ok(Some(AudioRecording {
        id: row.id.clone(),
        name: recording.id.clone(),
        attachment_ref: attachment.reference.clone(),
        start_time,
        mime_type: recording.mime_type.clone(),
        duration_ms: recording.duration_ms,
        channels: sources
            .iter()
            .map(|source| source.boundary.clone())
            .collect(),
    }))
}

/// The speech span's audio selections, each with the row ID of the recording it points into.
pub(crate) fn audio_clips<'a>(
    span: &'a PreparedSpan,
    by_span_id: &HashMap<&str, &'a PreparedSpan>,
    warn: &mut impl FnMut(String),
) -> Vec<(&'a str, &'a SelectionMetadata)> {
    let mut clips = Vec::new();
    for selection in &span.source.voice.selections {
        let row = selection
            .recording_span_id
            .as_deref()
            .and_then(|span_id| by_span_id.get(span_id));
        let Some(row) = row else {
            warn(format!(
                "Unresolved audio selection for recording {}: recording_span_id is not in this trace",
                selection.recording_id
            ));
            continue;
        };
        clips.push((row.id.as_str(), selection));
    }
    clips
}

/// The row ID of the span's group: its source turn when the trace records turns, otherwise the
/// model response that is running or next when it starts, otherwise the session root.
pub(crate) fn group_id(
    span: &PreparedSpan,
    spans: &[&PreparedSpan],
    by_span_id: &HashMap<&str, &PreparedSpan>,
    warn: &mut impl FnMut(String),
) -> Option<String> {
    if spans
        .iter()
        .any(|span| matches!(span.source.voice.kind, Some(VoiceKind::Turn(_))))
    {
        return turn_id(span, by_span_id, warn);
    }
    let mut responses = spans.iter().filter(|span| span.kind() == "llm").peekable();
    if responses.peek().is_none() {
        return by_span_id
            .get(span.root_span_id.as_str())
            .map(|root| root.id.clone());
    }
    responses
        .find(|response| response.source.end.is_some_and(|end| end >= span.start))
        .map(|response| response.id.clone())
}

/// The speech span's own text, or the final playback transcript when its playback was cut off.
pub(crate) fn utterance_text(
    span: &PreparedSpan,
    speaker: Speaker,
    spans: &[&PreparedSpan],
    by_span_id: &HashMap<&str, &PreparedSpan>,
    warn: &mut impl FnMut(String),
) -> Option<String> {
    match speech_text(span, speaker) {
        Ok(Some(text)) if !text.is_empty() => Some(text),
        Ok(_) => playback_text(span, spans, by_span_id),
        Err(message) => {
            warn(message);
            None
        }
    }
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

// Only use the last playback transcript when each speech has a matching playback event.
fn playback_text(
    span: &PreparedSpan,
    spans: &[&PreparedSpan],
    by_span_id: &HashMap<&str, &PreparedSpan>,
) -> Option<String> {
    let turn_id = span.source.voice.turn_id.as_deref()?;
    let turn = by_span_id.get(turn_id)?;
    let speeches: Vec<_> = spans
        .iter()
        .filter(|speech| {
            speech.source.voice.kind == Some(VoiceKind::Speech(Speaker::Agent))
                && speech.source.voice.turn_id.as_deref() == Some(turn_id)
        })
        .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::import::{import_span, Span};
    use serde_json::{json, Value};

    fn summarize(fixture: &str, keep: impl Fn(&PreparedSpan) -> bool) -> (Value, Vec<String>) {
        let sources: Vec<Span> = serde_json::from_str(fixture).unwrap();
        let mut owned: Vec<_> = sources
            .into_iter()
            .map(|source| PreparedSpan::new(import_span(source).unwrap()).unwrap())
            .filter(keep)
            .collect();
        owned.sort_by_key(|span| span.start);
        let spans: Vec<_> = owned.iter().collect();
        let by_span_id: HashMap<_, _> = spans
            .iter()
            .map(|span| (span.span_id.as_str(), *span))
            .collect();
        let mut warnings = Vec::new();
        let mut warn = |message: String| warnings.push(message);
        let mut rows = Vec::new();
        for span in &spans {
            span.source.voice.errors.iter().cloned().for_each(&mut warn);
            for recording in &span.source.voice.recordings {
                match audio_recording(recording, &by_span_id) {
                    Ok(recording) => rows.push(json!(recording.map(|recording| format!(
                        "{} {} {} {} {:?}",
                        recording.id,
                        recording.name,
                        recording.attachment_ref,
                        recording.start_time.to_rfc3339(),
                        recording.channels
                    )))),
                    Err(message) => warn(format!("{}: {message}", recording.id)),
                }
            }
            let group = group_id(span, &spans, &by_span_id, &mut warn);
            if let Some(VoiceKind::Speech(speaker)) = span.source.voice.kind {
                let text = utterance_text(span, speaker, &spans, &by_span_id, &mut warn);
                let clips: Vec<_> = audio_clips(span, &by_span_id, &mut warn)
                    .into_iter()
                    .map(|(row, clip)| {
                        format!(
                            "{row} {} {:?} {}-{}",
                            clip.recording_id,
                            clip.channel_index,
                            clip.start_offset_ms,
                            clip.end_offset_ms
                        )
                    })
                    .collect();
                rows.push(json!([span.id, "speech", group, text, clips]));
            } else if let Some(kind) = step_kind(span) {
                rows.push(json!([span.id, kind, group]));
            }
        }
        (json!(rows), warnings)
    }

    #[test]
    fn cascade_helpers_resolve_turns_audio_text_and_links() {
        let (rows, warnings) = summarize(include_str!("fixtures/voice-cascade.json"), |_| true);
        assert_eq!(
            rows,
            json!([
                "r-session call-0000 /input/audio/call-0000 1970-01-01T00:00:01.250+00:00 [\"session_input\", \"local_audio_output\"]",
                null,
                ["r-u1-speech", "speech", "r-u1", "Hi", ["r-session call-0000 Some(0) 750-1500.5"]],
                ["r-stt", "stt", "r-u1"],
                ["r-llm", "llm", "r-a1"],
                ["r-tool", "tool", "r-a1"],
                ["r-tts", "tts", "r-a1"],
                ["r-a1-speech", "speech", "r-a1", "Spoken so far", []],
                ["r-orphan", "speech", null, null, []],
            ])
        );
        assert_eq!(warnings.len(), 4);
        assert_eq!(
            warnings[0],
            "orphan: attachment span s-gone is not in this trace"
        );
        assert!(warnings[1].starts_with("Unresolved audio selection for recording call-0000"));
        assert!(warnings[2].starts_with("Invalid metadata.contrib.livekit.interrupted:"));
        assert_eq!(
            warnings[3],
            "Unresolved voice turn.id s-gone: no voice turn has that span ID"
        );
    }

    #[test]
    fn duplex_helpers_group_by_model_response_or_session() {
        let fixture = include_str!("fixtures/voice-duplex.json");
        let (rows, warnings) = summarize(fixture, |_| true);
        assert!(warnings.is_empty());
        assert_eq!(
            rows,
            json!([
                ["r-llm1", "llm", "r-llm1"],
                ["r-agent1", "speech", "r-llm1", "Hello", []],
                ["r-user1", "speech", "r-llm2", "Where is my order?", []],
                ["r-llm2", "llm", "r-llm2"],
                ["r-agent2", "speech", "r-llm2", null, []],
                ["r-tool", "tool", "r-llm2"],
                ["r-user2", "speech", null, null, []],
            ])
        );
        let (rows, _) = summarize(fixture, |span| span.kind() != "llm");
        assert_eq!(
            rows,
            json!([
                ["r-agent1", "speech", "r-session", "Hello", []],
                ["r-user1", "speech", "r-session", "Where is my order?", []],
                ["r-agent2", "speech", "r-session", null, []],
                ["r-tool", "tool", "r-session"],
                ["r-user2", "speech", "r-session", null, []],
            ])
        );
    }
}
