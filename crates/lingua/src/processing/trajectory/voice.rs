use super::{ImportFailure, Ownership, PreparedSpan};
use crate::processing::import::LoggedSelection;
use crate::universal::trajectory::{AudioRecording, AudioSelection, Speaker, Speech, VoiceCall};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

struct Utterance<'a> {
    span: &'a PreparedSpan,
    selections: &'a [LoggedSelection],
    start: Option<DateTime<Utc>>,
}

/// Builds each trace's voice calls from span headers, keyed by root span id.
///
/// A call is a span that logged recordings. Speech comes from LiveKit `user_turn` and
/// `assistant_turn` spans, one per utterance when the turn logs speaking spans. Agent speech
/// links to the LLM call that produced it and user speech to the LLM call that answered it;
/// without speaking spans, speech links to the steps among the turn span and its children.
pub(super) fn voice_calls(
    spans: &[PreparedSpan],
    parents: &[Option<usize>],
    owners: &HashMap<usize, Ownership>,
    is_step: &[bool],
    is_conversation: &[bool],
    failures: &mut Vec<ImportFailure>,
) -> HashMap<String, Vec<VoiceCall>> {
    let mut order: Vec<_> = (0..spans.len())
        .filter(|index| !owners[index].skipped)
        .collect();
    order.sort_by_key(|index| (spans[*index].start, &spans[*index].id));
    let row_ids: HashMap<_, _> = spans
        .iter()
        .map(|span| ((&span.root_span_id, &span.span_id), &span.id))
        .collect();
    let mut fail = |span: &PreparedSpan, message: String| {
        failures.push(ImportFailure {
            root_span_id: span.root_span_id.clone(),
            span_id: span.id.clone(),
            message,
        })
    };

    let mut calls: HashMap<String, Vec<VoiceCall>> = HashMap::new();
    for &index in &order {
        let span = &spans[index];
        let Some(voice) = &span.source.voice else {
            continue;
        };
        if voice.recordings.is_empty() {
            continue;
        }
        let mut recordings = Vec::new();
        for recording in &voice.recordings {
            let Some(attachment_span_id) =
                row_ids.get(&(&span.root_span_id, &recording.attachment.span_id))
            else {
                let message = format!("Recording {} is attached to a missing span", recording.id);
                fail(span, message);
                continue;
            };
            recordings.push(AudioRecording {
                id: recording.id.clone(),
                attachment_span_id: attachment_span_id.to_string(),
                attachment_pointer: recording.attachment.pointer.clone(),
                start_time: recording.timeline.as_ref().and_then(|timeline| {
                    let start_ms = timeline.origin_unix_ms + timeline.recording_start_offset_ms;
                    DateTime::from_timestamp_millis(start_ms as i64)
                }),
                duration_ms: recording.duration_ms,
            });
        }
        calls
            .entry(span.root_span_id.clone())
            .or_default()
            .push(VoiceCall {
                id: span.id.clone(),
                recordings,
                speech: Vec::new(),
            });
    }

    let is_utterance = |index: usize, speaking_name: &str| {
        spans[index].source.name.as_deref() == Some(speaking_name)
            && spans[index]
                .source
                .voice
                .as_ref()
                .is_some_and(|voice| !voice.selections.is_empty())
    };
    for &index in &order {
        let span = &spans[index];
        let Some(voice) = &span.source.voice else {
            continue;
        };
        let (speaker, speaking_name) = match (span.kind(), span.source.name.as_deref()) {
            ("task", Some("user_turn")) => (Speaker::User, "livekit.user_speaking"),
            ("task", Some("assistant_turn")) => (Speaker::Agent, "livekit.agent_speaking"),
            _ => continue,
        };
        let Some(first) = voice.selections.first() else {
            continue;
        };
        // Recordings may not be logged yet, as in a call still in progress.
        let Some(call) = row_ids
            .get(&(&span.root_span_id, &first.recording_span_id))
            .and_then(|call_id| {
                calls
                    .get_mut(&span.root_span_id)?
                    .iter_mut()
                    .find(|call| &call.id == *call_id)
            })
        else {
            continue;
        };
        let recording_starts: HashMap<_, _> = call
            .recordings
            .iter()
            .filter_map(|recording| Some((recording.id.as_str(), recording.start_time?)))
            .collect();
        let wall_time = |selection: &LoggedSelection, offset_ms: f64| {
            recording_starts
                .get(selection.recording_id.as_str())
                .map(|start| *start + Duration::microseconds((offset_ms * 1000.0) as i64))
        };
        let turn_steps: Vec<_> = order
            .iter()
            .copied()
            .filter(|step| is_step[*step] && (*step == index || parents[*step] == Some(index)))
            .collect();
        // One speech per utterance. LiveKit logs each on a speaking span under the turn task;
        // without those, the turn's selections split where the audio pauses.
        let speaking: Vec<_> = order
            .iter()
            .copied()
            .filter(|child| parents[*child] == Some(index) && is_utterance(*child, speaking_name))
            .collect();
        let mut utterances = Vec::new();
        if speaking.is_empty() {
            let selections = &voice.selections[..];
            let mut run = 0;
            for next in 1..=selections.len() {
                let pause = next == selections.len()
                    || match (
                        wall_time(&selections[next - 1], selections[next - 1].end_offset_ms),
                        wall_time(&selections[next], selections[next].start_offset_ms),
                    ) {
                        (Some(end), Some(start)) => start > end,
                        _ => false,
                    };
                if pause {
                    let start = wall_time(&selections[run], selections[run].start_offset_ms);
                    utterances.push(Utterance {
                        span,
                        selections: &selections[run..next],
                        start,
                    });
                    run = next;
                }
            }
        } else {
            for &child in &speaking {
                let child_span = &spans[child];
                let selections = &child_span.source.voice.as_ref().unwrap().selections[..];
                utterances.push(Utterance {
                    span: child_span,
                    selections,
                    start: Some(child_span.start),
                });
            }
        }
        let split = utterances.len() > 1 || !speaking.is_empty();
        for Utterance {
            span: utterance_span,
            selections,
            start,
        } in utterances
        {
            let resolves = selections.iter().all(|selection| {
                selection.recording_span_id == first.recording_span_id
                    && call
                        .recordings
                        .iter()
                        .any(|recording| recording.id == selection.recording_id)
            });
            if !resolves {
                fail(
                    utterance_span,
                    "Speech refers to a missing recording".to_string(),
                );
                continue;
            }
            let step = match (speaker, start.filter(|_| split)) {
                (_, None) => None,
                // The LLM call that produced the utterance, which starts before it is played.
                (Speaker::Agent, Some(start)) => order.iter().copied().rev().find(|step| {
                    is_conversation[*step]
                        && spans[*step].start <= start
                        && std::iter::successors(parents[*step], |parent| parents[*parent])
                            .any(|ancestor| ancestor == index)
                }),
                // The first LLM call after the utterance starts heard it, unless the user spoke
                // again first.
                (Speaker::User, Some(start)) => {
                    let next_utterance = order.iter().copied().find(|other| {
                        spans[*other].root_span_id == span.root_span_id
                            && spans[*other].start > start
                            && is_utterance(*other, speaking_name)
                    });
                    order.iter().copied().find(|step| {
                        is_conversation[*step]
                            && spans[*step].root_span_id == span.root_span_id
                            && spans[*step].start >= start
                            && next_utterance
                                .is_none_or(|next| spans[*step].start < spans[next].start)
                    })
                }
            };
            let step_ids = match step {
                Some(step) => vec![spans[step].id.clone()],
                None => turn_steps
                    .iter()
                    .map(|step| spans[*step].id.clone())
                    .collect(),
            };
            call.speech.push(Speech {
                id: utterance_span.id.clone(),
                speaker,
                step_ids,
                selections: selections
                    .iter()
                    .map(|selection| AudioSelection {
                        recording_id: selection.recording_id.clone(),
                        channel_index: selection.channel_index,
                        start_offset_ms: selection.start_offset_ms,
                        end_offset_ms: selection.end_offset_ms,
                    })
                    .collect(),
                interrupted: voice.interrupted,
            });
        }
    }
    let starts: HashMap<_, _> = spans.iter().map(|span| (&span.id, span.start)).collect();
    for call in calls.values_mut().flatten() {
        let recording_starts: HashMap<_, _> = call
            .recordings
            .iter()
            .filter_map(|recording| Some((recording.id.clone(), recording.start_time?)))
            .collect();
        let start = |speech: &Speech| {
            speech
                .selections
                .iter()
                .filter_map(|selection| {
                    let start = recording_starts.get(&selection.recording_id)?;
                    Some(
                        *start
                            + Duration::microseconds((selection.start_offset_ms * 1000.0) as i64),
                    )
                })
                .min()
                .unwrap_or(starts[&speech.id])
        };
        call.speech.sort_by_cached_key(|speech| start(speech));
    }
    calls
}
