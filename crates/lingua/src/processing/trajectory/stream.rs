use super::*;

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
        turn: Turn,
    },
    Request {
        scope: TrajectoryScope,
        id: String,
        request_id: String,
        request: Vec<Message>,
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
        response: Option<AgentResponse>,
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
    Done,
}

#[derive(Default)]
struct TurnState {
    id: Option<String>,
    previous_id: Option<String>,
    history: Vec<u64>,
    explicit: Option<String>,
    seen_explicit: HashSet<String>,
    request_found: bool,
    request_model: Option<String>,
    candidate_model: Option<String>,
    candidate: Option<(usize, usize)>,
    position: usize,
    end_time: Option<DateTime<Utc>>,
    unfinished: bool,
}

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

type ScopeKey = (String, Option<usize>, Option<usize>);

pub struct TrajectoryStream {
    spans: Vec<PreparedSpan>,
    ready: Vec<bool>,
    by_id: HashMap<String, usize>,
    owners: HashMap<usize, Ownership>,
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
    pub fn new(headers: Vec<SpanContext>, exclude_system: bool) -> Result<Self> {
        Self::with_failures(headers, Vec::new(), exclude_system)
    }

    pub fn with_failures(
        headers: Vec<SpanContext>,
        failures: Vec<ImportFailure>,
        exclude_system: bool,
    ) -> Result<Self> {
        let mut failures = failures;
        let mut spans = Vec::with_capacity(headers.len());
        for header in headers {
            match PreparedSpan::from_header(header.clone()) {
                Ok(span) => spans.push(span),
                Err(error) => failures.push(ImportFailure::from_context(&header, error)?),
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
        let parents: HashSet<_> = spans
            .iter()
            .flat_map(|span| {
                span.source
                    .span_parents
                    .iter()
                    .map(|parent| (span.root_span_id.as_str(), parent.as_str()))
            })
            .collect();
        let roots_with_llms: HashSet<_> = spans
            .iter()
            .filter(|span| span.kind() == "llm")
            .map(|span| span.root_span_id.as_str())
            .collect();
        let ready = spans
            .iter()
            .map(|span| {
                let message_task = span.kind() == "task"
                    && !parents.contains(&(span.root_span_id.as_str(), span.span_id.as_str()))
                    && (span.source.turn.is_some()
                        || !roots_with_llms.contains(span.root_span_id.as_str()));
                !matches!(span.kind(), "llm" | "tool") && !message_task
            })
            .collect();
        Ok(Self {
            ready,
            spans,
            by_id,
            order,
            failures,
            owners: HashMap::new(),
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
        if span.root_span_id != header.root_span_id
            || span.source.span_parents != header.source.span_parents
            || span.span_id != header.span_id
            || span.source.exec_counter != header.source.exec_counter
            || span.kind() != header.kind()
            || span.start != header.start
            || span.source.turn != header.source.turn
            || span.source.analysis != header.source.analysis
            || span.is_scorer() != header.is_scorer()
            || span.source.compaction.as_ref().map(|value| &value.id)
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
            root_span_id: key.0.clone(),
            owner_span_id: key.1.map(|index| self.spans[index].id.clone()),
        }
    }

    fn initialize(&mut self, events: &mut Vec<TrajectoryEvent>) -> Result<()> {
        let by_span_id = self
            .spans
            .iter()
            .enumerate()
            .map(|(index, span)| ((span.root_span_id.as_str(), span.span_id.as_str()), index))
            .collect();
        for index in 0..self.spans.len() {
            let mut visiting = HashSet::new();
            if let Err(message) = ownership(
                index,
                &self.spans,
                &by_span_id,
                &mut self.owners,
                &mut visiting,
            ) {
                for index in visiting {
                    self.owners.insert(
                        index,
                        Ownership {
                            skipped: true,
                            ..Default::default()
                        },
                    );
                    self.failures.push(ImportFailure {
                        root_span_id: self.spans[index].root_span_id.clone(),
                        span_id: self.spans[index].id.clone(),
                        message: message.clone(),
                    });
                }
            }
        }
        let parents: HashSet<_> = self
            .spans
            .iter()
            .flat_map(|span| {
                span.source
                    .span_parents
                    .iter()
                    .map(|parent| (span.root_span_id.as_str(), parent.as_str()))
            })
            .collect();
        let roots_with_llms: HashSet<_> = self
            .spans
            .iter()
            .filter(|span| span.kind() == "llm")
            .map(|span| span.root_span_id.as_str())
            .collect();
        for (index, span) in self.spans.iter().enumerate() {
            let owner = &self.owners[&index];
            if owner.skipped {
                continue;
            }
            let is_message = span.kind() == "task"
                && !parents.contains(&(span.root_span_id.as_str(), span.span_id.as_str()))
                && (span.output.is_empty() && span.source.turn.is_some()
                    || !roots_with_llms.contains(span.root_span_id.as_str()))
                && span.current_input().any(|(index, message)| {
                    matches!(message, Message::User { .. }) && !span.is_context(index, message)
                });
            if span.kind() != "llm" && span.kind() != "tool" && !is_message {
                continue;
            }
            let key = (span.root_span_id.clone(), owner.tool, owner.compaction);
            if is_message && span.source.turn.is_some() && span.output.is_empty() {
                self.task_boundaries.insert(key.clone());
            }
            self.scopes.entry(key).or_default().push(index);
        }
        let mut order_times = HashMap::new();
        for indices in self.scopes.values() {
            let mut llms: Vec<_> = indices
                .iter()
                .copied()
                .filter(|index| self.spans[*index].kind() == "llm")
                .collect();
            llms.sort_by_key(|index| std::cmp::Reverse(self.spans[*index].start));
            let mut earliest_end: Option<DateTime<Utc>> = None;
            for index in llms {
                let span = &self.spans[index];
                if let Some(end) = span.source.end {
                    if earliest_end.is_some_and(|other| other < end) {
                        order_times.insert(index, end);
                        self.resumed.insert(index);
                    }
                    earliest_end = Some(earliest_end.map_or(end, |other| other.min(end)));
                }
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
                key.1
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
            if self
                .spans
                .iter()
                .enumerate()
                .any(|(index, span)| span.kind() == "task" && !self.ready[index])
            {
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

    fn request(&self, span: &PreparedSpan, fresh: Option<&[bool]>) -> Vec<Message> {
        let mut position = 0;
        span.current_input()
            .filter(|(index, message)| {
                if !span.is_context(*index, message) {
                    let include = fresh.is_none_or(|fresh| fresh[position]);
                    position += 1;
                    if !include {
                        return false;
                    }
                }
                matches!(
                    message,
                    Message::User { .. } | Message::System { .. } | Message::Developer { .. }
                ) && (!self.exclude_system || !matches!(message, Message::System { .. }))
            })
            .map(|(_, message)| message.clone())
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
        if span.kind() == "task" {
            return Ok(None);
        }
        let work = if span.kind() == "tool" {
            Work::ToolResult(Box::new(span.tool_result.clone().ok_or_else(|| {
                format!("Missing imported tool result for {}", span.id)
            })?))
        } else if span.source.analysis {
            Work::LLMAnalysis(Box::new(LLMAnalysis {
                work: Some(span.input.iter().chain(&span.output).cloned().collect()),
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
        let final_span = state
            .candidate
            .map(|(index, _)| &self.spans[index])
            .filter(|span| key.2.is_none() && span.can_finish_turn());
        if final_span.is_none() {
            if let Some((index, position)) = state.candidate {
                events.extend(self.work(&scope, id, index, position)?);
            }
        }
        events.push(TrajectoryEvent::Response {
            scope,
            id: id.clone(),
            response_id: final_span.map(|span| span.id.clone()),
            response: final_span.map(PreparedSpan::response),
            end_time: if state.unfinished {
                None
            } else {
                state.end_time
            },
            model: state
                .request_model
                .clone()
                .or_else(|| state.candidate_model.clone()),
        });
        Ok(())
    }

    fn advance(&mut self, index: usize, events: &mut Vec<TrajectoryEvent>) -> Result<()> {
        let span = &self.spans[index];
        let owner = &self.owners[&index];
        let key = (span.root_span_id.clone(), owner.tool, owner.compaction);
        let scope = self.scope(&key);
        let mut state = self.states.remove(&key).unwrap_or_default();
        let explicit = if self.task_boundaries.contains(&key) {
            if span.kind() == "task" {
                span.source.turn.as_deref()
            } else {
                None
            }
        } else {
            owner.turn.as_deref()
        };
        let returning = self.resumed.contains(&index)
            && explicit.is_some_and(|value| {
                state.explicit.as_deref() != Some(value) && state.seen_explicit.contains(value)
            });
        let candidate = !span.source.analysis
            && !returning
            && span.current_input().any(|(index, message)| {
                matches!(message, Message::User { .. }) && !span.is_context(index, message)
            });
        let current_start = span.input_keys.len().saturating_sub(
            span.current_input()
                .filter(|(index, message)| !span.is_context(*index, message))
                .count(),
        );
        let (history, fresh) = merge_history(
            &state.history,
            &span.input_keys,
            &message_keys(&span.output),
        );
        let fresh_request = &fresh[current_start..];
        let has_new_input = fresh_request.iter().any(|fresh| *fresh);
        let interrupts_previous_turn = span.interruption_offsets.iter().any(|offset| {
            fresh
                .get(*offset)
                .is_some_and(|fresh| !has_new_input || *fresh)
        });
        let user_count = span
            .current_input()
            .filter(|(index, message)| {
                matches!(message, Message::User { .. }) && !span.is_context(*index, message)
            })
            .count();
        let new_turn = key.2.is_none()
            && (explicit.is_some()
                && state.explicit.is_some()
                && explicit != state.explicit.as_deref()
                && candidate
                && (user_count <= 1 || has_new_input)
                || candidate && has_new_input);
        let request_filter = has_new_input.then_some(fresh_request);
        if state.id.is_none() || new_turn {
            self.end_turn(&key, &mut state, events)?;
            state.previous_id = state.id.take();
            if key.2.is_none() && interrupts_previous_turn {
                if let Some(id) = &state.previous_id {
                    events.push(TrajectoryEvent::Interrupted {
                        scope: scope.clone(),
                        id: id.clone(),
                    });
                }
            }
            state.id = Some(span.id.clone());
            state.request_found = candidate;
            state.request_model = span.source.model.clone();
            state.candidate_model = None;
            state.candidate = None;
            state.position = 0;
            state.end_time = None;
            state.unfinished = false;
            events.push(TrajectoryEvent::Turn {
                scope: scope.clone(),
                id: span.id.clone(),
                turn: Turn {
                    request_id: span.id.clone(),
                    request: Some(self.request(span, request_filter)),
                    response_id: None,
                    response: None,
                    work: Vec::new(),
                    model: span.source.model.clone(),
                    params: None,
                    start_time: span.start,
                    end_time: None,
                    interrupted: None,
                    compaction: key
                        .2
                        .and_then(|index| self.spans[index].source.compaction.clone()),
                },
            });
        } else if candidate && !state.request_found {
            if key.2.is_none() && interrupts_previous_turn {
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
                request: self.request(span, request_filter),
            });
        }
        let id = state.id.as_deref().unwrap();
        if matches!(span.kind(), "llm" | "task")
            && !span.source.analysis
            && (!span.input_keys.is_empty() || !span.output.is_empty())
        {
            if let Some((previous, position)) = state.candidate.take() {
                events.extend(self.work(&scope, id, previous, position)?);
            }
            if state.candidate_model.is_none() {
                state.candidate_model = span.source.model.clone();
            }
            if key.2.is_none() && span.can_finish_turn() {
                state.candidate = Some((index, state.position));
            } else {
                events.extend(self.work(&scope, id, index, state.position)?);
            }
        } else {
            events.extend(self.work(&scope, id, index, state.position)?);
        }
        state.position += 1;
        state.unfinished =
            !span.can_finish_turn() && (state.unfinished || span.source.end.is_none());
        state.end_time = state.end_time.max(span.source.end);
        if !span.source.analysis && !span.input_keys.is_empty() {
            state.history = history;
        }
        if let Some(explicit) = explicit.filter(|_| new_turn || state.explicit.is_none()) {
            state.explicit = Some(explicit.to_string());
            state.seen_explicit.insert(explicit.to_string());
        }
        self.states.insert(key, state);
        Ok(())
    }

    pub fn finish(&mut self) -> Result<Vec<TrajectoryEvent>> {
        if self.finished {
            return Err("Trajectory stream has finished".to_string());
        }
        let mut events = self.drain()?;
        if !self.initialized || self.cursor < self.order.len() {
            return Err("Trajectory stream has unresolved spans".to_string());
        }
        for (key, mut state) in std::mem::take(&mut self.states) {
            self.end_turn(&key, &mut state, &mut events)?;
        }
        let mut failures = std::mem::take(&mut self.failures);
        failures.extend(self.spans.iter().filter_map(|span| span.failure.clone()));
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
        events.push(TrajectoryEvent::Done);
        self.finished = true;
        Ok(events)
    }
}

#[derive(Default)]
pub struct TrajectoryCollector {
    scopes: BTreeMap<
        TrajectoryScope,
        (
            Trajectory,
            BTreeMap<String, (Turn, BTreeMap<usize, WorkStep>)>,
        ),
    >,
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
                turns.insert(id, (turn, BTreeMap::new()));
            }
            TrajectoryEvent::Request {
                scope,
                id,
                request_id,
                request,
            } => {
                let (turn, _) = self.turn(&scope, &id)?;
                turn.request_id = request_id;
                turn.request = Some(request);
            }
            TrajectoryEvent::Work {
                scope,
                id,
                position,
                step,
            } => {
                self.turn(&scope, &id)?.1.insert(position, step);
            }
            TrajectoryEvent::Response {
                scope,
                id,
                response_id,
                response,
                end_time,
                model,
            } => {
                let (turn, _) = self.turn(&scope, &id)?;
                turn.response_id = response_id;
                turn.response = response;
                turn.end_time = end_time;
                turn.model = model;
            }
            TrajectoryEvent::Interrupted { scope, id } => {
                self.turn(&scope, &id)?.0.interrupted = Some(true)
            }
            TrajectoryEvent::Failure {
                root_span_id,
                span_id,
                message,
            } => {
                let (trajectory, _) = self
                    .scopes
                    .get_mut(&TrajectoryScope {
                        root_span_id,
                        owner_span_id: None,
                    })
                    .ok_or("Unknown trajectory scope")?;
                let failures = trajectory
                    .metadata
                    .entry("import_failures")
                    .or_insert_with(|| json::json!([]));
                let failures = failures
                    .as_array_mut()
                    .ok_or("Invalid trajectory failure collection")?;
                failures.push(json::json!({ "span_id": span_id, "message": message }));
            }
            TrajectoryEvent::Done => self.complete = true,
        }
        Ok(())
    }

    fn turn(
        &mut self,
        scope: &TrajectoryScope,
        id: &str,
    ) -> Result<&mut (Turn, BTreeMap<usize, WorkStep>)> {
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
        for (turn, work) in turns.values() {
            let mut turn = turn.clone();
            turn.work = work
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
        trajectory.turns.sort_by(|a, b| {
            a.start_time
                .cmp(&b.start_time)
                .then(a.request_id.cmp(&b.request_id))
        });
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
