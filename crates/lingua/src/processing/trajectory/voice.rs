//! Voice traces in audio trace schema v3: one `Turn` per exchange. A `user_turn` opens a
//! turn, an `agent_turn` joins the one its `turn.reply_to` names, and other spans join their
//! nearest `turn.id`. The last agent speech is the response; the rest is work.

use super::stream::Role;
use super::*;
use crate::universal::{UserContent, UserContentPart};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VoiceRole {
    Request,
    Reply,
    Work { kind: StepKind, position: usize },
    Response { position: usize },
}

pub(super) struct VoiceTurn {
    pub(super) request: usize,
    pub(super) span_count: usize,
    pub(super) response: Option<usize>,
    pub(super) start_time: DateTime<Utc>,
    pub(super) end_time: Option<DateTime<Utc>>,
    pub(super) model: Option<String>,
}

#[derive(Default)]
pub(super) struct VoicePlan {
    /// Traces with a span whose header holds an audio schema version.
    pub(super) roots: HashSet<String>,
    pub(super) sessions: HashSet<usize>,
    pub(super) owners: HashMap<usize, VoiceOwnership>,
    pub(super) turns: Vec<VoiceTurn>,
    pub(super) ignored: HashSet<usize>,
    pub(super) interrupted: HashSet<usize>,
    pub(super) failures: Vec<ImportFailure>,
    pub(super) warnings: HashMap<usize, String>,
}

impl VoicePlan {
    pub(super) fn is_voice(&self, index: usize) -> bool {
        self.sessions.contains(&index) || self.owners.contains_key(&index)
    }
}

