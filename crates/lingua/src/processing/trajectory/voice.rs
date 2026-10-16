use super::stream::{TrajectoryEvent, TrajectoryScope};
use super::PreparedSpan;
use crate::processing::import::{ImportedSpan, SpanVoice};
use crate::universal::trajectory::{
    ContextRef, InferenceBasis, MessageJoin, ObservedSource, VoiceContext, VoiceEvidence,
    VoiceMessage, VoiceSpeaker, VoiceSynthesis,
};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, UserContent, UserContentPart,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};

const INPUT_CONTEXT: &str = "input_context";
const GENERATED_BY: &str = "generated_by";

/// Header facts of one span in a voice trace, frozen when the stream starts.
struct Node {
    id: String,
    span_id: String,
    kind: String,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    voice: SpanVoice,
}

/// Payload text that voice joins read.
#[derive(Default)]
pub(super) struct VoiceText {
    /// Speech text for speech spans, assistant output text for model calls, and TTS input for
    /// synthesis spans.
    text: Option<String>,
    /// User and assistant text inputs of a model call, in order.
    inputs: Vec<(VoiceSpeaker, String)>,
}

struct Speech {
    node: usize,
    speaker: VoiceSpeaker,
    id: String,
}

#[derive(Default)]
struct RootVoice {
    nodes: Vec<Node>,
    by_span_id: HashMap<String, usize>,
    speech: Vec<Speech>,
    calls: Vec<usize>,
    texts: HashMap<usize, VoiceText>,
    emitted_messages: Vec<Option<VoiceMessage>>,
    emitted_contexts: Vec<Option<VoiceContext>>,
}

/// Joins speech spans to the model calls that generated or received them, per trace.
#[derive(Default)]
pub(super) struct VoiceJoins {
    roots: BTreeMap<String, RootVoice>,
    /// Stream span index to (root, node) for spans whose payload text feeds voice joins.
    sources: HashMap<usize, (String, usize)>,
}

fn speaker(span: &PreparedSpan) -> Option<VoiceSpeaker> {
    if span.kind() != "task" {
        return None;
    }
    match span.source.name.as_deref() {
        Some("user_speech") => Some(VoiceSpeaker::User),
        Some("agent_speech") => Some(VoiceSpeaker::Agent),
        _ => None,
    }
}

