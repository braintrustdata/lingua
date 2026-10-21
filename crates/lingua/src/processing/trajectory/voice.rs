use super::PreparedSpan;
use crate::processing::import::{RecordingMetadata, Speaker, VoiceKind};
use crate::universal::trajectory::{AudioRecording, AudioSelection, Utterance, Voice, VoiceTurn};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use std::collections::{BTreeMap, HashMap, HashSet};

/// A trace's voice projection and the diagnostics found while building it.
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

/// Builds voice from recorded ownership and associations only: `turn.id` and span
/// ancestry, never timing or speaker alternation.
pub(super) fn build(spans: &[PreparedSpan]) -> BTreeMap<String, VoiceBuild> {
    let mut roots = BTreeMap::<&str, Vec<usize>>::new();
    for (index, span) in spans.iter().enumerate() {
        roots
            .entry(span.root_span_id.as_str())
            .or_default()
            .push(index);
    }
    roots
        .into_iter()
        .filter_map(|(root, indices)| {
            build_root(spans, &indices).map(|build| (root.to_string(), build))
        })
        .collect()
}

fn build_root(spans: &[PreparedSpan], indices: &[usize]) -> Option<VoiceBuild> {
    if !indices.iter().any(|index| {
        let voice = &spans[*index].source.voice;
        voice.kind.is_some() || !voice.recordings.is_empty()
    }) {
        return None;
    }
    let mut indices = indices.to_vec();
    indices.sort_by(|a, b| {
        let (a, b) = (&spans[*a], &spans[*b]);
        a.start
            .cmp(&b.start)
            .then(a.source.exec_counter.cmp(&b.source.exec_counter))
            .then(a.id.cmp(&b.id))
    });
    let by_span_id: HashMap<&str, usize> = indices
        .iter()
        .map(|index| (spans[*index].span_id.as_str(), *index))
        .collect();
    let mut warnings = Vec::new();
    for index in &indices {
        let span = &spans[*index];
        warnings.extend(span.source.voice.errors.iter().map(|message| VoiceWarning {
            span_id: span.id.clone(),
            message: message.clone(),
        }));
    }
    let mut owners = HashMap::new();
    for &index in &indices {
        let span = &spans[index];
        let tracked = span.source.voice.kind.is_some()
            || matches!(span.kind(), "llm" | "tool") && !span.is_scorer();
        if !tracked {
            continue;
        }
        match owner(spans, &by_span_id, index) {
            Ok(Some(turn)) => {
                owners.insert(index, turn);
            }
            Ok(None) => {}
            Err(message) => warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message,
            }),
        }
    }
    let mut recordings = Vec::new();
    for &index in &indices {
        let span = &spans[index];
        for recording in &span.source.voice.recordings {
            match audio_recording(recording, spans, &by_span_id) {
                Ok(Some(recording)) => recordings.push(recording),
                Ok(None) => {}
                Err(message) => warnings.push(VoiceWarning {
                    span_id: span.id.clone(),
                    message: format!("Unsupported audio recording {}: {message}", recording.id),
                }),
            }
        }
    }
    recordings.sort_by(|a, b| a.start_unix_ms.total_cmp(&b.start_unix_ms));
    let mut utterances = Vec::new();
    let mut turns = Vec::new();
    let mut positions = HashMap::new();
    let mut text_warnings = Vec::new();
    for &index in &indices {
        let span = &spans[index];
        match span.source.voice.kind {
            Some(VoiceKind::Speech(speaker)) => {
                let text = match speech_text(span, speaker) {
                    Ok(text) => text,
                    Err(message) => {
                        text_warnings.push(VoiceWarning {
                            span_id: span.id.clone(),
                            message,
                        });
                        None
                    }
                };
                let mut audio = Vec::new();
                for selection in span.source.voice.selections.iter().flatten() {
                    let row = selection
                        .recording_span_id
                        .as_deref()
                        .and_then(|span_id| by_span_id.get(span_id));
                    let Some(&row) = row else {
                        warnings.push(VoiceWarning {
                            span_id: span.id.clone(),
                            message: format!(
                                "Unresolved audio selection for recording {}: recording_span_id is not in this trace",
                                selection.recording_id
                            ),
                        });
                        continue;
                    };
                    audio.push(AudioSelection {
                        row_id: spans[row].id.clone(),
                        recording_id: selection.recording_id.clone(),
                        channel: selection.channel_index,
                        start_ms: selection.start_offset_ms,
                        end_ms: selection.end_offset_ms,
                    });
                }
                utterances.push(Utterance {
                    id: span.id.clone(),
                    speaker: speaker.as_str().to_string(),
                    start_time: span.start,
                    end_time: span.source.end,
                    text,
                    interrupted: span.source.voice.interrupted,
                    audio,
                });
            }
            Some(VoiceKind::Turn(speaker)) => {
                positions.insert(index, turns.len());
                turns.push(VoiceTurn {
                    id: span.id.clone(),
                    speaker: speaker.as_str().to_string(),
                    start_time: span.start,
                    end_time: span.source.end,
                    utterance_ids: Vec::new(),
                    step_ids: Vec::new(),
                });
            }
            Some(VoiceKind::Operation) | None => {}
        }
    }
    for &index in &indices {
        let span = &spans[index];
        let turn = owners.get(&index).map(|turn| positions[turn]);
        match (span.source.voice.kind, turn) {
            (Some(VoiceKind::Speech(_)), Some(turn)) => {
                turns[turn].utterance_ids.push(span.id.clone());
            }
            (Some(VoiceKind::Operation) | None, Some(turn)) => {
                turns[turn].step_ids.push(span.id.clone());
            }
            (Some(VoiceKind::Operation), None) => warnings.push(VoiceWarning {
                span_id: span.id.clone(),
                message: format!(
                    "Unsupported voice operation {}: no recorded native voice turn owns it",
                    span.source.name.as_deref().unwrap_or_default()
                ),
            }),
            _ => {}
        }
    }
    warnings.extend(text_warnings);
    Some(VoiceBuild {
        voice: Voice {
            recordings,
            utterances,
            turns,
        },
        warnings,
    })
}

