use super::*;

pub fn assemble(
    spans: &[TrajectorySpan],
    failures: &[ImportFailure],
    exclude_system: bool,
) -> Result<Vec<Trajectory>> {
    let mut failures: Vec<_> = failures
        .iter()
        .cloned()
        .chain(spans.iter().filter_map(|span| span.failure.clone()))
        .collect();
    let by_span_id: HashMap<_, _> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            (
                (
                    span.source.root_span_id.as_str(),
                    span.source.span_id.as_deref().unwrap_or(&span.source.id),
                ),
                index,
            )
        })
        .collect();
    let mut owners = HashMap::new();
    for index in 0..spans.len() {
        let mut visiting = HashSet::new();
        if let Err(error) = ownership(index, spans, &by_span_id, &mut owners, &mut visiting) {
            for index in visiting {
                owners.insert(
                    index,
                    Ownership {
                        skipped: true,
                        ..Default::default()
                    },
                );
                failures.push(ImportFailure {
                    root_span_id: spans[index].source.root_span_id.clone(),
                    span_id: spans[index].source.id.clone(),
                    message: error.to_string(),
                });
            }
        }
    }
    let mut scopes: BTreeMap<(&str, Option<usize>, Option<usize>), Vec<usize>> = BTreeMap::new();
    let parents: HashSet<_> = spans
        .iter()
        .flat_map(|span| {
            span.source
                .span_parents
                .iter()
                .map(|parent| (span.source.root_span_id.as_str(), parent.as_str()))
        })
        .collect();
    let roots_with_llms: HashSet<_> = spans
        .iter()
        .filter(|span| span.kind() == "llm")
        .map(|span| span.source.root_span_id.as_str())
        .collect();
    for (index, span) in spans.iter().enumerate() {
        let owner = &owners[&index];
        if owner.skipped || span.source.skipped {
            continue;
        }
        let is_message = span.kind() == "task"
            && !parents.contains(&(
                span.source.root_span_id.as_str(),
                span.source.span_id.as_deref().unwrap_or(&span.source.id),
            ))
            && (span.output.is_empty() && span.turn.is_some()
                || !roots_with_llms.contains(span.source.root_span_id.as_str()))
            && current_input(&span.input)
                .iter()
                .any(|message| matches!(message, Message::User { .. }) && !is_context(message));
        if span.kind() != "llm" && span.kind() != "tool" && !is_message {
            continue;
        }
        scopes
            .entry((&span.source.root_span_id, owner.tool, owner.compaction))
            .or_default()
            .push(index);
    }
    for members in scopes.values_mut() {
        members.sort_by(|a, b| {
            spans[*a]
                .start
                .cmp(&spans[*b].start)
                .then(
                    spans[*a]
                        .source
                        .span_attributes
                        .exec_counter
                        .cmp(&spans[*b].source.span_attributes.exec_counter),
                )
                .then(spans[*a].source.id.cmp(&spans[*b].source.id))
        });
    }
    let mut roots: Vec<_> = spans
        .iter()
        .map(|span| span.source.root_span_id.as_str())
        .chain(failures.iter().map(|failure| failure.root_span_id.as_str()))
        .collect();
    roots.sort_unstable();
    roots.dedup();
    roots
        .into_iter()
        .map(|root| {
            let mut trajectory =
                build_trajectory(root, None, spans, &owners, &scopes, exclude_system)?;
            let mut root_failures: Vec<_> = failures
                .iter()
                .filter(|failure| failure.root_span_id == root)
                .collect();
            root_failures.sort_by(|a, b| a.span_id.cmp(&b.span_id).then(a.message.cmp(&b.message)));
            if !root_failures.is_empty() {
                trajectory.metadata.insert(
                    "import_failures".to_string(),
                    json::to_value(root_failures)
                        .map_err(|err| format!("Failed to serialize trajectory failures: {err}"))?,
                );
            }
            Ok(trajectory)
        })
        .collect()
}