impl VoiceJoins {
    pub(super) fn new(spans: &[PreparedSpan], skipped: impl Fn(usize) -> bool) -> Self {
        let mut joins = Self::default();
        let voice_roots: Vec<_> = spans
            .iter()
            .enumerate()
            .filter(|(index, span)| !skipped(*index) && speaker(span).is_some())
            .map(|(_, span)| span.root_span_id.clone())
            .collect();
        for root in voice_roots {
            joins.roots.entry(root).or_default();
        }
        let mut members: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, span) in spans.iter().enumerate() {
            if !skipped(index) && joins.roots.contains_key(&span.root_span_id) {
                members.entry(&span.root_span_id).or_default().push(index);
            }
        }
        for (root_id, mut indices) in members {
            let mut stream_index = Vec::with_capacity(indices.len());
            indices.sort_by(|a, b| {
                spans[*a]
                    .start
                    .cmp(&spans[*b].start)
                    .then(
                        spans[*a]
                            .source
                            .exec_counter
                            .cmp(&spans[*b].source.exec_counter),
                    )
                    .then(spans[*a].id.cmp(&spans[*b].id))
            });
            let Some(root) = joins.roots.get_mut(root_id) else {
                continue;
            };
            for index in indices {
                let span = &spans[index];
                let node = root.nodes.len();
                root.by_span_id.insert(span.span_id.clone(), node);
                let voice = span.source.voice.clone().unwrap_or_default();
                if let Some(speaker) = speaker(span) {
                    root.speech.push(Speech {
                        node,
                        speaker,
                        id: voice
                            .message_ids
                            .first()
                            .cloned()
                            .unwrap_or_else(|| span.id.clone()),
                    });
                    joins.sources.insert(index, (root_id.to_string(), node));
                } else if span.kind() == "llm" {
                    if !span.source.analysis {
                        root.calls.push(node);
                    }
                    joins.sources.insert(index, (root_id.to_string(), node));
                }
                stream_index.push(index);
                root.nodes.push(Node {
                    id: span.id.clone(),
                    span_id: span.span_id.clone(),
                    kind: span.kind().to_string(),
                    start: span.start,
                    end: span.source.end,
                    voice,
                });
            }
            let synthesis: Vec<_> = root
                .speech
                .iter()
                .filter_map(|speech| root.synthesis(speech))
                .collect();
            for node in synthesis {
                joins
                    .sources
                    .insert(stream_index[node], (root_id.to_string(), node));
            }
            let mut counts: HashMap<String, usize> = HashMap::new();
            for speech in &root.speech {
                *counts.entry(speech.id.clone()).or_default() += 1;
            }
            for speech in &mut root.speech {
                if counts[&speech.id] > 1 {
                    speech.id = root.nodes[speech.node].id.clone();
                }
            }
            root.emitted_messages = vec![None; root.speech.len()];
            root.emitted_contexts = vec![None; root.calls.len()];
        }
        joins
    }

    /// Stream indices of spans whose payload text voice joins read.
    pub(super) fn sources(&self) -> impl Iterator<Item = usize> + '_ {
        self.sources.keys().copied()
    }

    pub(super) fn wants(&self, index: usize) -> bool {
        self.sources.contains_key(&index)
    }

    /// Header-only joins of every voice trace.
    pub(super) fn initial_events(&mut self) -> Vec<TrajectoryEvent> {
        let roots: Vec<_> = self.roots.keys().cloned().collect();
        roots
            .into_iter()
            .flat_map(|root| self.events(&root))
            .collect()
    }

    pub(super) fn record(&mut self, index: usize, text: VoiceText) -> Vec<TrajectoryEvent> {
        let Some((root, node)) = self.sources.get(&index).cloned() else {
            return Vec::new();
        };
        if let Some(voice) = self.roots.get_mut(&root) {
            voice.texts.insert(node, text);
        }
        self.events(&root)
    }

    fn events(&mut self, root_id: &str) -> Vec<TrajectoryEvent> {
        let Some(root) = self.roots.get_mut(root_id) else {
            return Vec::new();
        };
        let (messages, contexts) = root.join();
        let scope = TrajectoryScope {
            root_span_id: root_id.to_string(),
            owner_span_id: None,
        };
        let mut events = Vec::new();
        for (position, message) in messages.into_iter().enumerate() {
            if root.emitted_messages[position].as_ref() != Some(&message) {
                root.emitted_messages[position] = Some(message.clone());
                events.push(TrajectoryEvent::VoiceMessage {
                    scope: scope.clone(),
                    position,
                    message: Box::new(message),
                });
            }
        }
        for (position, context) in contexts.into_iter().enumerate() {
            if root.emitted_contexts[position].as_ref() != Some(&context) {
                root.emitted_contexts[position] = Some(context.clone());
                events.push(TrajectoryEvent::VoiceContext {
                    scope: scope.clone(),
                    position,
                    context,
                });
            }
        }
        events
    }
}

fn add_join(joins: &mut Vec<MessageJoin>, join: MessageJoin) {
    let existing = joins.iter_mut().find(|existing| {
        existing.relationship == join.relationship
            && existing.span_id == join.span_id
            && existing.via_span_id == join.via_span_id
    });
    match existing {
        Some(existing) => {
            for evidence in join.evidence {
                if !existing.evidence.contains(&evidence) {
                    existing.evidence.push(evidence);
                }
            }
        }
        None => joins.push(join),
    }
}

const LINK: VoiceEvidence = VoiceEvidence::Observed {
    source: ObservedSource::Link,
};

impl RootVoice {
    fn text(&self, node: usize) -> Option<&str> {
        self.texts.get(&node).and_then(|text| text.text.as_deref())
    }

    /// The non-model span an agent message's first `generated_by` link targets, like a `tts` span.
    fn synthesis(&self, speech: &Speech) -> Option<usize> {
        if speech.speaker != VoiceSpeaker::Agent {
            return None;
        }
        self.nodes[speech.node]
            .voice
            .links
            .iter()
            .filter(|link| link.relationship == GENERATED_BY)
            .find_map(|link| self.by_span_id.get(&link.span_id).copied())
            .filter(|target| self.nodes[*target].kind != "llm")
    }

