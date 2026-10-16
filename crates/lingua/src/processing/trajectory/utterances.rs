use super::PreparedSpan;
use crate::processing::import::SpanVoice;
use crate::universal::trajectory::{
    EvidenceBasis, EvidenceSource, RelationDirection, RelationEvidence, Speaker, SpeechTurn,
    Utterance, UtteranceRelation,
};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use std::collections::{HashMap, HashSet};

pub(super) fn speech_speaker(span: &PreparedSpan) -> Option<Speaker> {
    match (span.kind(), span.source.name.as_deref()) {
        ("task", Some("user_speech")) => Some(Speaker::User),
        ("task", Some("agent_speech")) => Some(Speaker::Agent),
        _ => None,
    }
}

fn turn_role(span: &PreparedSpan) -> Option<Speaker> {
    match (span.kind(), span.source.name.as_deref()) {
        ("task", Some("user_turn")) => Some(Speaker::User),
        ("task", Some("agent_turn")) => Some(Speaker::Agent),
        _ => None,
    }
}

fn voice_of(span: &PreparedSpan) -> Option<&SpanVoice> {
    span.source.voice.as_ref()
}

/// Words logged on a loaded speech span: user input for user speech, assistant output for agent
/// speech.
pub(super) fn speech_text(span: &PreparedSpan, speaker: Speaker) -> Option<String> {
    let mut texts = Vec::new();
    match speaker {
        Speaker::User => {
            for input in span.input.iter().filter(|input| !input.context) {
                if let Message::User { content } = &input.message {
                    match content {
                        UserContent::String(text) => texts.push(text.clone()),
                        UserContent::Array(parts) => {
                            texts.extend(parts.iter().filter_map(|part| match part {
                                UserContentPart::Text(text) => Some(text.text.clone()),
                                _ => None,
                            }))
                        }
                    }
                }
            }
        }
        Speaker::Agent => {
            for message in &span.output {
                if let Message::Assistant { content, .. } = message {
                    match content {
                        AssistantContent::String(text) => texts.push(text.clone()),
                        AssistantContent::Array(parts) => {
                            texts.extend(parts.iter().filter_map(|part| match part {
                                AssistantContentPart::Text(text) => Some(text.text.clone()),
                                _ => None,
                            }))
                        }
                    }
                }
            }
        }
    }
    (!texts.is_empty()).then(|| texts.join("\n"))
}

struct Relations<'a> {
    spans: &'a [PreparedSpan],
    in_turns: &'a dyn Fn(usize) -> bool,
    relations: Vec<UtteranceRelation>,
}

impl Relations<'_> {
    fn add(
        &mut self,
        relationship: String,
        direction: RelationDirection,
        target: Result<usize, &str>,
        via: Option<usize>,
        evidence: RelationEvidence,
    ) {
        let (span_id, span_name, step_id, unresolved) = match target {
            Ok(index) => {
                let span = &self.spans[index];
                (
                    span.id.clone(),
                    span.source.name.clone(),
                    (self.in_turns)(index).then(|| span.id.clone()),
                    None,
                )
            }
            Err(logged) => (logged.to_string(), None, None, Some(true)),
        };
        let via = via.map(|index| self.spans[index].id.clone());
        if let Some(existing) = self.relations.iter_mut().find(|relation| {
            relation.relationship == relationship
                && relation.direction == direction
                && relation.span_id == span_id
                && relation.via == via
        }) {
            if !existing.evidence.contains(&evidence) {
                existing.evidence.push(evidence);
            }
            return;
        }
        self.relations.push(UtteranceRelation {
            relationship,
            direction,
            span_id,
            span_name,
            step_id,
            via,
            unresolved,
            evidence: vec![evidence],
        });
    }
}

fn link_evidence(logged_by: &PreparedSpan, link_type: &str) -> RelationEvidence {
    RelationEvidence {
        basis: EvidenceBasis::Observed,
        source: EvidenceSource::Link,
        logged_by: Some(logged_by.id.clone()),
        link_type: Some(link_type.to_string()),
        message_id: None,
    }
}

/// Logged links whose target span is not in the trace, as `(span index, message)`.
pub(super) fn unresolved_links(spans: &[PreparedSpan]) -> Vec<(usize, String)> {
    let span_ids: HashSet<_> = spans
        .iter()
        .map(|span| (span.root_span_id.as_str(), span.span_id.as_str()))
        .collect();
    let mut unresolved = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        for link in voice_of(span).iter().flat_map(|voice| &voice.links) {
            if !span_ids.contains(&(span.root_span_id.as_str(), link.span_id.as_str())) {
                unresolved.push((
                    index,
                    format!(
                        "Unresolved {} link to span {}: not in this trace",
                        link.relationship, link.span_id
                    ),
                ));
            }
        }
    }
    unresolved
}