/// Where a span goes. `turn` indexes `VoicePlan.turns`, unlike chat's `Ownership.turn`,
/// which is a turn id.
#[derive(Debug, Clone, Copy)]
pub(super) struct VoiceOwnership {
    pub(super) turn: usize,
    pub(super) role: VoiceRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StepKind {
    Chat(Role),
    Heard,
    Spoken,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpanKind {
    UserTurn,
    AgentTurn,
    UserSpeech,
    AgentSpeech,
    Work(StepKind),
}

fn kind(span: &PreparedSpan, role: Role) -> Option<SpanKind> {
    if matches!(role, Role::Conversation | Role::Analysis | Role::Tool) {
        return Some(SpanKind::Work(StepKind::Chat(role)));
    }
    if span.kind() != "task" {
        return None;
    }
    match span.source.name.as_deref()? {
        "user_turn" => Some(SpanKind::UserTurn),
        "agent_turn" => Some(SpanKind::AgentTurn),
        "user_speech" => Some(SpanKind::UserSpeech),
        "agent_speech" => Some(SpanKind::AgentSpeech),
        "stt" => Some(SpanKind::Work(StepKind::Heard)),
        "tts" => Some(SpanKind::Work(StepKind::Spoken)),
        _ => None,
    }
}

pub(super) fn speech_step(span: &PreparedSpan, kind: StepKind) -> WorkStep {
    let (input, output) = if kind == StepKind::Heard {
        (None, joined(span.output.iter().flat_map(user_text)))
    } else {
        (joined(span.output.iter().flat_map(assistant_text)), None)
    };
    WorkStep {
        id: span.id.clone(),
        span_type: span.kind().to_string(),
        name: span.source.name.clone(),
        error: span.source.error.clone(),
        start_time: span.start,
        end_time: span.source.end,
        work: Work::ToolResult(Box::new(ToolResult {
            input,
            output,
            // Empty, not missing: speech steps are complete, so the app shouldn't wait for content.
            content: Some(Vec::new()),
        })),
        sub_agent: None,
    }
}

fn joined<'a>(texts: impl Iterator<Item = &'a str>) -> Option<json::Value> {
    let text = texts.collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(json::Value::String(text))
}

fn user_text(message: &Message) -> Vec<&str> {
    match message {
        Message::User {
            content: UserContent::String(text),
        } => vec![text],
        Message::User {
            content: UserContent::Array(parts),
        } => parts
            .iter()
            .filter_map(|part| match part {
                UserContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn assistant_text(message: &Message) -> Vec<&str> {
    match message {
        Message::Assistant {
            content: AssistantContent::String(text),
            ..
        } => vec![text],
        Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } => parts
            .iter()
            .filter_map(|part| match part {
                AssistantContentPart::Text(part) => Some(part.text.as_str()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn failure(span: &PreparedSpan, message: String) -> ImportFailure {
    ImportFailure {
        root_span_id: span.root_span_id.clone(),
        span_id: span.id.clone(),
        message,
    }
}

pub(super) fn plan(
    spans: &[PreparedSpan],
    by_span_id: &HashMap<(&str, &str), usize>,
    parents: &[Option<usize>],
    owners: &HashMap<usize, Ownership>,
    roles: &[Role],
) -> VoicePlan {
    let mut plan = VoicePlan::default();
    let mut unsupported = HashSet::new();
    for (index, span) in spans.iter().enumerate() {
        let Some(version) = &span.source.voice.schema_version else {
            continue;
        };
        plan.roots.insert(span.root_span_id.clone());
        let version = match version {
            Ok(version) => version,
            Err(error) => {
                unsupported.insert(index);
                plan.failures.push(failure(span, error.clone()));
                continue;
            }
        };
        if version.split('.').next() == Some("3") {
            plan.sessions.insert(index);
        } else {
            unsupported.insert(index);
            plan.failures.push(failure(
                span,
                format!("Unsupported audio schema version {version}"),
            ));
        }
    }
    if plan.roots.is_empty() {
        return plan;
    }
    let session_of: Vec<Option<usize>> = (0..spans.len())
        .map(|index| {
            let mut current = Some(index);
            for _ in 0..spans.len() {
                let at = current?;
                if spans[at].source.voice.schema_version.is_some() {
                    return Some(at);
                }
                current = parents[at];
            }
            None
        })
        .collect();
    for (index, session) in session_of.iter().enumerate() {
        if session.is_some_and(|session| unsupported.contains(&session)) {
            plan.ignored.insert(index);
        }
    }
    let in_scope = |index: usize| {
        let owner = &owners[&index];
        session_of[index].is_some_and(|session| plan.sessions.contains(&session))
            && owner.tool.is_none()
            && owner.compaction.is_none()
    };
    let kinds: Vec<_> = (0..spans.len())
        .map(|index| {
            (in_scope(index) && !owners[&index].skipped && !plan.sessions.contains(&index))
                .then(|| kind(&spans[index], roles[index]))
                .flatten()
        })
        .collect();
    let is_turn =
        |index: usize| matches!(kinds[index], Some(SpanKind::UserTurn | SpanKind::AgentTurn));
    let resolve = |index: usize, id: &str| {
        by_span_id
            .get(&(spans[index].root_span_id.as_str(), id))
            .copied()
    };
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..spans.len() {
        if kinds[index].is_none() {
            continue;
        }
        let turn = match &owners[&index].voice_turn {
            _ if is_turn(index) => Ok(index),
            None => Err("Voice span has no turn".to_string()),
            Some(turn_id) => match resolve(index, turn_id) {
                Some(turn) if is_turn(turn) => Ok(turn),
                Some(_) => Err(format!("turn.id {turn_id} is not a user or agent turn")),
                None => Err(format!("Unresolved turn.id {turn_id}")),
            },
        };
        let turn = match turn {
            Ok(turn) => turn,
            Err(message) => {
                plan.failures.push(failure(&spans[index], message));
                plan.ignored.insert(index);
                continue;
            }
        };
        let request = match (kinds[turn], &spans[turn].source.voice.reply_to) {
            (Some(SpanKind::AgentTurn), Some(reply_to)) => match resolve(turn, reply_to) {
                Some(user_turn) if kinds[user_turn] == Some(SpanKind::UserTurn) => user_turn,
                _ => {
                    plan.warnings.insert(
                        turn,
                        format!("turn.reply_to {reply_to} is not a user turn in this trace"),
                    );
                    turn
                }
            },
            _ => turn,
        };
        groups.entry(request).or_default().push(index);
    }
    for (request, mut group) in groups {
        group.sort_by(|a, b| {
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
        let turn = plan.turns.len();
        let response = group
            .iter()
            .rev()
            .copied()
            .find(|index| kinds[*index] == Some(SpanKind::AgentSpeech));
        let end_time = group
            .iter()
            .map(|index| spans[*index].source.end)
            .collect::<Option<Vec<_>>>()
            .and_then(|ends| ends.into_iter().max());
        let model = group
            .iter()
            .find(|index| {
                kinds[**index] == Some(SpanKind::Work(StepKind::Chat(Role::Conversation)))
            })
            .and_then(|index| spans[*index].source.model.clone());
        // The exchange starts when its opener does; earlier STT streams are only work.
        let start_time = spans[request].start;
        let mut position = 0;
        let mut span_count = 0;
        for &index in &group {
            let role = match kinds[index] {
                Some(SpanKind::UserSpeech) => {
                    plan.ignored.insert(index);
                    continue;
                }
                _ if index == request => VoiceRole::Request,
                Some(SpanKind::UserTurn | SpanKind::AgentTurn) => VoiceRole::Reply,
                _ if Some(index) == response => VoiceRole::Response { position },
                Some(SpanKind::AgentSpeech) => VoiceRole::Work {
                    kind: StepKind::Chat(Role::Conversation),
                    position,
                },
                Some(SpanKind::Work(kind)) => VoiceRole::Work { kind, position },
                None => continue,
            };
            if matches!(role, VoiceRole::Work { .. } | VoiceRole::Response { .. }) {
                position += 1;
            }
            span_count += 1;
            plan.owners.insert(index, VoiceOwnership { turn, role });
        }
        plan.turns.push(VoiceTurn {
            request,
            span_count,
            response,
            start_time,
            end_time,
            model,
        });
    }
    for (index, span) in spans.iter().enumerate() {
        if !in_scope(index) {
            continue;
        }
        if !plan.is_voice(index) {
            plan.ignored.insert(index);
            continue;
        }
        if span.source.voice.interrupted == Some(true) {
            plan.interrupted.insert(index);
        }
        if !span.source.voice.errors.is_empty() {
            plan.failures
                .push(failure(span, span.source.voice.errors.join("; ")));
        }
    }
    plan
}

pub(super) fn response(span: &PreparedSpan) -> Option<AgentResponse> {
    let response = span.response();
    let silent = response.opaque_output.is_empty()
        && matches!(&response.response, Some(AssistantContent::Array(parts)) if parts.is_empty());
    (!silent).then_some(response)
}