    fn join(&self) -> (Vec<VoiceMessage>, Vec<VoiceContext>) {
        let mut messages: Vec<_> = self
            .speech
            .iter()
            .map(|speech| self.message(speech))
            .collect();
        let by_node: HashMap<usize, usize> = self
            .speech
            .iter()
            .enumerate()
            .map(|(position, speech)| (speech.node, position))
            .collect();
        let mut contexts = Vec::with_capacity(self.calls.len());
        for &call in &self.calls {
            let node = &self.nodes[call];
            let mut received: BTreeMap<usize, VoiceEvidence> = BTreeMap::new();
            let mut unresolved_span_ids = Vec::new();
            for link in &node.voice.links {
                if link.relationship != INPUT_CONTEXT {
                    continue;
                }
                match self
                    .by_span_id
                    .get(&link.span_id)
                    .and_then(|target| by_node.get(target))
                {
                    Some(&position) => {
                        received.insert(position, LINK);
                    }
                    None if !unresolved_span_ids.contains(&link.span_id) => {
                        unresolved_span_ids.push(link.span_id.clone());
                    }
                    None => {}
                }
            }
            let unmatched_input_count = self.texts.get(&call).map(|text| {
                let mut unmatched = 0;
                for (speaker, input) in &text.inputs {
                    let candidates: Vec<_> = messages
                        .iter()
                        .enumerate()
                        .filter(|(_, message)| {
                            message.speaker == *speaker
                                && (message.spoken_text.as_ref() == Some(input)
                                    || message.generated_text.as_ref() == Some(input))
                        })
                        .map(|(position, _)| position)
                        .collect();
                    if candidates.iter().any(|position| {
                        received
                            .get(position)
                            .is_some_and(|evidence| *evidence == LINK)
                    }) {
                        continue;
                    }
                    match candidates.as_slice() {
                        [position] => {
                            received
                                .entry(*position)
                                .or_insert(VoiceEvidence::Inferred {
                                    basis: InferenceBasis::TextMatch,
                                });
                        }
                        _ => unmatched += 1,
                    }
                }
                unmatched
            });
            for (&position, &evidence) in &received {
                messages[position].context_of.push(ContextRef {
                    step_id: node.id.clone(),
                    evidence,
                });
            }
            contexts.push(VoiceContext {
                step_id: node.id.clone(),
                message_ids: received
                    .keys()
                    .map(|position| messages[*position].id.clone())
                    .collect(),
                unmatched_input_count,
                unresolved_span_ids,
            });
        }
        (messages, contexts)
    }

