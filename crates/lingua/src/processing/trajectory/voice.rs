use super::PreparedSpan;
use crate::processing::import::{Speaker, VoiceKind};
use crate::universal::trajectory::{AudioClip, Utterance, Voice, VoiceStep, VoiceTurn};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use std::collections::{BTreeMap, HashMap};

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

/// Builds voice for traces with voice spans. Ownership comes only from `turn.id`.
pub(super) fn build(spans: &[PreparedSpan]) -> BTreeMap<String, VoiceBuild> {
    let mut roots = BTreeMap::<&str, Vec<&PreparedSpan>>::new();
    for span in spans {
        roots
            .entry(span.root_span_id.as_str())
            .or_default()
            .push(span);
    }
    roots
        .into_iter()
        .filter(|(_, spans)| spans.iter().any(|span| span.source.voice.kind.is_some()))
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
                let text = match speech_text(span, speaker) {
                    Ok(Some(text)) if !text.is_empty() => Some(text),
                    Ok(_) => span.source.voice.fallback_text.clone(),
                    Err(message) => {
                        warn(message);
                        None
                    }
                };
                let mut audio = Vec::new();
                for selection in span.source.voice.selections.iter().flatten() {
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
                voice.utterances.push(Utterance {
                    id: span.id.clone(),
                    speaker: speaker.as_str().to_string(),
                    start_time: span.start,
                    end_time: span.source.end,
                    text,
                    interrupted: span.source.voice.interrupted,
                    audio,
                    turn_id: turn_id(span, &by_span_id, &mut warn),
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
    VoiceBuild { voice, warnings }
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

#[cfg(test)]
mod tests;
