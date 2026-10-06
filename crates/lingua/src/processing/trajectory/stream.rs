use super::*;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct TrajectoryScope {
    pub root_span_id: String,
    pub owner_span_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrajectoryEvent {
    Start {
        scope: TrajectoryScope,
        trajectory: Trajectory,
    },
    Turn {
        scope: TrajectoryScope,
        id: String,
        turn: Box<Turn>,
    },
    Request {
        scope: TrajectoryScope,
        id: String,
        request_id: String,
        request: Vec<Message>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[ts(as = "Option<Vec<OpaqueItem>>", optional)]
        opaque_request: Vec<OpaqueItem>,
    },
    Work {
        scope: TrajectoryScope,
        id: String,
        position: usize,
        step: WorkStep,
    },
    Response {
        scope: TrajectoryScope,
        id: String,
        response_id: Option<String>,
        response: Option<Box<AgentResponse>>,
        end_time: Option<DateTime<Utc>>,
        model: Option<String>,
    },
    Interrupted {
        scope: TrajectoryScope,
        id: String,
    },
    Failure {
        root_span_id: String,
        span_id: String,
        message: String,
    },
    Warning {
        root_span_id: String,
        span_id: String,
        message: String,
    },
    Done,
}

#[derive(Default)]
struct TurnState {
    id: Option<String>,
    previous_id: Option<String>,
    history: Vec<u64>,
    explicit: Option<String>,
    completed: HashMap<String, Vec<CompletedTurn>>,
    request_found: bool,
    request_model: Option<String>,
    candidate_model: Option<String>,
    candidate: Option<(usize, usize)>,
    position: usize,
    end_time: Option<DateTime<Utc>>,
    unfinished: bool,
}

impl TurnState {
    fn returning_turn(
        &mut self,
        explicit: Option<&str>,
        start: DateTime<Utc>,
    ) -> Option<&mut CompletedTurn> {
        let explicit = explicit?;
        if self.explicit.as_deref() == Some(explicit) {
            return None;
        }
        let turns = self.completed.get_mut(explicit)?;
        let position = turns
            .iter()
            .rposition(|turn| turn.start <= start)
            .unwrap_or(0);
        Some(&mut turns[position])
    }
}

struct CompletedTurn {
    id: String,
    start: DateTime<Utc>,
    position: usize,
    response: Option<usize>,
    end_time: Option<DateTime<Utc>>,
    unfinished: bool,
    model: Option<String>,
}

impl CompletedTurn {
    fn response_event(&self, spans: &[PreparedSpan], scope: TrajectoryScope) -> TrajectoryEvent {
        let response = self.response.map(|index| &spans[index]);
        TrajectoryEvent::Response {
            scope,
            id: self.id.clone(),
            response_id: response.map(|span| span.id.clone()),
            response: response.map(|span| Box::new(span.response())),
            end_time: if self.unfinished { None } else { self.end_time },
            model: self.model.clone(),
        }
    }
}

/// Aligns message hashes greedily in occurrence order, preserving omitted history.
/// Input through the last replayed match is context; only unmatched suffix occurrences
/// are fresh. A repeated standalone request must be identified before this alignment.
fn merge_history(history: &[u64], input: &[u64], output: &[u64]) -> (Vec<u64>, Vec<bool>) {
    let mut merged = Vec::with_capacity(history.len() + output.len());
    let mut pending = Vec::new();
    let mut fresh = Vec::with_capacity(input.len());
    let mut cursor = 0;
    let mut matched_input = 0;
    for (index, key) in input.iter().chain(output).enumerate() {
        let found = if index >= input.len() && fresh.iter().any(|fresh| *fresh) {
            None
        } else {
            history[cursor..].iter().position(|old| old == key)
        };
        if index < input.len() {
            fresh.push(found.is_none());
        }
        if let Some(offset) = found {
            if index < input.len() {
                matched_input = index + 1;
            }
            merged.extend_from_slice(&history[cursor..cursor + offset]);
            merged.append(&mut pending);
            merged.push(*key);
            cursor += offset + 1;
        } else {
            pending.push(*key);
        }
    }
    merged.extend_from_slice(&history[cursor..]);
    merged.append(&mut pending);
    fresh[..matched_input].fill(false);
    (merged, fresh)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ScopeKey {
    root_span_id: String,
    tool: Option<usize>,
    compaction: Option<usize>,
}

impl ScopeKey {
    fn new(span: &PreparedSpan, owner: &Ownership) -> Self {
        Self {
            root_span_id: span.root_span_id.clone(),
            tool: owner.tool,
            compaction: owner.compaction,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Ignored,
    Conversation,
    Analysis,
    Tool,
    Task { fallback: bool },
}

impl Role {
    fn can_request(self) -> bool {
        matches!(self, Self::Conversation | Self::Task { .. })
    }
}

fn classify_headers(spans: &[PreparedSpan], owners: &HashMap<usize, Ownership>) -> Vec<Role> {
    let mut roles: Vec<_> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            if owners[&index].skipped {
                return Role::Ignored;
            }
            match span.kind() {
                "llm" if span.source.analysis => Role::Analysis,
                "llm" => Role::Conversation,
                "tool" => Role::Tool,
                "task" if !span.source.analysis => Role::Task { fallback: true },
                _ => Role::Ignored,
            }
        })
        .collect();
    let scopes_with_llms: HashSet<_> = spans
        .iter()
        .enumerate()
        .filter_map(|(index, span)| {
            let owner = &owners[&index];
            (roles[index] == Role::Conversation).then(|| ScopeKey::new(span, owner))
        })
        .collect();
    for (index, role) in roles.iter_mut().enumerate() {
        if let Role::Task { fallback } = role {
            let span = &spans[index];
            let owner = &owners[&index];
            *fallback = !scopes_with_llms.contains(&ScopeKey::new(span, owner));
            if !*fallback && span.source.turn.is_none() {
                *role = Role::Ignored;
            }
        }
    }
    roles
}

fn resolve_task_roles(
    spans: &[PreparedSpan],
    owners: &HashMap<usize, Ownership>,
    parents: &[Option<usize>],
    roles: &mut [Role],
) {
    let tasks: HashSet<_> = roles
        .iter()
        .enumerate()
        .filter_map(|(index, role)| {
            let Role::Task { fallback } = role else {
                return None;
            };
            let span = &spans[index];
            (span.has_request(true) && (*fallback || span.output.is_empty())).then_some(index)
        })
        .collect();
    let mut wrappers = HashSet::new();
    for &index in &tasks {
        let owner = &owners[&index];
        let mut parent = parents[index];
        while let Some(index) = parent {
            let ancestor = &owners[&index];
            if owner.tool != ancestor.tool || owner.compaction != ancestor.compaction {
                break;
            }
            if tasks.contains(&index) {
                wrappers.insert(index);
            }
            parent = parents[index];
        }
    }
    for (index, role) in roles.iter_mut().enumerate() {
        if matches!(role, Role::Task { .. })
            && (!tasks.contains(&index) || wrappers.contains(&index))
        {
            *role = Role::Ignored;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurnRelation {
    Unchanged,
    Changed,
}

struct SpanEvidence<'a> {
    span: &'a PreparedSpan,
    role: Role,
    explicit: Option<&'a str>,
    compaction: bool,
    resumed: bool,
}

struct Observation {
    conversation: ConversationObservation,
    initial_request: bool,
    interrupts: bool,
    history: Vec<u64>,
    request_filter: Option<Vec<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestEvidence {
    Absent,
    Replayed,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConversationObservation {
    Current(RequestEvidence),
    Compaction { has_request: bool },
}

impl ConversationObservation {
    fn has_request(self) -> bool {
        match self {
            Self::Current(request) => request != RequestEvidence::Absent,
            Self::Compaction { has_request } => has_request,
        }
    }
}

fn observe(
    evidence: &SpanEvidence<'_>,
    state: &TurnState,
    previous: Option<&PreparedSpan>,
) -> Observation {
    let span = evidence.span;
    let relation = match (evidence.explicit, state.explicit.as_deref()) {
        (None, _) => TurnRelation::Unchanged,
        (Some(next), Some(current)) if next == current => TurnRelation::Unchanged,
        (Some(_), None) => TurnRelation::Unchanged,
        (Some(_), Some(_)) => TurnRelation::Changed,
    };
    let initial_request = !state.request_found;
    let request_candidate = evidence.role.can_request() && span.has_request(initial_request);
    let repeated_request = request_candidate
        && span.standalone_request
        && evidence.explicit.is_none()
        && state.explicit.is_none()
        && previous.is_some_and(|previous| {
            previous.standalone_request
                && previous.input_keys == span.input_keys
                && previous.source.end.is_some_and(|end| end <= span.start)
        });
    let (history, fresh) = if repeated_request {
        let mut history = state.history.clone();
        history.extend(&span.input_keys);
        history.extend(message_keys(&span.output));
        (history, vec![true; span.input_keys.len()])
    } else {
        merge_history(
            &state.history,
            &span.input_keys,
            &message_keys(&span.output),
        )
    };
    let mut user_count = 0;
    let mut fresh_user_count = 0;
    for input in span.current_input(initial_request) {
        if matches!(input.message, Message::User { .. }) && !input.context {
            user_count += 1;
            if input.history_index.is_some_and(|position| fresh[position]) {
                fresh_user_count += 1;
            }
        }
    }
    let interrupts = span.interruption_offsets.iter().any(|offset| {
        fresh
            .get(*offset)
            .is_some_and(|fresh| fresh_user_count == 0 || *fresh)
    });
    let request = match (request_candidate, fresh_user_count, user_count, relation) {
        (false, _, _, _) => RequestEvidence::Absent,
        (true, 0, 1, TurnRelation::Changed) => RequestEvidence::New,
        (true, 0, _, _) => RequestEvidence::Replayed,
        (true, _, _, _) => RequestEvidence::New,
    };
    let conversation = if evidence.compaction {
        ConversationObservation::Compaction {
            has_request: request != RequestEvidence::Absent,
        }
    } else {
        ConversationObservation::Current(request)
    };
    Observation {
        conversation,
        initial_request,
        interrupts,
        history,
        request_filter: (fresh_user_count > 0).then_some(fresh),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundaryAction {
    NewTurn,
    AttachRequest,
    Continue,
}

fn decide(state: &TurnState, observation: &Observation) -> BoundaryAction {
    match (&state.id, observation.conversation) {
        (None, _) | (_, ConversationObservation::Current(RequestEvidence::New)) => {
            BoundaryAction::NewTurn
        }
        (_, conversation) if !state.request_found && conversation.has_request() => {
            BoundaryAction::AttachRequest
        }
        _ => BoundaryAction::Continue,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseRule {
    IndependentWork,
    OverlappingWork,
    ContinuedConversation,
}

fn decide_response(
    evidence: &SpanEvidence<'_>,
    boundary: BoundaryAction,
    previous: Option<&PreparedSpan>,
) -> ResponseRule {
    let span = evidence.span;
    if !evidence.role.can_request() || (span.input_keys.is_empty() && span.output.is_empty()) {
        return ResponseRule::IndependentWork;
    }
    if boundary == BoundaryAction::NewTurn {
        return ResponseRule::ContinuedConversation;
    }
    if evidence.resumed {
        if let Some(previous) = previous.filter(|previous| previous.start > span.start) {
            let reply = message_keys(&previous.output);
            let replays_reply = !reply.is_empty()
                && span
                    .input_keys
                    .windows(reply.len())
                    .any(|window| window == reply);
            if !replays_reply {
                return ResponseRule::OverlappingWork;
            }
        }
    }
    ResponseRule::ContinuedConversation
}

pub struct TrajectoryStream {
    spans: Vec<PreparedSpan>,
    ready: Vec<bool>,
    by_id: HashMap<String, usize>,
    owners: HashMap<usize, Ownership>,
    parents: Vec<Option<usize>>,
    roles: Vec<Role>,
    scopes: BTreeMap<ScopeKey, Vec<usize>>,
    states: BTreeMap<ScopeKey, TurnState>,
    task_boundaries: HashSet<ScopeKey>,
    resumed: HashSet<usize>,
    order: Vec<usize>,
    cursor: usize,
    exclude_system: bool,
    initialized: bool,
    finished: bool,
    failures: Vec<ImportFailure>,
}

impl TrajectoryStream {
    pub fn new(headers: Vec<ImportedSpan>, exclude_system: bool) -> Result<Self> {
        Self::with_failures(headers, Vec::new(), exclude_system)
    }

    pub fn with_failures(
        headers: Vec<ImportedSpan>,
        failures: Vec<ImportFailure>,
        exclude_system: bool,
    ) -> Result<Self> {
        let mut failures = failures;
        let mut spans = Vec::with_capacity(headers.len());
        for header in headers {
            let context = header.header.clone();
            match PreparedSpan::new(header) {
                Ok(span) => spans.push(span),
                Err(error) => failures.push(ImportFailure::from_context(&context, error)?),
            }
        }
        let mut by_id = HashMap::new();
        for (index, span) in spans.iter().enumerate() {
            if by_id.insert(span.id.clone(), index).is_some() {
                return Err(format!("Duplicate trajectory span {}", span.id));
            }
        }
        let mut order: Vec<_> = (0..spans.len()).collect();
        order.sort_by(|a, b| {
            let left = &spans[*a];
            let right = &spans[*b];
            (left.kind() != "task")
                .cmp(&(right.kind() != "task"))
                .then(left.start.cmp(&right.start))
                .then(left.source.exec_counter.cmp(&right.source.exec_counter))
                .then(left.id.cmp(&right.id))
        });
        let by_span_id: HashMap<_, _> = spans
            .iter()
            .enumerate()
            .map(|(index, span)| ((span.root_span_id.as_str(), span.span_id.as_str()), index))
            .collect();
        let parents: Vec<_> = spans
            .iter()
            .map(|span| {
                span.source.span_parents.first().and_then(|parent| {
                    by_span_id
                        .get(&(span.root_span_id.as_str(), parent.as_str()))
                        .copied()
                })
            })
            .collect();
        let mut owners = HashMap::new();
        for index in 0..spans.len() {
            let mut visiting = HashSet::new();
            if let Err(message) = ownership(index, &spans, &parents, &mut owners, &mut visiting) {
                for index in visiting {
                    owners.insert(
                        index,
                        Ownership {
                            skipped: true,
                            ..Default::default()
                        },
                    );
                    failures.push(ImportFailure {
                        root_span_id: spans[index].root_span_id.clone(),
                        span_id: spans[index].id.clone(),
                        message: message.clone(),
                    });
                }
            }
        }
        let roles = classify_headers(&spans, &owners);
        let ready = roles.iter().map(|role| *role == Role::Ignored).collect();
        Ok(Self {
            ready,
            spans,
            by_id,
            order,
            failures,
            owners,
            parents,
            roles,
            scopes: BTreeMap::new(),
            states: BTreeMap::new(),
            task_boundaries: HashSet::new(),
            resumed: HashSet::new(),
            cursor: 0,
            exclude_system,
            initialized: false,
            finished: false,
        })
    }

    pub fn pending_ids(&self, limit: usize) -> Vec<String> {
        self.order
            .iter()
            .filter(|index| !self.ready[**index])
            .take(limit)
            .map(|index| self.spans[*index].id.clone())
            .collect()
    }

    pub fn push(&mut self, span: ImportedSpan) -> Result<Vec<TrajectoryEvent>> {
        self.push_prepared(PreparedSpan::new(span)?)
    }

    pub fn push_prepared(&mut self, span: PreparedSpan) -> Result<Vec<TrajectoryEvent>> {
        if self.finished {
            return Err("Trajectory stream has finished".to_string());
        }
        let index = *self
            .by_id
            .get(&span.id)
            .ok_or_else(|| format!("Unknown trajectory span {}", span.id))?;
        if self.ready[index] {
            return Err(format!("Trajectory span {} was already loaded", span.id));
        }
        let header = &self.spans[index];
        let SpanContext {
            id: _,
            root_span_id: _,
            span_id: _,
            start: _,
            name: _,
            model: _,
            error: _,
            span_parents,
            kind,
            exec_counter,
            scorer,
            end,
            turn,
            analysis,
            compaction,
        } = &span.source;
        if span.id != header.id
            || span.root_span_id != header.root_span_id
            || span.span_id != header.span_id
            || span.start != header.start
            || span_parents != &header.source.span_parents
            || kind != &header.source.kind
            || exec_counter != &header.source.exec_counter
            || scorer != &header.source.scorer
            || end != &header.source.end
            || turn != &header.source.turn
            || analysis != &header.source.analysis
            || compaction.as_ref().map(|value| &value.id)
                != header.source.compaction.as_ref().map(|value| &value.id)
        {
            return Err(format!("Trajectory structure changed for span {}", span.id));
        }
        self.spans[index] = span;
        self.ready[index] = true;
        self.drain()
    }

    fn scope(&self, key: &ScopeKey) -> TrajectoryScope {
        TrajectoryScope {
            root_span_id: key.root_span_id.clone(),
            owner_span_id: key.tool.map(|index| self.spans[index].id.clone()),
        }
    }

    fn initialize(&mut self, events: &mut Vec<TrajectoryEvent>) -> Result<()> {
        resolve_task_roles(&self.spans, &self.owners, &self.parents, &mut self.roles);
        for (index, span) in self.spans.iter().enumerate() {
            let role = self.roles[index];
            if role == Role::Ignored {
                continue;
            }
            let owner = &self.owners[&index];
            let key = ScopeKey::new(span, owner);
            if matches!(role, Role::Task { .. })
                && span.source.turn.is_some()
                && span.output.is_empty()
            {
                self.task_boundaries.insert(key.clone());
            }
            self.scopes.entry(key).or_default().push(index);
        }
        let mut order_times = HashMap::new();
        for indices in self.scopes.values() {
            let mut llms: Vec<_> = indices
                .iter()
                .copied()
                .filter(|index| self.roles[*index] == Role::Conversation)
                .collect();
            llms.sort_by_key(|index| std::cmp::Reverse(self.spans[*index].start));
            let mut earliest_end: Option<DateTime<Utc>> = None;
            for group in llms.chunk_by(|a, b| self.spans[*a].start == self.spans[*b].start) {
                for &index in group {
                    if let Some(end) = self.spans[index].source.end {
                        if earliest_end.is_some_and(|other| other < end) {
                            order_times.insert(index, end);
                            self.resumed.insert(index);
                        }
                    }
                }
                earliest_end = group
                    .iter()
                    .filter_map(|index| self.spans[*index].source.end)
                    .chain(earliest_end)
                    .min();
            }
        }
        self.order = self.scopes.values().flatten().copied().collect();
        self.order.sort_by(|a, b| {
            order_times
                .get(a)
                .unwrap_or(&self.spans[*a].start)
                .cmp(order_times.get(b).unwrap_or(&self.spans[*b].start))
                .then(
                    self.spans[*a]
                        .source
                        .exec_counter
                        .cmp(&self.spans[*b].source.exec_counter),
                )
                .then(self.spans[*a].id.cmp(&self.spans[*b].id))
        });
        let mut scopes: BTreeMap<TrajectoryScope, Option<String>> = BTreeMap::new();
        for span in &self.spans {
            scopes.insert(
                TrajectoryScope {
                    root_span_id: span.root_span_id.clone(),
                    owner_span_id: None,
                },
                None,
            );
        }
        for failure in &self.failures {
            scopes.insert(
                TrajectoryScope {
                    root_span_id: failure.root_span_id.clone(),
                    owner_span_id: None,
                },
                None,
            );
        }
        for key in self.scopes.keys() {
            scopes.insert(
                self.scope(key),
                key.tool
                    .and_then(|index| self.spans[index].source.name.clone()),
            );
        }
        for (scope, name) in scopes {
            events.push(TrajectoryEvent::Start {
                trajectory: Trajectory {
                    version: Some("1".to_string()),
                    scope: vec![scope.owner_span_id.as_ref().map_or_else(
                        || Scope::Trace {
                            trace_id: scope.root_span_id.clone(),
                        },
                        |id| Scope::Span { id: id.clone() },
                    )],
                    agent: Agent {
                        name,
                        version: None,
                        metadata: Default::default(),
                        instructions: None,
                    },
                    turns: Vec::new(),
                    sections: None,
                    findings: None,
                    metadata: Default::default(),
                },
                scope,
            });
        }
        self.initialized = true;
        Ok(())
    }

    fn drain(&mut self) -> Result<Vec<TrajectoryEvent>> {
        let mut events = Vec::new();
        if !self.initialized {
            if self.spans.iter().enumerate().any(|(index, _)| {
                matches!(self.roles[index], Role::Task { .. }) && !self.ready[index]
            }) {
                return Ok(events);
            }
            self.initialize(&mut events)?;
        }
        while let Some(&index) = self.order.get(self.cursor) {
            if !self.ready[index] {
                break;
            }
            self.advance(index, &mut events)?;
            self.cursor += 1;
        }
        Ok(events)
    }

    fn request(
        &self,
        span: &PreparedSpan,
        fresh: Option<&[bool]>,
        initial_request: bool,
    ) -> Vec<Message> {
        span.current_input(initial_request)
            .filter(|input| {
                if input
                    .history_index
                    .is_some_and(|position| fresh.is_some_and(|fresh| !fresh[position]))
                {
                    return false;
                }
                matches!(
                    input.message,
                    Message::User { .. }
                        | Message::System { .. }
                        | Message::Developer { .. }
                        | Message::AdditionalTools { .. }
                ) && (!self.exclude_system || !matches!(input.message, Message::System { .. }))
            })
            .map(|input| input.message.clone())
            .collect()
    }

    fn work(
        &self,
        scope: &TrajectoryScope,
        id: &str,
        index: usize,
        position: usize,
    ) -> Result<Option<TrajectoryEvent>> {
        let span = &self.spans[index];
        if matches!(
            self.roles[index],
            Role::Ignored | Role::Task { fallback: false }
        ) {
            return Ok(None);
        }
        let work = if self.roles[index] == Role::Tool {
            Work::ToolResult(Box::new(span.tool_result.clone().ok_or_else(|| {
                format!("Missing imported tool result for {}", span.id)
            })?))
        } else if self.roles[index] == Role::Analysis {
            Work::LLMAnalysis(Box::new(LLMAnalysis {
                work: Some(
                    span.input
                        .iter()
                        .map(|input| &input.message)
                        .chain(&span.output)
                        .cloned()
                        .collect(),
                ),
                opaque_input: span.opaque_input.clone(),
                opaque_output: span.opaque_output.clone(),
                model: span.source.model.clone(),
                params: None,
                usage: span.usage(),
            }))
        } else {
            Work::AgentResponse(Box::new(span.response()))
        };
        Ok(Some(TrajectoryEvent::Work {
            scope: scope.clone(),
            id: id.to_string(),
            position,
            step: WorkStep {
                id: span.id.clone(),
                span_type: span.kind().to_string(),
                name: span.source.name.clone(),
                error: span.source.error.clone(),
                start_time: span.start,
                end_time: span.source.end,
                work,
                sub_agent: None,
            },
        }))
    }

    fn can_finish_turn(&self, index: usize) -> bool {
        self.roles[index].can_request() && self.spans[index].has_final_response()
    }

    fn end_turn(
        &self,
        key: &ScopeKey,
        state: &mut TurnState,
        events: &mut Vec<TrajectoryEvent>,
    ) -> Result<()> {
        let Some(id) = &state.id else {
            return Ok(());
        };
        let scope = self.scope(key);
        let response = state
            .candidate
            .filter(|(index, _)| key.compaction.is_none() && self.can_finish_turn(*index))
            .map(|(index, _)| index);
        if response.is_none() {
            if let Some((index, position)) = state.candidate {
                events.extend(self.work(&scope, id, index, position)?);
            }
        }
        let completed = CompletedTurn {
            id: id.clone(),
            start: self.spans[self.by_id[id]].start,
            position: state.position,
            response,
            end_time: state.end_time,
            unfinished: state.unfinished,
            model: state
                .request_model
                .clone()
                .or_else(|| state.candidate_model.clone()),
        };
        events.push(completed.response_event(&self.spans, scope));
        if let Some(explicit) = &state.explicit {
            state
                .completed
                .entry(explicit.clone())
                .or_default()
                .push(completed);
        }
        Ok(())
    }

    fn advance(&mut self, index: usize, events: &mut Vec<TrajectoryEvent>) -> Result<()> {
        let span = &self.spans[index];
        let owner = &self.owners[&index];
        let key = ScopeKey::new(span, owner);
        let scope = self.scope(&key);
        let mut state = self.states.remove(&key).unwrap_or_default();
        if self.resumed.contains(&index) {
            if let Some(target) = state.returning_turn(owner.turn.as_deref(), span.start) {
                events.extend(self.work(&scope, &target.id, index, target.position)?);
                target.position += 1;
                target.end_time = target.end_time.max(span.source.end);
                target.unfinished |= span.source.end.is_none();
                events.push(target.response_event(&self.spans, scope));
                if !span.input_keys.is_empty() {
                    state.history = merge_history(
                        &state.history,
                        &span.input_keys,
                        &message_keys(&span.output),
                    )
                    .0;
                }
                self.states.insert(key, state);
                return Ok(());
            }
        }
        let explicit = if self.task_boundaries.contains(&key) {
            if matches!(self.roles[index], Role::Task { .. }) {
                span.source.turn.as_deref()
            } else {
                None
            }
        } else {
            owner.turn.as_deref()
        };
        let evidence = SpanEvidence {
            span,
            role: self.roles[index],
            explicit,
            compaction: key.compaction.is_some(),
            resumed: self.resumed.contains(&index),
        };
        let previous = state.candidate.map(|(index, _)| &self.spans[index]);
        let observation = observe(&evidence, &state, previous);
        let boundary = decide(&state, &observation);
        let response_rule = decide_response(&evidence, boundary, previous);
        let new_turn = boundary == BoundaryAction::NewTurn;
        let initial_request = observation.initial_request;
        let request_filter = observation.request_filter.as_deref();
        if new_turn {
            self.end_turn(&key, &mut state, events)?;
            state.previous_id = state.id.take();
            if key.compaction.is_none() && observation.interrupts {
                if let Some(id) = &state.previous_id {
                    events.push(TrajectoryEvent::Interrupted {
                        scope: scope.clone(),
                        id: id.clone(),
                    });
                }
            }
            state.id = Some(span.id.clone());
            state.request_found = observation.conversation.has_request();
            state.request_model = span.source.model.clone();
            state.candidate_model = None;
            state.candidate = None;
            state.position = 0;
            state.end_time = key
                .compaction
                .and_then(|index| self.spans[index].source.end);
            state.unfinished = false;
            events.push(TrajectoryEvent::Turn {
                scope: scope.clone(),
                id: span.id.clone(),
                turn: Box::new(Turn {
                    request_id: span.id.clone(),
                    request: Some(if self.roles[index].can_request() {
                        self.request(span, request_filter, initial_request)
                    } else {
                        Vec::new()
                    }),
                    opaque_request: if self.roles[index].can_request() {
                        span.opaque_input.clone()
                    } else {
                        Vec::new()
                    },
                    response_id: None,
                    response: None,
                    work: Vec::new(),
                    model: span.source.model.clone(),
                    params: None,
                    start_time: key
                        .compaction
                        .map_or(span.start, |index| self.spans[index].start),
                    end_time: None,
                    interrupted: None,
                    compaction: key
                        .compaction
                        .and_then(|index| self.spans[index].source.compaction.clone()),
                }),
            });
        } else if boundary == BoundaryAction::AttachRequest {
            if key.compaction.is_none() && observation.interrupts {
                if let Some(id) = &state.previous_id {
                    events.push(TrajectoryEvent::Interrupted {
                        scope: scope.clone(),
                        id: id.clone(),
                    });
                }
            }
            state.request_found = true;
            state.request_model = span.source.model.clone();
            events.push(TrajectoryEvent::Request {
                scope: scope.clone(),
                id: state.id.clone().unwrap(),
                request_id: span.id.clone(),
                request: self.request(span, request_filter, initial_request),
                opaque_request: span.opaque_input.clone(),
            });
        }
        let id = state.id.as_deref().unwrap();
        if response_rule == ResponseRule::ContinuedConversation {
            if let Some((previous, position)) = state.candidate.take() {
                events.extend(self.work(&scope, id, previous, position)?);
            }
            if state.candidate_model.is_none() {
                state.candidate_model = span.source.model.clone();
            }
            if key.compaction.is_none() && self.can_finish_turn(index) {
                state.candidate = Some((index, state.position));
            } else {
                events.extend(self.work(&scope, id, index, state.position)?);
            }
        } else {
            events.extend(self.work(&scope, id, index, state.position)?);
        }
        state.position += 1;
        state.unfinished =
            !self.can_finish_turn(index) && (state.unfinished || span.source.end.is_none());
        state.end_time = state.end_time.max(span.source.end);
        if self.roles[index].can_request() && !span.input_keys.is_empty() {
            state.history = observation.history;
        }
        if let Some(explicit) = explicit.filter(|_| new_turn || state.explicit.is_none()) {
            state.explicit = Some(explicit.to_string());
        }
        self.states.insert(key, state);
        Ok(())
    }

    pub fn finish(&mut self) -> Result<Vec<TrajectoryEvent>> {
        if self.finished {
            return Err("Trajectory stream has finished".to_string());
        }
        if self.order.iter().any(|index| !self.ready[*index]) {
            return Err("Trajectory stream has unresolved spans".to_string());
        }
        let mut events = self.drain()?;
        for (key, mut state) in std::mem::take(&mut self.states) {
            self.end_turn(&key, &mut state, &mut events)?;
        }
        let mut failures = std::mem::take(&mut self.failures);
        failures.extend(
            self.spans
                .iter()
                .enumerate()
                .filter(|(index, _)| !self.owners[index].skipped)
                .filter_map(|(_, span)| span.failure.clone()),
        );
        failures.sort_by(|a, b| a.span_id.cmp(&b.span_id).then(a.message.cmp(&b.message)));
        failures.dedup_by(|a, b| {
            a.root_span_id == b.root_span_id && a.span_id == b.span_id && a.message == b.message
        });
        events.extend(
            failures
                .into_iter()
                .map(|failure| TrajectoryEvent::Failure {
                    root_span_id: failure.root_span_id,
                    span_id: failure.span_id,
                    message: failure.message,
                }),
        );
        let mut warnings: Vec<_> = self
            .spans
            .iter()
            .enumerate()
            .filter(|(index, span)| !self.owners[index].skipped && !span.warnings.is_empty())
            .map(|(_, span)| span)
            .collect();
        warnings.sort_by(|a, b| a.id.cmp(&b.id));
        for span in warnings {
            events.push(TrajectoryEvent::Warning {
                root_span_id: span.root_span_id.clone(),
                span_id: span.id.clone(),
                message: span.warnings.join("; "),
            });
        }
        events.push(TrajectoryEvent::Done);
        self.finished = true;
        Ok(events)
    }
}

struct CollectedTurn {
    turn: Turn,
    work: BTreeMap<usize, WorkStep>,
    order: usize,
}
type CollectedTrajectory = (Trajectory, BTreeMap<String, CollectedTurn>);

#[derive(Default)]
pub struct TrajectoryCollector {
    scopes: BTreeMap<TrajectoryScope, CollectedTrajectory>,
    complete: bool,
}

impl TrajectoryCollector {
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn push(&mut self, event: TrajectoryEvent) -> Result<()> {
        if self.complete {
            return Err("Trajectory collection has finished".to_string());
        }
        match event {
            TrajectoryEvent::Start { scope, trajectory } => {
                if self.scopes.contains_key(&scope) {
                    return Err("Duplicate trajectory scope".to_string());
                }
                self.scopes.insert(scope, (trajectory, BTreeMap::new()));
            }
            TrajectoryEvent::Turn { scope, id, turn } => {
                let (_, turns) = self
                    .scopes
                    .get_mut(&scope)
                    .ok_or("Unknown trajectory scope")?;
                if turns.contains_key(&id) {
                    return Err("Duplicate trajectory turn".to_string());
                }
                let order = turns.len();
                turns.insert(
                    id,
                    CollectedTurn {
                        turn: *turn,
                        work: BTreeMap::new(),
                        order,
                    },
                );
            }
            TrajectoryEvent::Request {
                scope,
                id,
                request_id,
                request,
                opaque_request,
            } => {
                let turn = &mut self.turn(&scope, &id)?.turn;
                turn.request_id = request_id;
                turn.request = Some(request);
                turn.opaque_request = opaque_request;
            }
            TrajectoryEvent::Work {
                scope,
                id,
                position,
                step,
            } => {
                self.turn(&scope, &id)?.work.insert(position, step);
            }
            TrajectoryEvent::Response {
                scope,
                id,
                response_id,
                response,
                end_time,
                model,
            } => {
                let turn = &mut self.turn(&scope, &id)?.turn;
                turn.response_id = response_id;
                turn.response = response.map(|response| *response);
                turn.end_time = end_time;
                turn.model = model;
            }
            TrajectoryEvent::Interrupted { scope, id } => {
                self.turn(&scope, &id)?.turn.interrupted = Some(true)
            }
            TrajectoryEvent::Failure {
                root_span_id,
                span_id,
                message,
            } => self.import_diagnostic("import_failures", root_span_id, span_id, message)?,
            TrajectoryEvent::Warning {
                root_span_id,
                span_id,
                message,
            } => self.import_diagnostic("import_warnings", root_span_id, span_id, message)?,
            TrajectoryEvent::Done => self.complete = true,
        }
        Ok(())
    }

    fn import_diagnostic(
        &mut self,
        field: &str,
        root_span_id: String,
        span_id: String,
        message: String,
    ) -> Result<()> {
        let (trajectory, _) = self
            .scopes
            .get_mut(&TrajectoryScope {
                root_span_id,
                owner_span_id: None,
            })
            .ok_or("Unknown trajectory scope")?;
        let diagnostics = trajectory
            .metadata
            .entry(field)
            .or_insert_with(|| json::json!([]))
            .as_array_mut()
            .ok_or("Invalid trajectory diagnostic collection")?;
        diagnostics.push(json::json!({ "span_id": span_id, "message": message }));
        Ok(())
    }

    fn turn(&mut self, scope: &TrajectoryScope, id: &str) -> Result<&mut CollectedTurn> {
        self.scopes
            .get_mut(scope)
            .and_then(|(_, turns)| turns.get_mut(id))
            .ok_or_else(|| "Unknown trajectory turn".to_string())
    }

    fn trajectory(
        &self,
        scope: &TrajectoryScope,
        visiting: &mut HashSet<TrajectoryScope>,
    ) -> Result<Trajectory> {
        if !visiting.insert(scope.clone()) {
            return Err("Cycle in trajectory scopes".to_string());
        }
        let (header, turns) = self.scopes.get(scope).ok_or("Unknown trajectory scope")?;
        let mut trajectory = header.clone();
        let mut turns: Vec<_> = turns.values().collect();
        turns.sort_by_key(|collected| (collected.turn.start_time, collected.order));
        for collected in turns {
            let mut turn = collected.turn.clone();
            turn.work = collected
                .work
                .values()
                .map(|step| {
                    let mut step = step.clone();
                    let child = TrajectoryScope {
                        root_span_id: scope.root_span_id.clone(),
                        owner_span_id: Some(step.id.clone()),
                    };
                    if self.scopes.contains_key(&child) {
                        step.sub_agent = Some(Box::new(self.trajectory(&child, visiting)?));
                    }
                    Ok(step)
                })
                .collect::<Result<Vec<_>>>()?;
            trajectory.turns.push(turn);
        }
        visiting.remove(scope);
        Ok(trajectory)
    }

    pub fn snapshot(&self) -> Result<Vec<Trajectory>> {
        self.scopes
            .keys()
            .filter(|scope| scope.owner_span_id.is_none())
            .map(|scope| self.trajectory(scope, &mut HashSet::new()))
            .collect()
    }
}
