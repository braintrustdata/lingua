use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, SelectionMetadata, Speaker, VoiceKind};
use crate::universal::trajectory::{AudioClip, Stt, Tts, Turn, Work, WorkStep};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};

/// The turn that holds each model or tool row, with the row's start time.
#[derive(Default)]
pub(super) struct ModelTurns {
    pub turns: Vec<(DateTime<Utc>, String)>,
    pub owners: HashMap<String, String>,
}

#[derive(Default)]
pub(super) struct VoiceBuild {
    /// Turns with only voice work, such as a greeting or a trailing user turn.
    pub turns: Vec<Turn>,
    /// Voice steps and the ID of the turn that holds each one.
    pub work: Vec<(String, WorkStep)>,
    pub interrupted: Vec<String>,
    pub warnings: Vec<VoiceWarning>,
}

pub(super) struct VoiceWarning {
    pub span_id: String,
    pub message: String,
}

pub(super) fn has_voice(span: &PreparedSpan) -> bool {
    span.source.voice.kind.is_some() || !span.source.voice.recordings.is_empty()
}

/// Builds voice work for traces with voice spans. Owners come from `turn.id`.
pub(super) fn build<'a>(
    spans: impl IntoIterator<Item = &'a PreparedSpan>,
    model_turns: &BTreeMap<String, ModelTurns>,
) -> BTreeMap<String, VoiceBuild> {
    let mut roots = BTreeMap::<&str, Vec<&PreparedSpan>>::new();
    for span in spans {
        roots
            .entry(span.root_span_id.as_str())
            .or_default()
            .push(span);
    }
    let empty = ModelTurns::default();
    roots
        .into_iter()
        .filter(|(_, spans)| spans.iter().any(|span| has_voice(span)))
        .map(|(root, spans)| {
            let build = build_root(spans, model_turns.get(root).unwrap_or(&empty));
            (root.to_string(), build)
        })
        .collect()
}

struct Recording<'a> {
    row: &'a str,
    metadata: &'a RecordingMetadata,
    attachment_ref: &'a str,
}

struct Step<'a> {
    speaker: Speaker,
    turn: Option<&'a PreparedSpan>,
    step: WorkStep,
}

fn build_root(mut spans: Vec<&PreparedSpan>, model: &ModelTurns) -> VoiceBuild {
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
    let mut build = VoiceBuild::default();
    let mut recordings = HashMap::<(&str, &str), Option<Recording>>::new();
    for span in &spans {
        for recording in &span.source.voice.recordings {
            let resolved = match audio_recording(recording, &by_span_id) {
                Ok(resolved) => resolved,
                Err(message) => {
                    build.warnings.push(VoiceWarning {
                        span_id: span.id.clone(),
                        message: format!("Unsupported audio recording {}: {message}", recording.id),
                    });
                    None
                }
            };
            recordings.insert((span.span_id.as_str(), recording.id.as_str()), resolved);
        }
    }
    let mut steps = Vec::new();
    for span in &spans {
        let mut warn = |message: String| {
            build.warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message,
            })
        };
        for error in &span.source.voice.errors {
            warn(error.clone());
        }
        let (speaker, speech) = match span.source.voice.kind {
            Some(VoiceKind::Speech(speaker)) => (speaker, true),
            Some(VoiceKind::Stt) => (Speaker::User, false),
            Some(VoiceKind::Tts) => (Speaker::Agent, false),
            Some(VoiceKind::Turn(_)) | None => continue,
        };
        let turn = voice_turn(span, &by_span_id, &mut warn);
        let mut audio = Vec::new();
        for selection in &span.source.voice.selections {
            if let Some(clip) = audio_clip(selection, &by_span_id, &recordings, &mut warn) {
                audio.push(clip);
            }
        }
        let text = if speech {
            match speech_text(span, speaker) {
                Ok(Some(text)) if !text.is_empty() => Some(text),
                Ok(_) => None,
                Err(message) => {
                    warn(message);
                    None
                }
            }
        } else {
            None
        };
        let work = match speaker {
            Speaker::User => Work::Stt(Box::new(Stt {
                transcript: text,
                audio,
            })),
            Speaker::Agent => Work::Tts(Box::new(Tts {
                text,
                interrupted: span.source.voice.interrupted,
                audio,
            })),
        };
        steps.push(Step {
            speaker,
            turn,
            step: WorkStep {
                id: span.id.clone(),
                span_type: span.kind().to_string(),
                name: span.source.name.clone(),
                error: span.source.error.clone(),
                start_time: span.start,
                end_time: span.source.end,
                work,
                sub_agent: None,
            },
        });
    }
    let source_turns: Vec<_> = spans
        .iter()
        .filter_map(|span| match span.source.voice.kind {
            Some(VoiceKind::Turn(speaker)) => Some((*span, speaker)),
            _ => None,
        })
        .collect();
    let mut owners = HashMap::<&str, String>::new();
    for (turn, _) in source_turns
        .iter()
        .filter(|(_, speaker)| *speaker == Speaker::Agent)
    {
        let id = spans
            .iter()
            .filter(|span| span.source.voice.turn_id.as_deref() == Some(turn.span_id.as_str()))
            .find_map(|span| model.owners.get(&span.id).cloned())
            .unwrap_or_else(|| {
                build
                    .turns
                    .push(voice_turn_shell(&turn.id, turn.start, turn.source.end));
                turn.id.clone()
            });
        if turn.source.voice.interrupted == Some(true) {
            build.interrupted.push(id.clone());
        }
        owners.insert(turn.span_id.as_str(), id);
    }
    for (turn, _) in source_turns
        .iter()
        .filter(|(_, speaker)| *speaker == Speaker::User)
    {
        let id = source_turns
            .iter()
            .find(|(next, speaker)| *speaker == Speaker::Agent && next.start > turn.start)
            .map(|(next, _)| owners[next.span_id.as_str()].clone())
            .unwrap_or_else(|| {
                build
                    .turns
                    .push(voice_turn_shell(&turn.id, turn.start, turn.source.end));
                turn.id.clone()
            });
        owners.insert(turn.span_id.as_str(), id);
    }
    let mut timeline: Vec<_> = model
        .turns
        .iter()
        .cloned()
        .chain(
            build
                .turns
                .iter()
                .map(|turn| (turn.start_time, turn.request_id.clone())),
        )
        .collect();
    timeline.sort();
    let mut unplaced = Vec::new();
    for Step {
        speaker,
        turn,
        step,
    } in steps
    {
        let owner = match (turn, speaker) {
            (Some(turn), _) => Some(&owners[turn.span_id.as_str()]),
            (None, Speaker::Agent) => timeline
                .iter()
                .rev()
                .find(|(start, _)| *start <= step.start_time)
                .map(|(_, id)| id),
            (None, Speaker::User) => timeline
                .iter()
                .find(|(start, _)| *start > step.start_time)
                .map(|(_, id)| id),
        };
        match owner {
            Some(id) => build.work.push((id.clone(), step)),
            None => unplaced.push(step),
        }
    }
    if let Some(first) = unplaced.first() {
        let id = first.id.clone();
        let end = unplaced.iter().filter_map(|step| step.end_time).max();
        build
            .turns
            .push(voice_turn_shell(&id, first.start_time, end));
        build
            .work
            .extend(unplaced.into_iter().map(|step| (id.clone(), step)));
    }
    build
}

