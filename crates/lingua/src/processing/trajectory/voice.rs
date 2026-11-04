//! Voice traces in audio trace schema v3: one turn per conversational exchange.
//!
//! Each framework module maps its span names and keys onto the neutral `Kind`s below; the
//! planner works only on those. Headers fix where every span goes before any payload loads.
//! A user turn opens an exchange, an agent turn joins the one named by its `turn.reply_to`
//! (or opens its own), and other spans join through `turn.id` or their nearest turn
//! ancestor. The last agent speech is the response unless it failed; LLM calls, tools, STT,
//! TTS, earlier speech and a failed last speech are work.

mod livekit;

use super::*;
use crate::universal::trajectory::{AudioSource, Channel, RecordingSegment, SegmentState};

/// The framework that traced a voice session, detected once per session from its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framework {
    LiveKit,
}

impl Framework {
    fn detect(session: &PreparedSpan) -> Option<Self> {
        livekit::is_session(session).then_some(Self::LiveKit)
    }

    /// The neutral kind of a task span.
    fn task_kind(self, span: &PreparedSpan) -> Option<Kind> {
        match self {
            Self::LiveKit => livekit::task_kind(span),
        }
    }

    fn interrupted(self, span: &PreparedSpan) -> bool {
        match self {
            Self::LiveKit => livekit::interrupted(span),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkKind {
    Llm,
    Analysis,
    Tool,
    SpeechTask,
    Speech,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    /// The `user_turn`, or an `agent_turn` that answers no user turn. It holds the request.
    Opener,
    /// An `agent_turn` that answers the exchange's `user_turn`.
    Reply,
    Work {
        kind: WorkKind,
        position: usize,
    },
    /// The exchange's last `agent_speech`. It is work at `position` if it failed.
    Response {
        position: usize,
    },
}

pub(super) struct Exchange {
    pub(super) opener: usize,
    /// Spans the voice path loads for this exchange, the opener included. A turn
    /// without speech closes when the last of them drains.
    pub(super) members: usize,
    pub(super) response: Option<usize>,
    pub(super) start_time: DateTime<Utc>,
    pub(super) end_time: Option<DateTime<Utc>>,
    pub(super) model: Option<String>,
}

#[derive(Default)]
pub(super) struct VoicePlan {
    /// Roots whose headers hold an audio schema version.
    pub(super) roots: HashSet<String>,
    pub(super) sessions: HashSet<usize>,
    pub(super) members: HashMap<usize, (usize, Slot)>,
    pub(super) exchanges: Vec<Exchange>,
    /// Spans of voice roots that the voice path doesn't load.
    pub(super) ignored: HashSet<usize>,
    /// Spans whose framework reports the agent's speech was cut off.
    pub(super) interrupted: HashSet<usize>,
    pub(super) failures: Vec<ImportFailure>,
}

impl VoicePlan {
    pub(super) fn is_voice(&self, index: usize) -> bool {
        self.sessions.contains(&index) || self.members.contains_key(&index)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    UserTurn,
    AgentTurn,
    UserSpeech,
    Work(WorkKind),
}

fn kind(span: &PreparedSpan, framework: Framework) -> Option<Kind> {
    match span.kind() {
        "llm" if span.source.analysis => Some(Kind::Work(WorkKind::Analysis)),
        "llm" => Some(Kind::Work(WorkKind::Llm)),
        "tool" => Some(Kind::Work(WorkKind::Tool)),
        "task" => framework.task_kind(span),
        _ => None,
    }
}

pub(super) fn failure(span: &PreparedSpan, message: String) -> ImportFailure {
    ImportFailure {
        root_span_id: span.root_span_id.clone(),
        span_id: span.id.clone(),
        message,
    }
}

/// Plans every span of the roots that hold a v3 voice session. Spans owned by tools keep
/// the chat path; unsupported or invalid schema versions are reported and their roots
/// ignored.
pub(super) fn plan(
    spans: &[PreparedSpan],
    by_span_id: &HashMap<(&str, &str), usize>,
    parents: &[Option<usize>],
    owners: &HashMap<usize, Ownership>,
) -> VoicePlan {
    let mut plan = VoicePlan::default();
    let mut supported = HashMap::new();
    let mut unsupported = HashSet::new();
    for (index, span) in spans.iter().enumerate() {
        let Some(version) = &span.source.voice.schema_version else {
            continue;
        };
        plan.roots.insert(span.root_span_id.clone());
        let version = match version {
            Ok(version) => version,
            Err(error) => {
                unsupported.insert(span.root_span_id.clone());
                plan.failures.push(failure(span, error.clone()));
                continue;
            }
        };
        let message = match (version.split('.').next(), Framework::detect(span)) {
            (Some("3"), Some(framework)) => {
                match supported.insert(span.root_span_id.clone(), framework) {
                    Some(other) if other != framework => {
                        "Voice sessions from different frameworks in one trace".to_string()
                    }
                    _ => {
                        plan.sessions.insert(index);
                        continue;
                    }
                }
            }
            (Some("3"), None) => "Unrecognized voice framework".to_string(),
            _ => format!("Unsupported audio schema version {version}"),
        };
        unsupported.insert(span.root_span_id.clone());
        plan.failures.push(failure(span, message));
    }
    for (index, span) in spans.iter().enumerate() {
        if unsupported.contains(&span.root_span_id) {
            plan.sessions.remove(&index);
            plan.ignored.insert(index);
        }
    }
    let in_scope = |index: usize| {
        let owner = &owners[&index];
        supported.contains_key(&spans[index].root_span_id)
            && !unsupported.contains(&spans[index].root_span_id)
            && owner.tool.is_none()
            && owner.compaction.is_none()
    };
    let kinds: Vec<_> = (0..spans.len())
        .map(|index| {
            (in_scope(index) && !owners[&index].skipped && !plan.sessions.contains(&index))
                .then(|| kind(&spans[index], supported[&spans[index].root_span_id]))
                .flatten()
        })
        .collect();
    let resolve = |index: usize, id: &str| {
        by_span_id
            .get(&(spans[index].root_span_id.as_str(), id))
            .copied()
    };
    // The turn span a span belongs to: itself for turn spans, else `turn.id`, else the
    // nearest ancestor's turn.
    let mut turns: Vec<Option<std::result::Result<usize, String>>> = vec![None; spans.len()];
    for index in 0..spans.len() {
        if kinds[index].is_none() {
            continue;
        }
        let mut current = Some(index);
        let mut found = None;
        while let Some(at) = current {
            if !in_scope(at) {
                break;
            }
            if matches!(kinds[at], Some(Kind::UserTurn | Kind::AgentTurn)) {
                found = Some(Ok(at));
                break;
            }
            if let Some(turn_id) = &spans[at].source.voice.turn_id {
                found = Some(match resolve(at, turn_id) {
                    Some(turn) if matches!(kinds[turn], Some(Kind::UserTurn | Kind::AgentTurn)) => {
                        Ok(turn)
                    }
                    Some(_) => Err(format!("turn.id {turn_id} is not a user or agent turn")),
                    None => Err(format!("Unresolved turn.id {turn_id}")),
                });
                break;
            }
            current = parents[at];
        }
        turns[index] = Some(found.unwrap_or_else(|| Err("Voice span has no turn".to_string())));
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..spans.len() {
        let Some(turn) = turns[index].take() else {
            continue;
        };
        let turn = match turn {
            Ok(turn) => turn,
            Err(message) => {
                plan.failures.push(failure(&spans[index], message));
                plan.ignored.insert(index);
                continue;
            }
        };
        let opener = match (kinds[turn], &spans[turn].source.voice.reply_to) {
            (Some(Kind::AgentTurn), Some(reply_to)) => match resolve(turn, reply_to) {
                Some(user_turn) if kinds[user_turn] == Some(Kind::UserTurn) => user_turn,
                _ => {
                    if index == turn {
                        plan.failures.push(failure(
                            &spans[turn],
                            format!("Unresolved turn.reply_to {reply_to}"),
                        ));
                    }
                    turn
                }
            },
            _ => turn,
        };
        groups.entry(opener).or_default().push(index);
    }
    for (opener, mut group) in groups {
        group.sort_by(|a, b| spans[*a].start_order(&spans[*b]));
        let exchange = plan.exchanges.len();
        let response = group
            .iter()
            .rev()
            .copied()
            .find(|index| kinds[*index] == Some(Kind::Work(WorkKind::Speech)));
        let end_time = group
            .iter()
            .map(|index| spans[*index].source.end)
            .collect::<Option<Vec<_>>>()
            .and_then(|ends| ends.into_iter().max());
        let model = group
            .iter()
            .find(|index| kinds[**index] == Some(Kind::Work(WorkKind::Llm)))
            .and_then(|index| spans[*index].source.model.clone());
        let start_time = spans[group[0]].start;
        let mut position = 0;
        let mut members = 0;
        for &index in &group {
            let slot = match kinds[index] {
                Some(Kind::UserSpeech) => {
                    plan.ignored.insert(index);
                    continue;
                }
                _ if index == opener => Slot::Opener,
                Some(Kind::UserTurn | Kind::AgentTurn) => Slot::Reply,
                _ if Some(index) == response => {
                    position += 1;
                    Slot::Response {
                        position: position - 1,
                    }
                }
                Some(Kind::Work(kind)) => {
                    position += 1;
                    Slot::Work {
                        kind,
                        position: position - 1,
                    }
                }
                None => continue,
            };
            members += 1;
            plan.members.insert(index, (exchange, slot));
        }
        plan.exchanges.push(Exchange {
            opener,
            members,
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
        if supported[&span.root_span_id].interrupted(span) {
            plan.interrupted.insert(index);
        }
        if !span.source.voice.errors.is_empty() {
            plan.failures
                .push(failure(span, span.source.voice.errors.join("; ")));
        }
    }
    plan
}

/// The recording a session span's payload describes, and its problems. Problems are
/// reported without dropping the recording; only a missing origin leaves no recording.
pub(super) fn recording(span: &PreparedSpan) -> (Option<Recording>, Vec<String>) {
    let session = span.voice.session.clone().unwrap_or_default();
    let mut errors = Vec::new();
    let origin = session
        .origin_unix_ms
        .filter(|ms| ms.is_finite())
        .and_then(|ms| DateTime::from_timestamp_micros((ms * 1000.0).round() as i64));
    if origin.is_none() {
        errors.push("Voice session has no valid audio.timeline.origin_unix_ms".to_string());
    }
    let mut segments = Vec::new();
    for (id, entry) in session.segments {
        let mut channels = Vec::new();
        for (index, channel) in entry.channels {
            let Ok(index) = index.parse() else {
                errors.push(format!(
                    "Invalid channel index {index} in audio segment {id}"
                ));
                continue;
            };
            for source in &channel.sources {
                if !session.sources.contains_key(source) {
                    errors.push(format!("Unknown audio source {source} in segment {id}"));
                }
            }
            channels.push(Channel {
                index,
                sources: channel.sources,
            });
        }
        channels.sort_by_key(|channel| channel.index);
        let state = entry.state;
        if state == SegmentState::Ready && entry.attachment.is_none() {
            errors.push(format!("Ready audio segment {id} has no attachment"));
        }
        segments.push(RecordingSegment {
            id,
            state,
            start_offset_ms: entry.start_offset_ms,
            duration_ms: entry.duration_ms,
            attachment_ref: entry.attachment.map(|attachment| attachment.reference),
            mime_type: entry.mime_type,
            channels,
            reason: entry.reason,
        });
    }
    segments.sort_by(|a, b| {
        a.start_offset_ms
            .cmp(&b.start_offset_ms)
            .then(a.id.cmp(&b.id))
    });
    let recording = origin.map(|origin| Recording {
        id: span.id.clone(),
        origin,
        sources: session
            .sources
            .into_iter()
            .map(|(id, source)| AudioSource {
                id,
                label: source.label,
            })
            .collect(),
        segments,
    });
    (recording, errors)
}

/// What a response span played, or `None` when it played nothing.
pub(super) fn response(span: &PreparedSpan) -> Option<AgentResponse> {
    let response = span.response();
    let silent = response.opaque_output.is_empty()
        && matches!(&response.response, Some(AssistantContent::Array(parts)) if parts.is_empty());
    (!silent).then_some(response)
}

pub(super) fn work(span: &PreparedSpan, kind: WorkKind) -> std::result::Result<WorkStep, String> {
    let work = match kind {
        WorkKind::Llm | WorkKind::Speech => Work::AgentResponse(Box::new(span.response())),
        WorkKind::Analysis => Work::LLMAnalysis(Box::new(span.analysis())),
        WorkKind::Tool | WorkKind::SpeechTask => {
            Work::ToolResult(Box::new(span.tool_result.clone().ok_or_else(|| {
                format!("Missing imported tool result for {}", span.id)
            })?))
        }
    };
    Ok(WorkStep {
        id: span.id.clone(),
        span_type: span.kind().to_string(),
        name: span.source.name.clone(),
        error: span.source.error.clone(),
        start_time: span.start,
        end_time: span.source.end,
        work,
        sub_agent: None,
    })
}
