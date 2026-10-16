use super::PreparedSpan;
use crate::universal::trajectory::{
    AudioRecording, CallTimeline, TimelineClock, TimelineEntry, TimelineEntryKind,
    TimelineEvidence, TimelineGap, TimelineLane, TimelineLink, TimelineOverlap, TimingSource,
};
use crate::universal::{AssistantContent, AssistantContentPart, Message, UserContent};
use crate::universal::{TextContentPart, UserContentPart};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap, HashSet};

/// The lane of a speech span: a `task` named `user_speech` or `agent_speech`.
pub(super) fn speech_lane(span: &PreparedSpan) -> Option<TimelineLane> {
    if span.kind() != "task" {
        return None;
    }
    match span.source.name.as_deref() {
        Some("user_speech") => Some(TimelineLane::User),
        Some("agent_speech") => Some(TimelineLane::Agent),
        _ => None,
    }
}

/// Heard words of user speech from its input, or played words of agent speech from its output.
pub(super) fn speech_text(span: &PreparedSpan) -> Option<String> {
    let texts: Vec<String> = match speech_lane(span)? {
        TimelineLane::User => span
            .input
            .iter()
            .filter_map(|input| match &input.message {
                Message::User { content } => user_text(content),
                _ => None,
            })
            .collect(),
        _ => span
            .output
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { content, .. } => assistant_text(content),
                _ => None,
            })
            .collect(),
    };
    let text = texts.join("\n");
    (!text.is_empty()).then_some(text)
}

