use super::PreparedSpan;
use crate::processing::import::ImportedSpan;
use crate::universal::trajectory::{FrameworkTurn, Link, SpanRef, Speaker, Synthesis, Utterance};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use std::collections::HashMap;

const GENERATED_BY: &str = "generated_by";

/// A `task` span named `user_speech` or `agent_speech`, in any scope.
pub(super) fn speech_speaker(span: &PreparedSpan) -> Option<Speaker> {
    match (span.kind(), span.source.name.as_deref()) {
        ("task", Some("user_speech")) => Some(Speaker::User),
        ("task", Some("agent_speech")) => Some(Speaker::Agent),
        _ => None,
    }
}

fn is_synthesis(span: &PreparedSpan) -> bool {
    span.kind() == "task" && span.source.name.as_deref() == Some("tts")
}

fn user_text(content: &UserContent, texts: &mut Vec<String>) {
    match content {
        UserContent::String(text) => texts.push(text.clone()),
        UserContent::Array(parts) => texts.extend(parts.iter().filter_map(|part| match part {
            UserContentPart::Text(part) => Some(part.text.clone()),
            _ => None,
        })),
    }
}

fn assistant_text(content: &AssistantContent, texts: &mut Vec<String>) {
    match content {
        AssistantContent::String(text) => texts.push(text.clone()),
        AssistantContent::Array(parts) => {
            texts.extend(parts.iter().filter_map(|part| match part {
                AssistantContentPart::Text(part) => Some(part.text.clone()),
                _ => None,
            }))
        }
    }
}

/// Words logged on a loaded speech span: user input for user speech, assistant output for
/// agent speech, text parts joined with newlines.
pub(super) fn speech_text(span: &ImportedSpan, speaker: Speaker) -> Option<String> {
    let mut texts = Vec::new();
    match speaker {
        Speaker::User => {
            for message in &span.input {
                if let Message::User { content } = message {
                    user_text(content, &mut texts);
                }
            }
        }
        Speaker::Agent => {
            for message in &span.output {
                if let Message::Assistant { content, .. } = message {
                    assistant_text(content, &mut texts);
                }
            }
        }
    }
    (!texts.is_empty()).then(|| texts.join("\n"))
}

/// Everything voice-related that span headers determine.
pub(super) struct VoiceHeaders {
    /// Speech span index and its utterance, in `(start, id)` order.
    pub utterances: Vec<(usize, Utterance)>,
    /// Logging span index and its link, in span order then logged order.
    pub links: Vec<(usize, Link)>,
    /// Each `tts` span index to the speech span indices whose `synthesis` it fills.
    pub synthesis_sources: HashMap<usize, Vec<usize>>,
}

pub(super) fn voice_headers(spans: &[PreparedSpan], skipped: &[bool]) -> VoiceHeaders {
    let by_span_id: HashMap<(&str, &str), usize> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| ((span.root_span_id.as_str(), span.span_id.as_str()), index))
        .collect();
    let resolve = |span: &PreparedSpan, target: &str| -> SpanRef {
        SpanRef {
            span_id: target.to_string(),
            step_id: by_span_id
                .get(&(span.root_span_id.as_str(), target))
                .map(|index| spans[*index].id.clone()),
        }
    };
    let mut ordered: Vec<_> = (0..spans.len()).filter(|index| !skipped[*index]).collect();
    ordered.sort_by(|a, b| {
        spans[*a]
            .start
            .cmp(&spans[*b].start)
            .then(spans[*a].id.cmp(&spans[*b].id))
    });
    let mut utterances = Vec::new();
    let mut links = Vec::new();
    let mut synthesis_sources: HashMap<usize, Vec<usize>> = HashMap::new();
    for &index in &ordered {
        let span = &spans[index];
        let voice = span.source.voice.as_ref();
        for link in voice.iter().flat_map(|voice| &voice.links) {
            links.push((
                index,
                Link {
                    from: span.id.clone(),
                    relationship: link.relationship.clone(),
                    to: resolve(span, &link.span_id),
                },
            ));
        }
        let Some(speaker) = speech_speaker(span) else {
            continue;
        };
        let turn = voice
            .and_then(|voice| voice.turn_id.as_deref())
            .map(|turn_id| {
                let turn_span = resolve(span, turn_id);
                let turn_voice = turn_span
                    .step_id
                    .as_ref()
                    .and_then(|_| by_span_id.get(&(span.root_span_id.as_str(), turn_id)))
                    .and_then(|index| spans[*index].source.voice.as_ref());
                FrameworkTurn {
                    span: turn_span,
                    reply_to: turn_voice
                        .and_then(|voice| voice.reply_to.as_deref())
                        .map(|reply_to| resolve(span, reply_to)),
                    incomplete: turn_voice.and_then(|voice| voice.incomplete),
                }
            });
        let synthesis = (speaker == Speaker::Agent)
            .then(|| {
                voice
                    .iter()
                    .flat_map(|voice| &voice.links)
                    .filter(|link| link.relationship == GENERATED_BY)
                    .find_map(|link| {
                        let target = *by_span_id
                            .get(&(span.root_span_id.as_str(), link.span_id.as_str()))?;
                        is_synthesis(&spans[target]).then_some(target)
                    })
            })
            .flatten()
            .map(|target| {
                synthesis_sources.entry(target).or_default().push(index);
                Synthesis {
                    span: SpanRef {
                        span_id: spans[target].span_id.clone(),
                        step_id: Some(spans[target].id.clone()),
                    },
                    text: None,
                }
            });
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
                turn,
                synthesis,
            },
        ));
    }
    VoiceHeaders {
        utterances,
        links,
        synthesis_sources,
    }
}
