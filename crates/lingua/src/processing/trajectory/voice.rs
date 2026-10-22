use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, Speaker, VoiceKind};
use crate::universal::trajectory::{
    AudioClip, AudioRecording, Utterance, Voice, VoiceStep, VoiceTurn,
};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::DateTime;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub(super) struct VoiceBuild {
    pub voice: Voice,
    pub warnings: Vec<VoiceWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VoiceWarning {
    pub span_id: String,
    pub message: String,
}

/// Builds voice for traces with voice spans or recordings. Owners come from `turn.id`.
pub(super) fn build<'a>(
    spans: impl IntoIterator<Item = &'a PreparedSpan>,
) -> BTreeMap<String, VoiceBuild> {
    let mut roots = BTreeMap::<&str, Vec<&PreparedSpan>>::new();
    for span in spans {
        roots
            .entry(span.root_span_id.as_str())
            .or_default()
            .push(span);
    }
    roots
        .into_iter()
        .filter(|(_, spans)| {
            spans.iter().any(|span| {
                span.source.voice.kind.is_some() || !span.source.voice.recordings.is_empty()
            })
        })
        .map(|(root, spans)| (root.to_string(), build_root(spans)))
        .collect()
}

fn build_root(mut spans: Vec<&PreparedSpan>) -> VoiceBuild {
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
    let mut voice = Voice {
        recordings: Vec::new(),
        turns: Vec::new(),
        utterances: Vec::new(),
        steps: Vec::new(),
    };
    let mut warnings = Vec::new();
    for span in spans {
        let mut warn = |message: String| {
            warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message,
            })
        };
        for error in &span.source.voice.errors {
            warn(error.clone());
        }
        for recording in &span.source.voice.recordings {
            match audio_recording(recording, &by_span_id) {
                Ok(Some(recording)) => voice.recordings.push(recording),
                Ok(None) => {}
                Err(message) => warn(format!(
                    "Unsupported audio recording {}: {message}",
                    recording.id
                )),
            }
        }
        let step_kind = match span.source.voice.kind {
            Some(VoiceKind::Turn(speaker)) => {
                voice.turns.push(VoiceTurn {
                    id: span.id.clone(),
                    speaker: speaker.as_str().to_string(),
                    start_time: span.start,
                    end_time: span.source.end,
                });
                continue;
            }
            Some(VoiceKind::Speech(speaker)) => {
                let turn_id = turn_id(span, &by_span_id, &mut warn);
                let mut audio = Vec::new();
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
                    audio.push(AudioClip {
                        recording_id: row.id.clone(),
                        recording_name: selection.recording_id.clone(),
                        channel_index: selection.channel_index,
                        start_offset_ms: selection.start_offset_ms,
                        end_offset_ms: selection.end_offset_ms,
                    });
                }
                let text = match speech_text(span, speaker) {
                    Ok(Some(text)) if !text.is_empty() => Some(text),
                    Ok(_) => span.source.voice.fallback_text.clone(),
                    Err(message) => {
                        warn(message);
                        None
                    }
                };
                voice.utterances.push(Utterance {
                    id: span.id.clone(),
                    speaker: speaker.as_str().to_string(),
                    start_time: span.start,
                    end_time: span.source.end,
                    text,
                    interrupted: span.source.voice.interrupted,
                    audio,
                    turn_id,
                });
                continue;
            }
            Some(VoiceKind::Stt) => "stt",
            Some(VoiceKind::Tts) => "tts",
            None if span.is_scorer() => continue,
            None => match span.kind() {
                "llm" => "llm",
                "tool" => "tool",
                _ => continue,
            },
        };
        voice.steps.push(VoiceStep {
            id: span.id.clone(),
            kind: step_kind.to_string(),
            start_time: span.start,
            end_time: span.source.end,
            turn_id: turn_id(span, &by_span_id, &mut warn),
        });
    }
    voice
        .recordings
        .sort_by_key(|recording| recording.start_time);
    VoiceBuild { voice, warnings }
}

fn audio_recording(
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