/// Utterances placed from span headers, with every relationship observed in `metadata.links` or
/// native message ids. Text fills in when the speech payload loads.
pub(super) fn utterance_headers(
    spans: &[PreparedSpan],
    in_turns: &dyn Fn(usize) -> bool,
) -> Vec<(usize, Utterance)> {
    let by_span_id: HashMap<_, _> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| ((span.root_span_id.as_str(), span.span_id.as_str()), index))
        .collect();
    let resolve = |span: &PreparedSpan, target: &str| {
        by_span_id
            .get(&(span.root_span_id.as_str(), target))
            .copied()
    };
    let mut incoming: HashMap<usize, Vec<(usize, &str)>> = HashMap::new();
    let mut by_response: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    let mut ordered: Vec<_> = (0..spans.len()).collect();
    ordered.sort_by(|a, b| {
        spans[*a]
            .start
            .cmp(&spans[*b].start)
            .then(spans[*a].id.cmp(&spans[*b].id))
    });
    for &index in &ordered {
        let span = &spans[index];
        let Some(voice) = voice_of(span) else {
            continue;
        };
        for link in &voice.links {
            if let Some(target) = resolve(span, &link.span_id) {
                incoming
                    .entry(target)
                    .or_default()
                    .push((index, &link.relationship));
            }
        }
        if let Some(response_id) = &voice.response_id {
            by_response
                .entry((span.root_span_id.as_str(), response_id))
                .or_default()
                .push(index);
        }
    }
    let mut utterances = Vec::new();
    for &index in &ordered {
        let span = &spans[index];
        let Some(speaker) = speech_speaker(span) else {
            continue;
        };
        let voice = voice_of(span);
        let mut relations = Relations {
            spans,
            in_turns,
            relations: Vec::new(),
        };
        for link in voice.iter().flat_map(|voice| &voice.links) {
            let target = resolve(span, &link.span_id);
            relations.add(
                link.relationship.clone(),
                RelationDirection::Outgoing,
                target.ok_or(&link.span_id),
                None,
                link_evidence(span, &link.relationship),
            );
            if link.relationship != "generated_by" {
                continue;
            }
            let mut visited = HashSet::from([index]);
            let mut hop = target;
            while let Some(via) = hop.filter(|via| visited.insert(*via)) {
                let next = voice_of(&spans[via]).and_then(|voice| {
                    voice
                        .links
                        .iter()
                        .find(|link| link.relationship == "generated_by")
                });
                let Some(next) = next else {
                    break;
                };
                hop = resolve(&spans[via], &next.span_id);
                relations.add(
                    next.relationship.clone(),
                    RelationDirection::Outgoing,
                    hop.ok_or(&next.span_id),
                    Some(via),
                    link_evidence(&spans[via], &next.relationship),
                );
            }
        }
        for message_id in voice.iter().flat_map(|voice| &voice.message_ids) {
            for &target in by_response
                .get(&(span.root_span_id.as_str(), message_id.as_str()))
                .into_iter()
                .flatten()
                .filter(|target| **target != index)
            {
                relations.add(
                    match speaker {
                        Speaker::Agent => "generated_by",
                        Speaker::User => "same_message",
                    }
                    .to_string(),
                    RelationDirection::Outgoing,
                    Ok(target),
                    None,
                    RelationEvidence {
                        basis: EvidenceBasis::Observed,
                        source: EvidenceSource::MessageId,
                        logged_by: None,
                        link_type: None,
                        message_id: Some(message_id.clone()),
                    },
                );
            }
        }
        for &(source, relationship) in incoming.get(&index).into_iter().flatten() {
            relations.add(
                match relationship {
                    "input_context" => "in_context_of".to_string(),
                    other => other.to_string(),
                },
                RelationDirection::Incoming,
                Ok(source),
                None,
                link_evidence(&spans[source], relationship),
            );
        }
        utterances.push((
            index,
            Utterance {
                id: span.id.clone(),
                speaker,
                text: None,
                start_time: span.start,
                end_time: span.source.end,
                selections: voice
                    .map(|voice| voice.selections.clone())
                    .unwrap_or_default(),
                turn_id: voice.and_then(|voice| voice.turn_id.clone()),
                relations: relations.relations,
            },
        ));
    }
    utterances
}

fn descends_from(parents: &[Option<usize>], index: usize, ancestor: usize) -> bool {
    let mut visited = HashSet::new();
    let mut parent = parents[index];
    while let Some(current) = parent.filter(|current| visited.insert(*current)) {
        if current == ancestor {
            return true;
        }
        parent = parents[current];
    }
    false
}

/// Turns the framework logged as `user_turn`/`agent_turn` spans. None are invented.
pub(super) fn speech_turns(
    spans: &[PreparedSpan],
    parents: &[Option<usize>],
    utterances: &[(usize, Utterance)],
) -> Vec<(usize, SpeechTurn)> {
    let mut turns = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        let Some(role) = turn_role(span) else {
            continue;
        };
        let voice = voice_of(span);
        let turn_id = voice.and_then(|voice| voice.turn_id.as_deref());
        let utterance_ids = utterances
            .iter()
            .filter(|(speech, utterance)| {
                (turn_id.is_some() && utterance.turn_id.as_deref() == turn_id)
                    || descends_from(parents, *speech, index)
            })
            .map(|(_, utterance)| utterance.id.clone())
            .collect();
        turns.push((
            index,
            SpeechTurn {
                id: turn_id.map_or_else(|| span.id.clone(), str::to_string),
                span_id: span.id.clone(),
                role,
                start_time: span.start,
                end_time: span.source.end,
                utterance_ids,
                reply_to: voice.and_then(|voice| voice.reply_to.clone()),
                incomplete: voice.and_then(|voice| voice.incomplete),
            },
        ));
    }
    turns
}
