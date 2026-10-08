use super::{ImportFailure, Ownership, PreparedSpan};
use crate::universal::trajectory::{AudioRecording, AudioSelection, Speaker, Speech, VoiceCall};
use chrono::DateTime;
use std::collections::HashMap;

/// Builds each trace's voice calls from span headers, keyed by root span id.
///
/// A call is a span that logged recordings. Speech comes from LiveKit `user_turn` and
/// `assistant_turn` spans, and links to the steps among that span and its children.
pub(super) fn voice_calls(
    spans: &[PreparedSpan],
    parents: &[Option<usize>],
    owners: &HashMap<usize, Ownership>,
    is_step: &[bool],
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

    for &index in &order {
        let span = &spans[index];
        let Some(voice) = &span.source.voice else {
            continue;
        };
        let speaker = match (span.kind(), span.source.name.as_deref()) {
            ("task", Some("user_turn")) => Speaker::User,
            ("task", Some("assistant_turn")) => Speaker::Agent,
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
        let resolves = voice.selections.iter().all(|selection| {
            selection.recording_span_id == first.recording_span_id
                && call
                    .recordings
                    .iter()
                    .any(|recording| recording.id == selection.recording_id)
        });
        if !resolves {
            fail(span, "Speech refers to a missing recording".to_string());
            continue;
        }
        let step_ids = order
            .iter()
            .filter(|step| is_step[**step] && (**step == index || parents[**step] == Some(index)))
            .map(|step| spans[*step].id.clone())
            .collect();
        call.speech.push(Speech {
            id: span.id.clone(),
            speaker,
            step_ids,
            selections: voice
                .selections
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
    calls
}