fn voice_turn_shell(id: &str, start: DateTime<Utc>, end: Option<DateTime<Utc>>) -> Turn {
    Turn {
        request_id: id.to_string(),
        response_id: None,
        request: None,
        opaque_request: Vec::new(),
        response: None,
        work: Vec::new(),
        model: None,
        params: None,
        start_time: start,
        end_time: end,
        interrupted: None,
        compaction: None,
    }
}

/// The voice turn span that the span's `turn.id` names.
fn voice_turn<'a>(
    span: &PreparedSpan,
    by_span_id: &HashMap<&str, &'a PreparedSpan>,
    warn: &mut impl FnMut(String),
) -> Option<&'a PreparedSpan> {
    let turn_id = span.source.voice.turn_id.as_deref()?;
    match by_span_id.get(turn_id) {
        Some(turn) if matches!(turn.source.voice.kind, Some(VoiceKind::Turn(_))) => Some(turn),
        _ => {
            warn(format!(
                "Unresolved voice turn.id {turn_id}: no voice turn has that span ID"
            ));
            None
        }
    }
}

fn audio_clip(
    selection: &SelectionMetadata,
    by_span_id: &HashMap<&str, &PreparedSpan>,
    recordings: &HashMap<(&str, &str), Option<Recording>>,
    warn: &mut impl FnMut(String),
) -> Option<AudioClip> {
    let Some(span_id) = selection
        .recording_span_id
        .as_deref()
        .filter(|span_id| by_span_id.contains_key(span_id))
    else {
        warn(format!(
            "Unresolved audio selection for recording {}: recording_span_id is not in this trace",
            selection.recording_id
        ));
        return None;
    };
    let Some(recording) = recordings.get(&(span_id, selection.recording_id.as_str())) else {
        warn(format!(
            "Unresolved audio selection for recording {}: span {span_id} does not list it",
            selection.recording_id
        ));
        return None;
    };
    let recording = recording.as_ref()?;
    Some(AudioClip {
        recording_id: recording.row.to_string(),
        attachment_ref: recording.attachment_ref.to_string(),
        mime_type: recording.metadata.mime_type.clone(),
        channel_index: selection.channel_index,
        channel: recording
            .metadata
            .sources
            .iter()
            .find(|source| source.channel_index == selection.channel_index)
            .map(|source| source.boundary.clone()),
        start_offset_ms: selection.start_offset_ms,
        end_offset_ms: selection.end_offset_ms,
    })
}

/// A ready recording, or `None` while the source is still writing it.
fn audio_recording<'a>(
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
    Ok(Some(Recording {
        row: &row.id,
        metadata: recording,
        attachment_ref: &attachment.reference,
    }))
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