fn build_trajectory(
    root: &str,
    tool: Option<usize>,
    spans: &[TrajectorySpan],
    owners: &HashMap<usize, Ownership>,
    scopes: &BTreeMap<(&str, Option<usize>, Option<usize>), Vec<usize>>,
    exclude_system: bool,
) -> Result<Trajectory> {
    let mut turns: Vec<Turn> = Vec::new();
    for ((scope_root, scope_tool, compaction), members) in scopes {
        if *scope_root != root || *scope_tool != tool {
            continue;
        }
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut history: Vec<u64> = Vec::new();
        let mut explicit: Option<&str> = None;
        let has_user_span_boundaries = members.iter().any(|index| {
            let span = &spans[*index];
            span.kind() == "task" && span.turn.is_some() && span.output.is_empty()
        });
        for index in members {
            let span = &spans[*index];
            let turn = if has_user_span_boundaries {
                if span.kind() == "task" {
                    span.turn.as_deref()
                } else {
                    None
                }
            } else {
                owners[index].turn.as_deref()
            };
            let keys = &span.input_keys;
            let candidate = !span.analysis
                && current_input(&span.input)
                    .iter()
                    .any(|message| matches!(message, Message::User { .. }) && !is_context(message));
            let mut history_cursor = 0;
            let mut has_new_input = false;
            let current_start = keys.len().saturating_sub(message_keys(&span.input).len());
            for (index, key) in keys.iter().enumerate() {
                if let Some(offset) = history[history_cursor..].iter().position(|old| old == key) {
                    history_cursor += offset + 1;
                } else if index >= current_start {
                    has_new_input = true;
                }
            }
            let new_turn = compaction.is_none()
                && (turn.is_some() && explicit.is_some() && turn != explicit
                    || candidate && has_new_input);
            if groups.is_empty() || new_turn {
                groups.push(Vec::new());
            }
            groups.last_mut().unwrap().push(*index);
            if !span.analysis && !keys.is_empty() {
                history.clone_from(keys);
                history.extend(message_keys(&span.output));
            }
            explicit = turn.or(explicit);
        }
        for group in groups {
            let first = &spans[group[0]];
            let request_span = group
                .iter()
                .map(|index| &spans[*index])
                .find(|span| {
                    !span.analysis
                        && current_input(&span.input).iter().any(|message| {
                            matches!(message, Message::User { .. }) && !is_context(message)
                        })
                })
                .unwrap_or(first);
            if compaction.is_none() && interrupts_previous_turn(&request_span.input) {
                if let Some(previous) = turns
                    .iter_mut()
                    .rev()
                    .find(|turn| turn.compaction.is_none())
                {
                    previous.interrupted = Some(true);
                }
            }
            let request = current_input(&request_span.input)
                .iter()
                .filter(|message| {
                    matches!(
                        message,
                        Message::User { .. } | Message::System { .. } | Message::Developer { .. }
                    ) && (!exclude_system || !matches!(message, Message::System { .. }))
                })
                .cloned()
                .collect();
            let candidates: Vec<_> = group
                .iter()
                .map(|index| &spans[*index])
                .filter(|span| matches!(span.kind(), "llm" | "task") && !span.analysis)
                .collect();
            let final_span = candidates
                .last()
                .copied()
                .filter(|span| compaction.is_none() && span.can_finish_turn());
            let mut work = Vec::new();
            for index in &group {
                let span = &spans[*index];
                if final_span.is_some_and(|last| last.source.id == span.source.id)
                    || span.kind() == "task"
                {
                    continue;
                }
                let work_content = if span.kind() == "tool" {
                    Work::ToolResult(Box::new(span.tool_result.clone().unwrap_or(ToolResult {
                        input: None,
                        content: None,
                    })))
                } else if span.analysis {
                    Work::LLMAnalysis(Box::new(LLMAnalysis {
                        work: span.analysis_messages.clone(),
                        model: span.source.model.clone(),
                        params: None,
                        usage: span.usage(),
                    }))
                } else {
                    Work::AgentResponse(Box::new(span.response()))
                };
                let sub_agent = if span.kind() == "tool"
                    && scopes.keys().any(|(_, owner, _)| *owner == Some(*index))
                {
                    Some(Box::new(build_trajectory(
                        root,
                        Some(*index),
                        spans,
                        owners,
                        scopes,
                        exclude_system,
                    )?))
                } else {
                    None
                };
                work.push(WorkStep {
                    id: span.source.id.clone(),
                    span_type: span.kind().to_string(),
                    name: span.source.span_attributes.name.clone(),
                    error: span.source.error.clone(),
                    start_time: span.start,
                    end_time: span.end,
                    work: work_content,
                    sub_agent,
                });
            }
            let end_time = if group.iter().all(|index| spans[*index].end.is_some()) {
                group.iter().filter_map(|index| spans[*index].end).max()
            } else {
                None
            };
            turns.push(Turn {
                request_id: request_span.source.id.clone(),
                request: Some(request),
                response_id: final_span.map(|span| span.source.id.clone()),
                response: final_span.map(TrajectorySpan::response),
                work,
                model: request_span
                    .source
                    .model
                    .clone()
                    .or_else(|| candidates.iter().find_map(|span| span.source.model.clone())),
                params: None,
                start_time: first.start,
                end_time,
                interrupted: None,
                compaction: compaction.and_then(|index| spans[index].compaction.clone()),
            });
        }
    }
    turns.sort_by(|a, b| {
        a.start_time
            .cmp(&b.start_time)
            .then(a.request_id.cmp(&b.request_id))
    });
    Ok(Trajectory {
        version: Some("1".to_string()),
        scope: vec![tool.map_or_else(
            || Scope::Trace {
                trace_id: root.to_string(),
            },
            |index| Scope::Span {
                id: spans[index].source.id.clone(),
            },
        )],
        agent: Agent {
            name: tool.and_then(|index| spans[index].source.span_attributes.name.clone()),
            version: None,
            metadata: Default::default(),
            instructions: None,
        },
        turns,
        sections: None,
        findings: None,
        metadata: Default::default(),
    })
}