/// The native voice turn named by `turn.id` and the nearest native turn ancestor must agree.
fn owner(
    spans: &[PreparedSpan],
    by_span_id: &HashMap<&str, usize>,
    index: usize,
) -> Result<Option<usize>, String> {
    let is_turn = |index: usize| matches!(spans[index].source.voice.kind, Some(VoiceKind::Turn(_)));
    let recorded = match &spans[index].source.voice.turn_id {
        Some(turn_id) => match by_span_id.get(turn_id.as_str()) {
            Some(&turn) if is_turn(turn) => Some(turn),
            _ => {
                return Err(format!(
                    "Unresolved voice turn.id {turn_id}: no native voice turn has that span ID"
                ))
            }
        },
        None => None,
    };
    let mut ancestor = None;
    let mut current = Some(index);
    let mut visited = HashSet::new();
    while let Some(candidate) = current {
        if !visited.insert(candidate) {
            return Err("Cycle in voice span parents".to_string());
        }
        if is_turn(candidate) {
            ancestor = Some(candidate);
            break;
        }
        current = spans[candidate]
            .source
            .span_parents
            .first()
            .and_then(|parent| by_span_id.get(parent.as_str()).copied());
    }
    match (recorded, ancestor) {
        (Some(recorded), Some(ancestor)) if recorded != ancestor => Err(format!(
            "Conflicting voice turn ownership: turn.id names {} but the nearest native voice turn ancestor is {}",
            spans[recorded].id, spans[ancestor].id
        )),
        (recorded, ancestor) => Ok(recorded.or(ancestor)),
    }
}

/// A playable recording, or `None` while the source reports it as not yet ready.
fn audio_recording(
    recording: &RecordingMetadata,
    spans: &[PreparedSpan],
    by_span_id: &HashMap<&str, usize>,
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
    let mut sources: Vec<_> = recording.sources.iter().collect();
    sources.sort_by_key(|source| source.channel_index);
    let indexed = sources
        .iter()
        .enumerate()
        .all(|(index, source)| source.channel_index == Some(index as u32));
    if !indexed && !matches!(sources.as_slice(), [source] if source.channel_index.is_none()) {
        return Err("channel indexes must be 0 through n - 1".to_string());
    }
    Ok(Some(AudioRecording {
        row_id: spans[*row].id.clone(),
        recording_id: recording.id.clone(),
        attachment_ref: attachment.reference.clone(),
        start_unix_ms: timeline.origin_unix_ms + timeline.recording_start_offset_ms,
        mime_type: recording.mime_type.clone(),
        duration_ms: recording.duration_ms,
        channels: sources
            .iter()
            .map(|source| source.boundary.clone())
            .collect(),
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

#[cfg(test)]
mod stream_tests;
#[cfg(test)]
mod tests;