fn user_text(content: &UserContent) -> Option<String> {
    match content {
        UserContent::String(text) => Some(text.clone()),
        UserContent::Array(parts) => {
            let texts: Vec<_> = parts
                .iter()
                .filter_map(|part| match part {
                    UserContentPart::Text(TextContentPart { text, .. }) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            (!texts.is_empty()).then(|| texts.concat())
        }
    }
}

fn assistant_text(content: &AssistantContent) -> Option<String> {
    match content {
        AssistantContent::String(text) => Some(text.clone()),
        AssistantContent::Array(parts) => {
            let texts: Vec<_> = parts
                .iter()
                .filter_map(|part| match part {
                    AssistantContentPart::Text(TextContentPart { text, .. }) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            (!texts.is_empty()).then(|| texts.concat())
        }
    }
}

fn turn_of(span: &PreparedSpan) -> Option<&str> {
    span.source
        .voice
        .as_ref()
        .and_then(|voice| voice.turn_id.as_deref())
}

fn unix_ms(time: DateTime<Utc>) -> f64 {
    time.timestamp_micros() as f64 / 1000.0
}

fn round(ms: f64) -> f64 {
    (ms * 1000.0).round() / 1000.0
}

/// Builds the timeline of one trace from span headers, or None when the trace has no voice data.
pub(super) fn build(
    spans: &[&PreparedSpan],
    recordings: &[AudioRecording],
) -> Option<CallTimeline> {
    let has_events = spans.iter().any(|span| {
        span.source
            .voice
            .as_ref()
            .is_some_and(|voice| !voice.events.is_empty())
    });
    if recordings.is_empty() && !has_events && !spans.iter().any(|span| speech_lane(span).is_some())
    {
        return None;
    }
    let mut spans = spans.to_vec();
    spans.sort_by(|a, b| a.start.cmp(&b.start).then(a.id.cmp(&b.id)));
    let origins: HashMap<&str, f64> = recordings
        .iter()
        .filter_map(|recording| Some((recording.id.as_str(), recording.origin_unix_ms?)))
        .collect();
    let (clock, origin) = match origins.values().copied().min_by(f64::total_cmp) {
        Some(origin) => (TimelineClock::Recording, origin),
        None => (TimelineClock::SpanTime, unix_ms(spans.first()?.start)),
    };
    let at = |unix: f64| round(unix - origin);
    let by_span_id: HashMap<&str, &PreparedSpan> = spans
        .iter()
        .map(|span| (span.span_id.as_str(), *span))
        .collect();
    let linked: HashSet<&str> = spans
        .iter()
        .filter_map(|span| span.source.voice.as_ref())
        .flat_map(|voice| &voice.links)
        .filter_map(|link| by_span_id.get(link.span_id.as_str()))
        .map(|span| span.id.as_str())
        .collect();
    let mut turns: BTreeMap<&str, &PreparedSpan> = BTreeMap::new();
    let mut entries = Vec::new();
    let mut span_entries: HashMap<&str, &PreparedSpan> = HashMap::new();
    for &span in &spans {
        let voice = span.source.voice.as_ref();
        let span_interval = (
            at(unix_ms(span.start)),
            span.source.end.map(|end| at(unix_ms(end))),
        );
        let entry = |kind, lane, (start_ms, end_ms), timing_source| TimelineEntry {
            id: span.id.clone(),
            kind,
            lane,
            start_ms,
            end_ms,
            timing_source,
            label: span
                .source
                .name
                .clone()
                .unwrap_or_else(|| span.kind().to_string()),
            text: None,
            selections: Vec::new(),
            attributes: BTreeMap::new(),
        };
        if let Some(lane) = speech_lane(span) {
            let selections = voice
                .map(|voice| voice.selections.clone())
                .unwrap_or_default();
            let placed = (!selections.is_empty())
                .then(|| {
                    selections
                        .iter()
                        .map(|selection| {
                            let origin = origins.get(selection.recording_id.as_str())?;
                            Some((
                                origin + selection.start_offset_ms,
                                origin + selection.end_offset_ms,
                            ))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .flatten();
            let mut speech = match placed {
                Some(placed) => entry(
                    TimelineEntryKind::Speech,
                    lane,
                    (
                        at(placed
                            .iter()
                            .map(|(start, _)| *start)
                            .fold(f64::INFINITY, f64::min)),
                        Some(at(placed
                            .iter()
                            .map(|(_, end)| *end)
                            .fold(f64::NEG_INFINITY, f64::max))),
                    ),
                    TimingSource::AudioSelection,
                ),
                None => entry(
                    TimelineEntryKind::Speech,
                    lane,
                    span_interval,
                    TimingSource::SpanTime,
                ),
            };
            speech.selections = selections;
            entries.push(speech);
            span_entries.insert(&span.id, span);
        } else if matches!(span.kind(), "llm" | "tool")
            || linked.contains(span.id.as_str())
            || voice.is_some_and(|voice| !voice.links.is_empty())
        {
            entries.push(entry(
                TimelineEntryKind::Operation,
                TimelineLane::Operations,
                span_interval,
                TimingSource::SpanTime,
            ));
            span_entries.insert(&span.id, span);
        } else if let Some(turn_id) = turn_of(span).filter(|turn_id| {
            span.source
                .span_parents
                .first()
                .and_then(|parent| by_span_id.get(parent.as_str()))
                .and_then(|parent| turn_of(parent))
                != Some(*turn_id)
        }) {
            let mut turn = entry(
                TimelineEntryKind::FrameworkTurn,
                TimelineLane::Framework,
                span_interval,
                TimingSource::SpanTime,
            );
            turn.attributes
                .insert("turn_id".into(), turn_id.to_string());
            if let Some(reply_to) = voice.and_then(|voice| voice.reply_to.clone()) {
                turn.attributes.insert("reply_to".into(), reply_to);
            }
            entries.push(turn);
            span_entries.insert(&span.id, span);
            turns.entry(turn_id).or_insert(span);
        }
        for (index, event) in voice
            .into_iter()
            .flat_map(|voice| voice.events.iter().enumerate())
        {
            entries.push(TimelineEntry {
                id: format!("{}/events/{index}", span.id),
                kind: TimelineEntryKind::FrameworkEvent,
                lane: TimelineLane::Framework,
                start_ms: at(event.time_unix_ms),
                end_ms: None,
                timing_source: TimingSource::FrameworkEvent,
                label: event.name.clone(),
                text: None,
                selections: Vec::new(),
                attributes: event.attributes.clone(),
            });
        }
    }
    entries.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms).then(a.id.cmp(&b.id)));

    let mut links = Vec::new();
    for entry in &entries {
        let Some(span) = span_entries.get(entry.id.as_str()) else {
            continue;
        };
        let Some(voice) = &span.source.voice else {
            continue;
        };
        for link in &voice.links {
            links.push(TimelineLink {
                from: entry.id.clone(),
                to: by_span_id
                    .get(link.span_id.as_str())
                    .map(|target| target.id.clone()),
                target: link.span_id.clone(),
                relationship: link.relationship.clone(),
                evidence: TimelineEvidence::SpanLink,
            });
        }
        if entry.kind == TimelineEntryKind::FrameworkTurn {
            if let Some(reply_to) = &voice.reply_to {
                links.push(TimelineLink {
                    from: entry.id.clone(),
                    to: turns.get(reply_to.as_str()).map(|turn| turn.id.clone()),
                    target: reply_to.clone(),
                    relationship: "reply_to".into(),
                    evidence: TimelineEvidence::TurnReplyTo,
                });
            }
        }
    }

    let speech: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == TimelineEntryKind::Speech)
        .collect();
    let mut overlaps = Vec::new();
    for (position, a) in speech.iter().enumerate() {
        for b in &speech[position + 1..] {
            let (Some(a_end), Some(b_end)) = (a.end_ms, b.end_ms) else {
                continue;
            };
            let start_ms = a.start_ms.max(b.start_ms);
            let end_ms = a_end.min(b_end);
            if a.lane != b.lane && end_ms > start_ms {
                overlaps.push(TimelineOverlap {
                    a: a.id.clone(),
                    b: b.id.clone(),
                    start_ms,
                    end_ms,
                });
            }
        }
    }

    let by_id: HashMap<&str, &TimelineEntry> = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let link_to = |from: &str, relationship: &str| -> Vec<&str> {
        links
            .iter()
            .filter(|link| {
                link.from == from
                    && link.relationship == relationship
                    && link.evidence == TimelineEvidence::SpanLink
            })
            .filter_map(|link| link.to.as_deref())
            .collect()
    };
    let latest_user = |candidates: Vec<&str>| {
        candidates
            .into_iter()
            .filter_map(|id| by_id.get(id).copied())
            .filter(|entry| {
                entry.kind == TimelineEntryKind::Speech && entry.lane == TimelineLane::User
            })
            .filter_map(|entry| Some((entry.end_ms?, entry)))
            .max_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)))
    };
    let mut gaps = Vec::new();
    for agent in speech
        .iter()
        .filter(|entry| entry.lane == TimelineLane::Agent)
    {
        let mut current = agent.id.as_str();
        let mut visited = HashSet::from([current]);
        let mut model = None;
        while let Some(next) = link_to(current, "generated_by").into_iter().next() {
            if !visited.insert(next) {
                break;
            }
            if span_entries
                .get(next)
                .is_some_and(|span| span.kind() == "llm")
            {
                model = Some(next);
                break;
            }
            current = next;
        }
        let measured = model
            .and_then(|model| {
                let (end, user) = latest_user(link_to(model, "input_context"))?;
                Some((end, user, TimelineEvidence::SpanLink, model))
            })
            .or_else(|| {
                let span = span_entries.get(agent.id.as_str())?;
                let turn = turns.get(turn_of(span)?)?;
                let reply_to = turn.source.voice.as_ref()?.reply_to.as_deref()?;
                let users = speech
                    .iter()
                    .filter(|entry| {
                        span_entries
                            .get(entry.id.as_str())
                            .is_some_and(|span| turn_of(span) == Some(reply_to))
                    })
                    .map(|entry| entry.id.as_str())
                    .collect();
                let (end, user) = latest_user(users)?;
                Some((end, user, TimelineEvidence::TurnReplyTo, turn.id.as_str()))
            });
        if let Some((end, user, evidence, via)) = measured {
            gaps.push(TimelineGap {
                from: user.id.clone(),
                to: agent.id.clone(),
                gap_ms: round(agent.start_ms - end),
                evidence,
                via: Some(via.to_string()),
            });
        }
    }

    Some(CallTimeline {
        origin_unix_ms: origin,
        clock,
        entries,
        links,
        overlaps,
        gaps,
    })
}
