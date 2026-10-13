use super::{ImportFailure, Ownership, PreparedSpan};
use crate::processing::import::{LoggedSelection, RecordingState, SourceBoundary, SpanVoice};
use crate::universal::trajectory::{
    AudioRecording, AudioSelection, Speaker, Speech, TimeBasis, VoiceCall, VoiceEvent,
    VoiceEventDetail,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

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
///
/// A call span with `voice.timeline_version` instead builds its speech from the `voice.timeline`
/// rows logged under it, and its turn spans are ignored. A call span with `voice.points_version`
/// does the same from the `voice.points` logged on it and on the spans under it.
pub(super) fn voice_calls(
    spans: &[PreparedSpan],
    parents: &[Option<usize>],
    owners: &HashMap<usize, Ownership>,
    is_step: &[bool],
    is_conversation: &[bool],
    failures: &mut Vec<ImportFailure>,
    warnings: &mut Vec<ImportFailure>,
) -> HashMap<String, Vec<VoiceCall>> {
    let mut order: Vec<_> = (0..spans.len())
        .filter(|index| !owners[index].skipped)
        .collect();
    order.sort_by_key(|index| (spans[*index].start, &spans[*index].id));
    let row_ids: HashMap<_, _> = spans
        .iter()
        .map(|span| ((&span.root_span_id, &span.span_id), &span.id))
        .collect();
    let mut fail = |span: &PreparedSpan, message: String| failures.push(diagnostic(span, message));
    let mut warn = |span: &PreparedSpan, message: String| warnings.push(diagnostic(span, message));
    let mut timeline_rows: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for &index in &order {
        if let Some(chunk) = spans[index]
            .source
            .voice
            .as_ref()
            .and_then(|voice| voice.timeline.as_ref())
        {
            timeline_rows
                .entry((&spans[index].root_span_id, &chunk.call_span_id))
                .or_default()
                .push(&spans[index]);
        }
    }
    let mut timeline_calls = HashSet::new();
    let points_calls: HashSet<usize> = order
        .iter()
        .copied()
        .filter(|index| {
            spans[*index]
                .source
                .voice
                .as_ref()
                .is_some_and(|voice| voice.points_version.is_some())
        })
        .collect();
    let mut point_owners: HashMap<usize, Vec<&PreparedSpan>> = HashMap::new();
    for &index in &order {
        if spans[index]
            .source
            .voice
            .as_ref()
            .is_none_or(|voice| voice.points.is_empty())
        {
            continue;
        }
        match std::iter::successors(Some(index), |child| parents[*child])
            .find(|ancestor| points_calls.contains(ancestor))
        {
            Some(call) => point_owners.entry(call).or_default().push(&spans[index]),
            None => warn(
                &spans[index],
                "voice points without a versioned call span".to_string(),
            ),
        }
    }

    let mut calls: HashMap<String, Vec<VoiceCall>> = HashMap::new();
    for &index in &order {
        let span = &spans[index];
        let Some(voice) = &span.source.voice else {
            continue;
        };
        if voice.recordings.is_empty()
            && voice.timeline_version.is_none()
            && voice.points_version.is_none()
        {
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
        let mut call = VoiceCall {
            id: span.id.clone(),
            recordings,
            speech: Vec::new(),
            events: None,
        };
        if let Some(version) = voice.timeline_version {
            timeline_calls.insert(span.id.clone());
            let rows = timeline_rows
                .remove(&(&span.root_span_id, &span.span_id))
                .unwrap_or_default();
            if version != 1 {
                fail(
                    span,
                    format!("unsupported voice timeline version {version}"),
                );
            } else {
                build_timeline(
                    &mut call, span, voice, &rows, &row_ids, &mut fail, &mut warn,
                );
            }
        } else if let Some(version) = voice.points_version {
            timeline_calls.insert(span.id.clone());
            if version != 1 {
                fail(span, format!("unsupported voice points version {version}"));
            } else {
                let owners = point_owners.remove(&index).unwrap_or_default();
                build_points(
                    &mut call, span, voice, &owners, &row_ids, &mut fail, &mut warn,
                );
            }
        }
        calls
            .entry(span.root_span_id.clone())
            .or_default()
            .push(call);
    }
    for rows in timeline_rows.values() {
        warn(
            rows[0],
            "timeline rows without a versioned call span".to_string(),
        );
    }
    let mut ignored_calls = BTreeSet::new();

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
        if timeline_calls.contains(&call.id) {
            ignored_calls.insert(call.id.clone());
            continue;
        }
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
    for call in spans.iter().filter(|span| ignored_calls.contains(&span.id)) {
        warn(
            call,
            "turn span voice metadata is ignored for a timeline call".to_string(),
        );
    }
    let starts: HashMap<_, _> = spans.iter().map(|span| (&span.id, span.start)).collect();
    for call in calls
        .values_mut()
        .flatten()
        .filter(|call| !timeline_calls.contains(&call.id))
    {
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

fn diagnostic(span: &PreparedSpan, message: String) -> ImportFailure {
    ImportFailure {
        root_span_id: span.root_span_id.clone(),
        span_id: span.id.clone(),
        message,
    }
}

/// Builds a timeline call's events and speech from its `voice.timeline` rows.
fn build_timeline(
    call: &mut VoiceCall,
    span: &PreparedSpan,
    voice: &SpanVoice,
    rows: &[&PreparedSpan],
    row_ids: &HashMap<(&String, &String), &String>,
    fail: &mut impl FnMut(&PreparedSpan, String),
    warn: &mut impl FnMut(&PreparedSpan, String),
) {
    let mut chunks = BTreeMap::new();
    for row in rows {
        let Some(row_voice) = &row.source.voice else {
            continue;
        };
        let Some(chunk) = &row_voice.timeline else {
            continue;
        };
        match chunks.get(&chunk.chunk_index) {
            None => {
                chunks.insert(chunk.chunk_index, row_voice);
            }
            Some(other)
                if other.timeline == row_voice.timeline && other.events == row_voice.events => {}
            Some(_) => {
                fail(
                    row,
                    format!("Conflicting voice timeline chunk {}", chunk.chunk_index),
                );
                return;
            }
        }
    }
    let mut events: Vec<VoiceEvent> = chunks
        .values()
        .flat_map(|chunk| chunk.events.iter().cloned())
        .collect();
    events.sort_by_key(|event| event.seq);
    if let Some(first) = events.first().filter(|first| first.seq > 0) {
        warn(span, format!("timeline gap before seq {}", first.seq));
    }
    for pair in events.windows(2) {
        if pair[1].seq > pair[0].seq + 1 {
            warn(span, format!("timeline gap after seq {}", pair[0].seq));
        }
    }
    if !chunks.values().any(|chunk| {
        chunk
            .timeline
            .as_ref()
            .is_some_and(|timeline| timeline.is_final)
    }) {
        warn(
            span,
            "call timeline is in progress or the session crashed".to_string(),
        );
    }
    if voice.recordings.is_empty() && events.iter().any(|event| event.basis == TimeBasis::Audio) {
        warn(
            span,
            "audio basis without a recording is treated as local".to_string(),
        );
    }
    call.speech = timeline_speech(span, voice, call, &events, row_ids, &HashSet::new(), warn);
    call.events = Some(events);
}

/// The span kinds that may own each point, by role: who observed the fact, not which operation
/// was running when it happened.
fn point_owner_allowed(kind: &str, role: &str) -> bool {
    match kind {
        "end_of_turn" => role == "user_turn",
        "response_start" => role == "llm",
        "tool_call" | "tool_result" => role == "tool",
        "truncate" => role == "assistant_turn",
        "call_end" => role == "call",
        _ => matches!(role, "user_turn" | "assistant_turn" | "call"),
    }
}

/// Builds a points call's events and speech from the `voice.points` on its spans.
///
/// Each point becomes a timeline event: its key is its `seq`, and the owner span supplies the
/// `span_id` of `response_start`, `tool_call` and `tool_result` and the tool `name`. Speech is then
/// built as for a timeline, except that a caller utterance whose `user_turn` holds no
/// `end_of_turn` was never committed and links to no step.
fn build_points(
    call: &mut VoiceCall,
    span: &PreparedSpan,
    voice: &SpanVoice,
    owners: &[&PreparedSpan],
    row_ids: &HashMap<(&String, &String), &String>,
    fail: &mut impl FnMut(&PreparedSpan, String),
    warn: &mut impl FnMut(&PreparedSpan, String),
) {
    let mut points: BTreeMap<u32, (&PreparedSpan, &crate::serde_json::Value)> = BTreeMap::new();
    let mut events = Vec::new();
    let mut open_turns = Vec::new();
    for &owner in owners {
        let role = if owner.id == span.id {
            "call"
        } else {
            match (owner.kind(), owner.source.name.as_deref()) {
                ("task", Some(name @ ("user_turn" | "assistant_turn"))) => name,
                (kind, _) => kind,
            }
        };
        let mut turn = (Vec::new(), false);
        for (key, point) in &owner.source.voice.as_ref().unwrap().points {
            let Some(seq) = key
                .parse::<u32>()
                .ok()
                .filter(|seq| seq.to_string() == *key)
            else {
                warn(owner, format!("voice point key {key} is not a seq"));
                continue;
            };
            match points.get(&seq) {
                Some((_, other)) if *other == point => continue,
                Some(_) => {
                    fail(owner, format!("Conflicting voice point {seq}"));
                    return;
                }
                None => {
                    points.insert(seq, (owner, point));
                }
            }
            let mut point = point.clone();
            let Some(fields) = point.as_object_mut() else {
                warn(owner, format!("voice point {seq} is not an object"));
                continue;
            };
            let kind = fields
                .get("kind")
                .and_then(|kind| kind.as_str())
                .unwrap_or_default()
                .to_string();
            if !point_owner_allowed(&kind, role) {
                warn(
                    owner,
                    format!("voice point {seq} ({kind}) on a {role} span"),
                );
            }
            fields.insert("seq".to_string(), seq.into());
            if matches!(
                kind.as_str(),
                "response_start" | "tool_call" | "tool_result"
            ) {
                fields.insert("span_id".to_string(), owner.span_id.clone().into());
            }
            if kind == "tool_call" {
                let name = owner.source.name.clone().unwrap_or_default();
                fields.insert("name".to_string(), name.into());
            }
            match crate::serde_json::from_value::<VoiceEvent>(point) {
                Ok(event) => {
                    if role == "user_turn" {
                        match &event.detail {
                            VoiceEventDetail::SpeechStart {
                                speaker: Speaker::User,
                                utterance_id,
                                ..
                            } => turn.0.push(utterance_id.clone()),
                            VoiceEventDetail::EndOfTurn { .. } => turn.1 = true,
                            _ => {}
                        }
                    }
                    events.push(event);
                }
                Err(error) => warn(owner, format!("invalid voice point {seq}: {error}")),
            }
        }
        if !turn.1 {
            open_turns.extend(turn.0);
        }
    }
    events.sort_by_key(|event| event.seq);
    if let Some(first) = events.first().filter(|first| first.seq > 0) {
        warn(span, format!("points gap before seq {}", first.seq));
    }
    for pair in events.windows(2) {
        if pair[1].seq > pair[0].seq + 1 {
            warn(span, format!("points gap after seq {}", pair[0].seq));
        }
    }
    let call_end = events
        .iter()
        .find(|event| matches!(event.detail, VoiceEventDetail::CallEnd { .. }));
    match call_end {
        None => warn(
            span,
            "call points are in progress or the session crashed".to_string(),
        ),
        Some(end) if events.last().is_some_and(|last| last.seq > end.seq) => {
            warn(span, "voice points after call_end".to_string())
        }
        Some(_) => {}
    }
    let uncommitted: HashSet<&str> = open_turns.iter().map(String::as_str).collect();
    call.speech = timeline_speech(span, voice, call, &events, row_ids, &uncommitted, warn);
    call.events = Some(events);
}

struct TimelineSpeech<'a> {
    start: &'a VoiceEvent,
    speaker: Speaker,
    utterance_id: &'a str,
    response_id: Option<&'a str>,
}

fn timeline_speech(
    span: &PreparedSpan,
    voice: &SpanVoice,
    call: &VoiceCall,
    events: &[VoiceEvent],
    row_ids: &HashMap<(&String, &String), &String>,
    uncommitted: &HashSet<&str>,
    warn: &mut impl FnMut(&PreparedSpan, String),
) -> Vec<Speech> {
    let mut starts: Vec<TimelineSpeech> = Vec::new();
    let mut ends = HashMap::new();
    let mut end_of_turns = Vec::new();
    let mut response_starts = Vec::new();
    for event in events {
        match &event.detail {
            VoiceEventDetail::SpeechStart {
                speaker,
                utterance_id,
                response_id,
            } => {
                if starts
                    .iter()
                    .all(|start| start.utterance_id != utterance_id)
                {
                    starts.push(TimelineSpeech {
                        start: event,
                        speaker: *speaker,
                        utterance_id,
                        response_id: response_id.as_deref(),
                    });
                }
            }
            VoiceEventDetail::SpeechEnd { utterance_id, .. } => {
                ends.entry(utterance_id.as_str()).or_insert(event);
            }
            VoiceEventDetail::EndOfTurn { utterance_ids } => {
                end_of_turns.push((event.seq, utterance_ids));
            }
            VoiceEventDetail::ResponseStart {
                response_id,
                span_id,
                in_reply_to,
            } => response_starts.push((event.seq, event.at_ms, response_id, span_id, in_reply_to)),
            _ => {}
        }
    }
    let mut unmatched_ends = BTreeSet::new();
    for utterance_id in ends.keys() {
        if starts
            .iter()
            .all(|start| start.utterance_id != *utterance_id)
        {
            unmatched_ends.insert(*utterance_id);
        }
    }
    for utterance_id in unmatched_ends {
        warn(span, format!("speech_end without a start {utterance_id}"));
    }
    starts.sort_by(|a, b| {
        a.start
            .at_ms
            .total_cmp(&b.start.at_ms)
            .then(a.start.seq.cmp(&b.start.seq))
    });

    // Turn k holds the events after the k-th `end_of_turn`.
    let window = |seq: u32| {
        end_of_turns
            .iter()
            .filter(|(turn_seq, _)| *turn_seq < seq)
            .count()
            .checked_sub(1)
    };
    let mut utterance_turns = HashMap::new();
    for (turn, (_, utterance_ids)) in end_of_turns.iter().enumerate() {
        for utterance_id in utterance_ids.iter() {
            utterance_turns.entry(utterance_id.as_str()).or_insert(turn);
        }
    }
    for speech in &starts {
        if speech.speaker != Speaker::User
            || utterance_turns.contains_key(speech.utterance_id)
            || uncommitted.contains(speech.utterance_id)
        {
            continue;
        }
        match window(speech.start.seq) {
            Some(turn) => {
                utterance_turns.insert(speech.utterance_id, turn);
            }
            None => warn(
                span,
                format!(
                    "speech {} is before the first end of turn",
                    speech.utterance_id
                ),
            ),
        }
    }
    let mut missing_spans = BTreeSet::new();
    let mut turn_steps = vec![Vec::new(); end_of_turns.len()];
    // A speech handle can run several generations (LiveKit cascaded tool replies reuse it).
    let mut response_steps: HashMap<&str, Vec<_>> = HashMap::new();
    for (seq, at_ms, response_id, span_id, in_reply_to) in response_starts {
        let step = row_ids.get(&(&span.root_span_id, span_id)).copied();
        if step.is_none() && missing_spans.insert(span_id) {
            warn(span, format!("timeline refers to a missing span {span_id}"));
        }
        response_steps
            .entry(response_id.as_str())
            .or_default()
            .push((at_ms, step));
        let turns: BTreeSet<_> = match in_reply_to.as_deref().filter(|ids| !ids.is_empty()) {
            Some(ids) => ids
                .iter()
                .filter_map(|id| utterance_turns.get(id.as_str()).copied())
                .collect(),
            None => window(seq).into_iter().collect(),
        };
        if turns.len() > 1 {
            warn(
                span,
                format!("response {response_id} replies to several turns"),
            );
        }
        if turns.is_empty() {
            warn(
                span,
                format!("response {response_id} is not linked to a turn"),
            );
        }
        for turn in turns {
            if let Some(step) = step.filter(|step| !turn_steps[turn].contains(step)) {
                turn_steps[turn].push(step);
            }
        }
    }

    let call_end = events.iter().find_map(|event| {
        matches!(event.detail, VoiceEventDetail::CallEnd { .. }).then_some(event.at_ms)
    });
    let last_at = events.iter().map(|event| event.at_ms).reduce(f64::max);
    let mut speech = Vec::new();
    for (position, utterance) in starts.iter().enumerate() {
        let end = ends.get(utterance.utterance_id);
        let end_ms = match end {
            Some(end) => end.at_ms,
            None => {
                warn(span, format!("unclosed speech {}", utterance.utterance_id));
                let next = starts[position + 1..]
                    .iter()
                    .find(|next| next.speaker == utterance.speaker)
                    .map(|next| next.start.at_ms);
                next.or(call_end)
                    .or(last_at)
                    .unwrap_or(utterance.start.at_ms)
            }
        };
        let interrupted = end.and_then(|end| match &end.detail {
            VoiceEventDetail::SpeechEnd { interrupted, .. } => *interrupted,
            _ => None,
        });
        let step_ids = match (utterance.speaker, utterance.response_id) {
            // The handle's latest generation that started before playback.
            (Speaker::Agent, Some(response_id)) => response_steps
                .get(response_id)
                .and_then(|starts| {
                    starts
                        .iter()
                        .rev()
                        .find(|(at_ms, _)| *at_ms <= utterance.start.at_ms)
                        .or(starts.first())
                })
                .and_then(|(_, step)| *step)
                .into_iter()
                .cloned()
                .collect(),
            (Speaker::Agent, None) => window(utterance.start.seq)
                .map(|turn| turn_steps[turn].iter().copied().cloned().collect())
                .unwrap_or_default(),
            (Speaker::User, _) => utterance_turns
                .get(utterance.utterance_id)
                .map(|turn| turn_steps[*turn].iter().copied().cloned().collect())
                .unwrap_or_default(),
        };
        speech.push(Speech {
            id: utterance.utterance_id.to_string(),
            speaker: utterance.speaker,
            step_ids,
            selections: selections(
                span,
                voice,
                call,
                utterance.speaker,
                utterance.start.at_ms,
                end_ms,
                warn,
            ),
            interrupted,
        });
    }
    speech
}

/// Clips `[start_ms, end_ms]` on the call clock to each ready recording of the call.
fn selections(
    span: &PreparedSpan,
    voice: &SpanVoice,
    call: &VoiceCall,
    speaker: Speaker,
    start_ms: f64,
    end_ms: f64,
    warn: &mut impl FnMut(&PreparedSpan, String),
) -> Vec<AudioSelection> {
    let mut selections = Vec::new();
    for recording in &voice.recordings {
        if recording.state == Some(RecordingState::Other)
            || call.recordings.iter().all(|other| other.id != recording.id)
        {
            continue;
        }
        let (Some(timeline), Some(duration_ms)) = (&recording.timeline, recording.duration_ms)
        else {
            continue;
        };
        let recording_start = timeline.recording_start_offset_ms;
        let start = start_ms.max(recording_start);
        let end = end_ms.min(recording_start + duration_ms);
        if end <= start {
            continue;
        }
        let source = recording.sources.as_ref().and_then(|sources| {
            sources.iter().find(|source| {
                matches!(
                    (speaker, source.boundary),
                    (
                        Speaker::User,
                        SourceBoundary::SessionInput
                            | SourceBoundary::TransportInput
                            | SourceBoundary::ModelInput
                    ) | (
                        Speaker::Agent,
                        SourceBoundary::LocalAudioOutput | SourceBoundary::TransportOutput
                    )
                )
            })
        });
        let channel_index = match source {
            Some(source) => source.channel_index,
            None => {
                let channel_index = match speaker {
                    Speaker::User => 0,
                    Speaker::Agent => 1,
                };
                warn(
                    span,
                    format!(
                        "recording {} has no {} source; using channel {channel_index}",
                        recording.id,
                        match speaker {
                            Speaker::User => "user",
                            Speaker::Agent => "agent",
                        }
                    ),
                );
                channel_index
            }
        };
        selections.push(AudioSelection {
            recording_id: recording.id.clone(),
            channel_index: Some(channel_index),
            start_offset_ms: start - recording_start,
            end_offset_ms: end - recording_start,
        });
    }
    selections
}