    fn message(&self, speech: &Speech) -> VoiceMessage {
        let node = &self.nodes[speech.node];
        let mut joins = Vec::new();
        for link in &node.voice.links {
            let target = self.by_span_id.get(&link.span_id).copied();
            add_join(
                &mut joins,
                MessageJoin {
                    relationship: link.relationship.clone(),
                    span_id: link.span_id.clone(),
                    step_id: target.map(|target| self.nodes[target].id.clone()),
                    via_span_id: None,
                    evidence: vec![LINK],
                },
            );
            let Some(target) = target.filter(|target| {
                link.relationship == GENERATED_BY && self.nodes[*target].kind != "llm"
            }) else {
                continue;
            };
            for hop in &self.nodes[target].voice.links {
                if hop.relationship != GENERATED_BY {
                    continue;
                }
                add_join(
                    &mut joins,
                    MessageJoin {
                        relationship: GENERATED_BY.to_string(),
                        span_id: hop.span_id.clone(),
                        step_id: self
                            .by_span_id
                            .get(&hop.span_id)
                            .map(|hop| self.nodes[*hop].id.clone()),
                        via_span_id: Some(link.span_id.clone()),
                        evidence: vec![LINK],
                    },
                );
            }
        }
        if speech.speaker == VoiceSpeaker::Agent {
            for &call in &self.calls {
                let call_node = &self.nodes[call];
                if call_node
                    .voice
                    .response_id
                    .as_ref()
                    .is_some_and(|response_id| node.voice.message_ids.contains(response_id))
                {
                    add_join(
                        &mut joins,
                        MessageJoin {
                            relationship: GENERATED_BY.to_string(),
                            span_id: call_node.span_id.clone(),
                            step_id: Some(call_node.id.clone()),
                            via_span_id: None,
                            evidence: vec![VoiceEvidence::Observed {
                                source: ObservedSource::MessageId,
                            }],
                        },
                    );
                }
            }
        }
        let generated = joins
            .iter()
            .filter(|join| join.relationship == GENERATED_BY)
            .find_map(|join| {
                let target = *self.by_span_id.get(&join.span_id)?;
                (self.nodes[target].kind == "llm").then_some(target)
            });
        let spoken_text = self.text(speech.node).map(str::to_string);
        let generated_text = generated
            .and_then(|target| self.text(target))
            .map(str::to_string);
        let synthesis = self.synthesis(speech).map(|target| VoiceSynthesis {
            span_id: self.nodes[target].span_id.clone(),
            step_id: self.nodes[target].id.clone(),
            text: self.text(target).map(str::to_string),
        });
        VoiceMessage {
            id: speech.id.clone(),
            speaker: speech.speaker,
            speech_id: node.id.clone(),
            native_ids: node.voice.message_ids.clone(),
            spoken_text,
            generated_text,
            generated_step_id: generated.map(|target| self.nodes[target].id.clone()),
            synthesis,
            selections: node.voice.selections.clone(),
            start_time: node.start,
            end_time: node.end,
            turn_id: node.voice.turn_id.clone(),
            reply_to: node.voice.reply_to.clone().or_else(|| {
                let turn_id = node.voice.turn_id.as_ref()?;
                self.nodes
                    .iter()
                    .filter(|other| other.voice.turn_id.as_ref() == Some(turn_id))
                    .find_map(|other| other.voice.reply_to.clone())
            }),
            joins,
            context_of: Vec::new(),
        }
    }
}

fn user_text(content: &UserContent) -> Option<String> {
    match content {
        UserContent::String(text) => Some(text.clone()),
        UserContent::Array(parts) => {
            let texts: Vec<_> = parts
                .iter()
                .filter_map(|part| match part {
                    UserContentPart::Text(part) => Some(part.text.as_str()),
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
                    AssistantContentPart::Text(part) => Some(part.text.as_str()),
                    _ => None,
                })
                .collect();
            (!texts.is_empty()).then(|| texts.concat())
        }
    }
}

fn message_text(message: &Message) -> Option<(VoiceSpeaker, String)> {
    match message {
        Message::User { content } => Some((VoiceSpeaker::User, user_text(content)?)),
        Message::Assistant { content, .. } => Some((VoiceSpeaker::Agent, assistant_text(content)?)),
        _ => None,
    }
}

fn joined(texts: impl Iterator<Item = String>) -> Option<String> {
    let texts: Vec<_> = texts.collect();
    (!texts.is_empty()).then(|| texts.concat())
}

impl VoiceText {
    pub(super) fn read(span: &ImportedSpan) -> Self {
        let speaker = match (span.header.kind.as_str(), span.header.name.as_deref()) {
            ("task", Some("user_speech")) => Some(VoiceSpeaker::User),
            ("task", Some("agent_speech")) => Some(VoiceSpeaker::Agent),
            (kind, _) if kind != "llm" => {
                return Self {
                    text: span.synthesis_text.clone(),
                    inputs: Vec::new(),
                };
            }
            _ => None,
        };
        let texts = |messages: &[Message], wanted: VoiceSpeaker| {
            joined(
                messages
                    .iter()
                    .filter_map(message_text)
                    .filter(|(speaker, _)| *speaker == wanted)
                    .map(|(_, text)| text),
            )
        };
        match speaker {
            Some(VoiceSpeaker::User) => Self {
                text: texts(&span.input, VoiceSpeaker::User),
                inputs: Vec::new(),
            },
            Some(VoiceSpeaker::Agent) => Self {
                text: texts(&span.output, VoiceSpeaker::Agent),
                inputs: Vec::new(),
            },
            None => Self {
                text: texts(&span.output, VoiceSpeaker::Agent),
                inputs: span.input.iter().filter_map(message_text).collect(),
            },
        }
    }
}
