use super::*;
use crate::processing::import::{import_span, import_span_with_options, ImportOptions, Span};
use crate::universal::{UserContent, UserContentPart};
use proptest::prelude::*;
use serde_json::json;
use serde_json::Value;

fn span(id: &str, start: i64, input: Value, output: Value) -> Span {
    Span::deserialize(json!({
        "id": id, "span_id": id, "root_span_id": "root", "span_parents": ["root"],
        "span_attributes": {"type": "llm"}, "metrics": {"start": start, "end": start + 1},
        "input": input, "output": output,
    }))
    .unwrap()
}

fn stream_from_sources(sources: Vec<Span>) -> Result<TrajectoryStream> {
    let headers = sources
        .into_iter()
        .map(|mut source| {
            source.input = None;
            source.output = None;
            import_span(source)
        })
        .collect::<Result<Vec<_>>>()?;
    TrajectoryStream::new(headers, false)
}

fn collect(collector: &mut TrajectoryCollector, events: Vec<TrajectoryEvent>) {
    for event in events {
        let wire = serde_json::to_value(event).unwrap();
        collector
            .push(serde_json::from_value(wire).unwrap())
            .unwrap();
    }
}

#[derive(Clone, Deserialize)]
struct ImportFixture {
    #[serde(default)]
    import_options: ImportOptions,
    spans: Vec<Span>,
    turns: Vec<ExpectedTurn>,
    #[serde(default)]
    worker_responses: Vec<String>,
    start_times: Option<Vec<DateTime<Utc>>>,
    end_times: Option<Vec<Option<DateTime<Utc>>>>,
    compactions: Option<Vec<Value>>,
    request_tools: Option<Vec<Vec<crate::universal::UniversalTool>>>,
    requests: Option<Vec<Vec<Message>>>,
    #[serde(default)]
    import_failures: Vec<crate::serde_json::Value>,
    #[serde(default)]
    import_warnings: Vec<crate::serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct ExpectedTurn {
    request_id: String,
    response_id: Option<String>,
    response_text: String,
    work_ids: Vec<String>,
    user_texts: Vec<String>,
    interrupted: Option<bool>,
}

fn check_import_fixture(fixture: &str) {
    let fixture: ImportFixture = serde_json::from_str(fixture).unwrap();
    check_fixture(&fixture);
    check_fixture_variants(&fixture);
}

fn check_fixture(fixture: &ImportFixture) {
    let mut snapshot = None;
    for (batch_size, import_bodies) in [(1, false), (16, false), (1, true), (16, true)] {
        let trajectories = run_fixture(fixture, batch_size, import_bodies, &[]);
        let actual = serde_json::to_value(&trajectories).unwrap();
        if let Some(expected) = &snapshot {
            assert_eq!(&actual, expected, "Import result depends on load order");
        }
        snapshot = Some(actual);
        assert_eq!(trajectories.len(), 1);
        if let Some(requests) = &fixture.requests {
            let actual: Vec<_> = trajectories[0]
                .turns
                .iter()
                .map(|turn| turn.request.as_ref().unwrap())
                .collect();
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(requests).unwrap(),
            );
        }
        if let Some(request_tools) = &fixture.request_tools {
            let actual: Vec<Vec<_>> = trajectories[0]
                .turns
                .iter()
                .map(|turn| {
                    turn.request
                        .iter()
                        .flatten()
                        .filter_map(|message| match message {
                            Message::AdditionalTools { tools, .. } => Some(tools.clone()),
                            _ => None,
                        })
                        .flatten()
                        .collect()
                })
                .collect();
            assert_eq!(&actual, request_tools);
        }
        assert_eq!(
            trajectories[0]
                .metadata
                .get("import_failures")
                .cloned()
                .unwrap_or(crate::serde_json::json!([])),
            crate::serde_json::json!(fixture.import_failures),
        );
        assert_eq!(
            trajectories[0]
                .metadata
                .get("import_warnings")
                .cloned()
                .unwrap_or(crate::serde_json::json!([])),
            crate::serde_json::json!(fixture.import_warnings),
        );
        if let Some(compactions) = &fixture.compactions {
            assert_eq!(
                trajectories[0]
                    .turns
                    .iter()
                    .filter_map(|turn| turn.compaction.as_ref())
                    .map(|compaction| serde_json::to_value(compaction).unwrap())
                    .collect::<Vec<_>>(),
                *compactions,
            );
        }
        if let Some(start_times) = &fixture.start_times {
            assert_eq!(
                &trajectories[0]
                    .turns
                    .iter()
                    .map(|turn| turn.start_time)
                    .collect::<Vec<_>>(),
                start_times,
            );
        }
        if let Some(end_times) = &fixture.end_times {
            assert_eq!(
                &trajectories[0]
                    .turns
                    .iter()
                    .map(|turn| turn.end_time)
                    .collect::<Vec<_>>(),
                end_times,
            );
        }
        let turns: Vec<_> = trajectories[0]
            .turns
            .iter()
            .map(|turn| ExpectedTurn {
                request_id: turn.request_id.clone(),
                interrupted: turn.interrupted,
                response_id: turn.response_id.clone(),
                response_text: match turn
                    .response
                    .as_ref()
                    .and_then(|response| response.response.as_ref())
                {
                    Some(AssistantContent::String(text)) => text.clone(),
                    Some(AssistantContent::Array(parts)) => parts
                        .iter()
                        .filter_map(|part| match part {
                            AssistantContentPart::Text(text) => Some(text.text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    None => String::new(),
                },
                work_ids: turn.work.iter().map(|step| step.id.clone()).collect(),
                user_texts: turn
                    .request
                    .iter()
                    .flatten()
                    .filter_map(|message| match message {
                        Message::User {
                            content: UserContent::String(text),
                        } => Some(text.clone()),
                        Message::User {
                            content: UserContent::Array(parts),
                        } => Some(
                            parts
                                .iter()
                                .filter_map(|part| match part {
                                    UserContentPart::Text(text) => Some(text.text.as_str()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ),
                        _ => None,
                    })
                    .collect(),
            })
            .collect();
        assert_eq!(turns, fixture.turns);
        let worker_responses: Vec<_> = trajectories[0]
            .turns
            .iter()
            .flat_map(|turn| &turn.work)
            .filter_map(|step| step.sub_agent.as_ref())
            .flat_map(|worker| &worker.turns)
            .filter_map(|turn| turn.response_id.clone())
            .collect();
        assert_eq!(worker_responses, fixture.worker_responses);
    }
    for sources in [
        fixture.spans.clone(),
        fixture.spans.iter().rev().cloned().collect(),
    ] {
        let mut stream = stream_from_sources(fixture.spans.clone()).unwrap();
        let mut collector = TrajectoryCollector::default();
        for source in sources {
            let span = import_span_with_options(source, fixture.import_options).unwrap();
            if stream.needs_payload(span.header.id.as_deref().unwrap()) {
                collect(&mut collector, stream.push(span).unwrap());
            }
        }
        collect(&mut collector, stream.finish().unwrap());
        assert_eq!(
            Some(serde_json::to_value(collector.snapshot().unwrap()).unwrap()),
            snapshot,
            "Import result depends on payload arrival order",
        );
    }
}

fn run_fixture(
    fixture: &ImportFixture,
    batch_size: usize,
    import_bodies: bool,
    priorities: &[u64],
) -> Vec<Trajectory> {
    let mut spans: Vec<_> = fixture.spans.iter().cloned().enumerate().collect();
    if priorities.is_empty() {
        if batch_size > 1 {
            spans.reverse();
        }
    } else {
        spans.sort_by_key(|(index, _)| priorities[*index]);
    }
    let headers: Vec<_> = spans
        .iter()
        .map(|(_, source)| {
            let mut source = source.clone();
            if !import_bodies {
                source.input = None;
                source.output = None;
            }
            import_span_with_options(source, fixture.import_options)
                .unwrap()
                .header_only()
        })
        .collect();
    let source_headers: Vec<_> = headers.iter().map(|span| span.header.clone()).collect();
    let mut stream = TrajectoryStream::new(headers, false).unwrap();
    let mut collector = TrajectoryCollector::default();
    let mut events = Vec::new();
    let snapshot_stride = fixture.spans.len().div_ceil(16).max(1);
    let mut event_count = 0;
    loop {
        let mut ids = stream.pending_ids(usize::MAX);
        if ids.is_empty() {
            break;
        }
        if !priorities.is_empty() {
            ids.sort_by_key(|id| {
                spans
                    .iter()
                    .position(|(_, source)| source.other["id"].as_str() == Some(id.as_str()))
                    .unwrap()
            });
        }
        ids.truncate(batch_size);
        for id in ids.iter().rev() {
            let source = fixture
                .spans
                .iter()
                .find(|source| source.other["id"].as_str() == Some(id.as_str()))
                .unwrap();
            let batch = stream
                .push(import_span_with_options(source.clone(), fixture.import_options).unwrap())
                .unwrap();
            for event in &batch {
                collector.push(event.clone()).unwrap();
                event_count += 1;
                if event_count % snapshot_stride == 0 {
                    collector.snapshot().unwrap();
                }
            }
            events.extend(batch);
        }
    }
    let final_events = stream.finish().unwrap();
    for event in &final_events {
        collector.push(event.clone()).unwrap();
    }
    events.extend(final_events);
    let mut deferred = TrajectoryCollector::default();
    collect(&mut deferred, events);
    assert!(collector.is_complete());
    let result = collector.snapshot().unwrap();
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::to_value(deferred.snapshot().unwrap()).unwrap()
    );
    check_span_coverage(fixture, &source_headers, &result);
    result
}

fn check_span_coverage(
    fixture: &ImportFixture,
    headers: &[SpanContext],
    trajectories: &[Trajectory],
) {
    fn visit<'a>(trajectory: &'a Trajectory, owners: &mut HashMap<&'a str, &'a Trajectory>) {
        for turn in &trajectory.turns {
            let mut ids = HashSet::from([turn.request_id.as_str()]);
            if let Some(id) = &turn.response_id {
                ids.insert(id);
            }
            let mut work_ids = HashSet::new();
            for step in &turn.work {
                assert!(work_ids.insert(&step.id), "Duplicate work span {}", step.id);
                assert_ne!(
                    turn.response_id.as_ref(),
                    Some(&step.id),
                    "Final response duplicated as work"
                );
                ids.insert(&step.id);
                if let Some(worker) = &step.sub_agent {
                    visit(worker, owners);
                }
            }
            for id in ids {
                assert!(
                    owners.insert(id, trajectory).is_none(),
                    "Span {id} belongs to multiple turns"
                );
            }
        }
    }
    let roots: HashSet<_> = trajectories
        .iter()
        .map(|trajectory| match &trajectory.scope[0] {
            Scope::Trace { trace_id } => trace_id,
            _ => panic!("Expected a root trajectory"),
        })
        .collect();
    assert_eq!(
        roots,
        headers
            .iter()
            .filter_map(|header| header.root_span_id.as_ref())
            .collect()
    );
    for trajectory in trajectories {
        let Scope::Trace { trace_id } = &trajectory.scope[0] else {
            panic!("Expected a root trajectory");
        };
        let mut owners = HashMap::new();
        visit(trajectory, &mut owners);
        let scoped: HashMap<_, _> = headers
            .iter()
            .filter(|header| header.root_span_id.as_ref() == Some(trace_id))
            .map(|header| {
                (
                    header.span_id.as_ref().or(header.id.as_ref()).unwrap(),
                    header,
                )
            })
            .collect();
        for header in scoped.values() {
            if !matches!(header.kind.as_str(), "llm" | "tool") || header.start.is_none() {
                continue;
            }
            let mut ancestor = Some(*header);
            let mut visited = HashSet::new();
            let mut excluded = false;
            while let Some(current) = ancestor {
                if current.scorer || current.kind == "score" {
                    excluded = true;
                    break;
                }
                if !visited.insert(&current.id) {
                    excluded = true;
                    break;
                }
                ancestor = current
                    .span_parents
                    .first()
                    .and_then(|id| scoped.get(id).copied());
            }
            if !excluded {
                let id = header.id.as_ref().unwrap();
                assert!(owners.contains_key(id.as_str()), "Lost span {id}");
            }
        }
        for source in &fixture.spans {
            let Some(owner) = source.other["id"].as_str().and_then(|id| owners.get(id)) else {
                continue;
            };
            let imported =
                import_span_with_options(source.clone(), fixture.import_options).unwrap();
            if imported.header.analysis
                || !matches!(imported.header.kind.as_str(), "llm" | "task")
                || matches!(
                    imported.header.name.as_deref(),
                    Some("user_speech" | "agent_speech")
                )
            {
                continue;
            }
            let requests: HashSet<_> = owner
                .turns
                .iter()
                .flat_map(|turn| turn.request.iter().flatten())
                .filter(|message| matches!(message, Message::User { .. }))
                .map(message_dedup_hash)
                .collect();
            for (index, message) in imported
                .input
                .iter()
                .enumerate()
                .rev()
                .take_while(|(_, message)| !matches!(message, Message::Assistant { .. }))
            {
                if matches!(message, Message::User { .. })
                    && !imported.context_messages.contains(&index)
                {
                    assert!(
                        requests.contains(&message_dedup_hash(message)),
                        "Lost current user message from {}",
                        imported.header.id.as_ref().unwrap()
                    );
                }
            }
        }
    }
}

fn check_fixture_variants(fixture: &ImportFixture) {
    let baseline = serde_json::to_value(run_fixture(fixture, 1, false, &[])).unwrap();
    let strategy = (
        proptest::collection::vec(any::<u64>(), fixture.spans.len()),
        1..=fixture.spans.len().max(1),
        any::<bool>(),
    );
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 8,
        failure_persistence: None,
        ..Default::default()
    });
    runner
        .run(&strategy, |(order, batch_size, full)| {
            prop_assert_eq!(
                serde_json::to_value(run_fixture(fixture, batch_size, full, &order)).unwrap(),
                baseline.clone()
            );
            Ok(())
        })
        .unwrap();
    for (kind, purpose) in [("task", None), ("score", None), ("llm", Some("scorer"))] {
        let mut variant = fixture.clone();
        let first = &variant.spans[0];
        variant.spans.push(
            Span::deserialize(json!({
                "id": "fixture-auxiliary", "span_id": "fixture-auxiliary",
                "root_span_id": first.other["root_span_id"],
                "span_parents": [first.other.get("span_id").unwrap_or(&first.other["id"])],
                "span_attributes": {"type": kind, "purpose": purpose}, "metrics": {"start": 0, "end": 1},
                "input": (kind != "task").then(|| json!([{"role": "user", "content": "Score the answer"}])),
                "output": (kind != "task").then(|| json!([{"role": "assistant", "content": "Pass"}]))
            }))
            .unwrap(),
        );
        assert_eq!(
            serde_json::to_value(run_fixture(&variant, 16, false, &[])).unwrap(),
            baseline
        );
    }
    check_content_bearing_auxiliaries(fixture);
}

fn check_content_bearing_auxiliaries(fixture: &ImportFixture) {
    let mut baseline = run_fixture(fixture, 1, false, &[]);
    let mut wrapped = fixture.clone();
    let mut wrappers = Vec::new();
    for source in &mut wrapped.spans {
        let imported = import_span_with_options(source.clone(), fixture.import_options).unwrap();
        if !fixture
            .turns
            .iter()
            .any(|turn| Some(&turn.request_id) == imported.header.id.as_ref())
            || imported.header.analysis
            || !imported.errors.is_empty()
            || !imported.input.iter().enumerate().any(|(index, message)| {
                matches!(message, Message::User { .. })
                    && !imported.context_messages.contains(&index)
            })
        {
            continue;
        }
        let id = format!("fixture-wrapper-{}", imported.header.id.as_ref().unwrap());
        let parents = source
            .other
            .insert("span_parents".into(), crate::serde_json::json!([id]));
        wrappers.push(
            Span::deserialize(json!({
                "id": id, "span_id": id, "root_span_id": imported.header.root_span_id,
                "span_parents": parents, "span_attributes": {"type": "task"},
                "metrics": source.other.get("metrics"), "created": source.other.get("created"),
                "input": source.input,
            }))
            .unwrap(),
        );
    }
    if !wrappers.is_empty() {
        wrapped.spans.extend(wrappers);
        assert_eq!(
            serde_json::to_value(run_fixture(&wrapped, 16, false, &[])).unwrap(),
            serde_json::to_value(&baseline).unwrap(),
            "A wrapper carrying the conversation changed it"
        );
    }

    let parent = fixture
        .turns
        .iter()
        .filter_map(|turn| turn.response_id.as_ref())
        .find_map(|id| {
            let source = fixture
                .spans
                .iter()
                .find(|source| source.other["id"].as_str() == Some(id))?;
            let header = import_span_with_options(source.clone(), fixture.import_options)
                .unwrap()
                .header;
            let start = header.start?;
            let end = header.end?;
            (end > start && header.compaction.is_none()).then_some((header, start, end))
        });
    let Some((parent, start, end)) = parent else {
        return;
    };
    let mut variant = fixture.clone();
    variant.spans.push(Span::deserialize(json!({
        "id": "fixture-analysis", "span_id": "fixture-analysis", "root_span_id": parent.root_span_id,
        "span_parents": [parent.span_id.or(parent.id)],
        "span_attributes": {"type": "llm"}, "metadata": {"trajectory_role": "analysis"},
        "metrics": {"start": (start + (end - start) / 2).timestamp_micros() as f64 / 1e6,
                    "end": end.timestamp_micros() as f64 / 1e6},
        "input": [{"role": "user", "content": "Check the answer"}],
        "output": [{"role": "assistant", "content": "The answer is consistent"}],
    })).unwrap());
    let mut result = run_fixture(&variant, 16, false, &[]);
    let mut found = 0;
    assert_eq!(result[0].turns.len(), baseline[0].turns.len());
    for (turn, expected) in result[0].turns.iter_mut().zip(&mut baseline[0].turns) {
        turn.work.retain(|step| {
            if step.id != "fixture-analysis" {
                return true;
            }
            assert!(matches!(step.work, Work::LLMAnalysis(_)));
            if expected.end_time.is_some() {
                expected.end_time = expected.end_time.max(step.end_time);
            }
            found += 1;
            false
        });
    }
    assert_eq!(found, 1);
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(baseline).unwrap(),
        "An analysis child changed the conversation"
    );
}

#[derive(Clone, Copy, Debug)]
enum TurnIds {
    Inferred,
    SharedAcrossSteering,
    Distinct,
}

impl TurnIds {
    fn value(self, turn: usize) -> Option<String> {
        match self {
            Self::Inferred => None,
            Self::SharedAcrossSteering => Some("shared".into()),
            Self::Distinct => Some(format!("turn-{turn}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ConversationVariation {
    Plain,
    TiedStarts,
    Resumed,
    Compaction,
    Interrupted,
    Context,
    Worker,
}

fn generated_conversation(
    turns: &[(u8, usize, bool)],
    ids: TurnIds,
    variation: ConversationVariation,
) -> ImportFixture {
    let mut spans = Vec::new();
    let mut expected: Vec<ExpectedTurn> = Vec::new();
    let mut worker_responses = Vec::new();
    let mut history = vec![json!({"role": "system", "content": "Initial instructions"})];
    for (turn, &(prompt, tools, retry)) in turns.iter().enumerate() {
        history[0] = json!({"role": "system", "content": format!("Instructions {turn}")});
        if variation == ConversationVariation::Compaction && turn == 1 {
            let mut compact = span("compact", spans.len() as i64 * 2 + 1, json!([]), json!([]));
            compact.other.insert(
                "metadata".into(),
                crate::serde_json::json!({"compaction": true}),
            );
            spans.push(compact);
            expected.push(ExpectedTurn {
                request_id: "compact".into(),
                response_id: None,
                response_text: String::new(),
                user_texts: Vec::new(),
                work_ids: vec!["compact".into()],
                interrupted: None,
            });
            history.drain(1..history.len() - 1);
        }
        if variation == ConversationVariation::Interrupted && turn == 1 {
            history.push(json!({"role": "developer", "content": "<turn_aborted>Previous turn interrupted</turn_aborted>"}));
            expected[0].interrupted = Some(true);
        }
        let request = format!("Request {prompt}");
        history.push(json!({"role": "user", "content": request}));
        let mut user_texts = vec![request];
        if variation == ConversationVariation::Context {
            let context = format!("<environment_context>Environment {turn}</environment_context>");
            history.push(json!({"role": "user", "content": context}));
            user_texts.push(context);
        }
        let first = spans.len();
        let mut work_ids = Vec::new();
        if retry {
            let id = format!("retry-{turn}");
            spans.push(span(
                &id,
                spans.len() as i64 * 2 + 1,
                json!(history),
                json!([]),
            ));
            work_ids.push(id);
        }
        let tools = if variation == ConversationVariation::Worker && turn == 0 {
            tools.max(1)
        } else {
            tools
        };
        for tool in 0..tools {
            let id = format!("call-{turn}-{tool}");
            let call = json!({"role": "assistant", "content": null, "tool_calls": [{
                "type": "function", "id": id,
                "function": {"name": "example_tool", "arguments": "{}"}
            }]});
            spans.push(span(
                &id,
                spans.len() as i64 * 2 + 1,
                json!(history),
                json!([call]),
            ));
            work_ids.push(id.clone());
            history.push(call);
            let tool_id = format!("tool-{turn}-{tool}");
            let start = spans.len() as i64 * 2 + 1;
            let mut result = span(&tool_id, start, json!({}), json!("Result"));
            result.other.insert(
                "span_attributes".into(),
                crate::serde_json::json!({"type": "tool", "name": "example_tool"}),
            );
            result.other.insert(
                "metadata".into(),
                crate::serde_json::json!({"tool_call_id": id}),
            );
            spans.push(result);
            if variation == ConversationVariation::Worker && turn == 0 && tool == 0 {
                let mut worker = span(
                    "worker",
                    start,
                    json!([
                        {"role": "user", "content": "Worker request"}
                    ]),
                    json!([{"role": "assistant", "content": "Worker answer"}]),
                );
                worker
                    .other
                    .insert("span_parents".into(), crate::serde_json::json!([tool_id]));
                spans.push(worker);
                worker_responses.push("worker".into());
            }
            work_ids.push(tool_id);
            history.push(json!({"role": "tool", "tool_call_id": id, "content": "Result"}));
        }
        let final_id = format!("final-{turn}");
        let response_text = format!("Answer {turn}");
        let answer = json!({"role": "assistant", "content": response_text});
        spans.push(span(
            &final_id,
            spans.len() as i64 * 2 + 1,
            json!(history),
            json!([answer]),
        ));
        history.push(answer);
        if let Some(turn_id) = ids.value(turn) {
            for source in &mut spans[first..] {
                source
                    .other
                    .entry("metadata")
                    .or_insert_with(|| crate::serde_json::json!({}))["turn_id"] =
                    crate::serde_json::json!(turn_id);
            }
        }
        expected.push(ExpectedTurn {
            request_id: spans[first].other["id"].as_str().unwrap().to_owned(),
            response_id: Some(final_id),
            response_text,
            user_texts,
            work_ids,
            interrupted: None,
        });
    }
    if variation == ConversationVariation::TiedStarts {
        for (index, source) in spans.iter_mut().enumerate() {
            source.other.insert(
                "metrics".into(),
                crate::serde_json::json!({"start": 1, "end": 2}),
            );
            source.other["span_attributes"]["exec_counter"] = crate::serde_json::json!(index);
        }
    }
    if variation == ConversationVariation::Resumed {
        let first = spans
            .iter()
            .find(|source| source.other["id"] == "final-0")
            .unwrap();
        let mut resumed = span("resumed", 0, json!([]), json!([]));
        resumed.input = first.input.clone();
        resumed.other.insert(
            "metrics".into(),
            crate::serde_json::json!({
                "start": 0.5, "end": spans.len() * 2 + 1,
            }),
        );
        if let Some(turn_id) = ids.value(0) {
            resumed.other.insert(
                "metadata".into(),
                crate::serde_json::json!({"turn_id": turn_id}),
            );
        }
        spans.push(resumed);
        let target = if matches!(ids, TurnIds::Distinct) {
            &mut expected[0]
        } else {
            expected.last_mut().unwrap()
        };
        target.work_ids.push("resumed".into());
    }
    ImportFixture {
        spans,
        turns: expected,
        import_options: ImportOptions::default(),
        worker_responses,
        import_failures: Vec::new(),
        import_warnings: Vec::new(),
        start_times: None,
        end_times: None,
        compactions: None,
        request_tools: None,
        requests: None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn generated_conversations_roundtrip(
        turns in proptest::collection::vec((0u8..3, 0usize..4, any::<bool>()), 2..7),
        ids in prop::sample::select(vec![TurnIds::Inferred, TurnIds::SharedAcrossSteering, TurnIds::Distinct]),
        priorities in proptest::collection::vec(any::<u64>(), 128),
        batch_size in 1usize..20,
    ) {
        use ConversationVariation::*;
        for variation in [Plain, TiedStarts, Resumed, Compaction, Interrupted, Context, Worker] {
            let fixture = generated_conversation(&turns, ids, variation);
            check_fixture(&fixture);
            let baseline = run_fixture(&fixture, 1, false, &[]);
            let shuffled = run_fixture(&fixture, batch_size, false, &priorities);
            prop_assert_eq!(serde_json::to_value(&baseline).unwrap(), serde_json::to_value(shuffled).unwrap());
            for (index, turn) in baseline[0].turns.iter().filter(|turn| turn.compaction.is_none()).enumerate() {
                let Some(Message::System { content: UserContent::String(instructions) }) =
                    turn.request.as_ref().unwrap().first() else { panic!("Missing system instructions") };
                prop_assert_eq!(instructions, &format!("Instructions {index}"));
            }
            if variation == Worker {
                let worker = baseline[0].turns.iter().flat_map(|turn| &turn.work)
                    .find_map(|step| step.sub_agent.as_ref()).unwrap();
                prop_assert_eq!(worker.turns.len(), 1);
                prop_assert_eq!(serde_json::to_value(&worker.turns[0].request).unwrap(), json!([
                    {"role": "user", "content": "Worker request"}
                ]));
            }
            if variation == Plain {
                check_content_bearing_auxiliaries(&fixture);
            }
        }
    }
}

#[test]
fn preserves_opaque_items_without_weakening_strict_imports() {
    let fixture: ImportFixture = serde_json::from_str(include_str!(
        "fixtures/opaque-history-tool-continuation.json"
    ))
    .unwrap();
    let source = fixture.spans[0].clone();
    let strict = import_span(source.clone()).unwrap();
    assert!(strict.input.is_empty());
    assert!(!strict.errors.is_empty());
    let imported = import_span_with_options(source.clone(), fixture.import_options).unwrap();
    assert!(imported.errors.is_empty());
    assert!(matches!(imported.input.as_slice(), [Message::User { .. }]));
    assert_eq!(imported.opaque_input.len(), 1);
    assert_eq!(imported.opaque_input[0].index, Some(0));
    assert_eq!(imported.opaque_input[0].value, source.input.unwrap()[0]);
    assert_eq!(imported.opaque_output[0].value, source.output.unwrap()[0]);
    assert!(imported.header_only().opaque_input.is_empty());
    let item = crate::providers::openai::generated::InputItem::deserialize(
        &imported.opaque_input[0].value,
    )
    .unwrap();
    assert!(<Vec<Message> as crate::universal::convert::TryFromLLM<
        Vec<crate::providers::openai::generated::InputItem>,
    >>::try_from(vec![item])
    .is_err());
}

#[test]
fn best_effort_import_preserves_unsupported_items_and_diagnostics() {
    let spans: Vec<Span> =
        serde_json::from_str(include_str!("fixtures/unsupported-mixed-input.json")).unwrap();
    let source = spans[0].clone();
    let strict = import_span(source.clone()).unwrap();
    assert!(strict.input.is_empty());
    assert!(strict.output.is_empty());
    let imported = import_span_with_options(
        source,
        ImportOptions {
            preserve_unsupported: true,
        },
    )
    .unwrap();
    assert!(!imported.input.is_empty());
    assert!(!imported.output.is_empty());
    assert!(imported.errors.is_empty());
    assert_eq!(
        imported.warnings,
        [
            "input: Unsupported message item at index 0",
            "output: Unsupported message item at index 1",
        ]
    );
    assert_eq!(imported.opaque_input.len(), 1);
    assert_eq!(imported.opaque_output.len(), 1);
    let trajectories = assemble(std::slice::from_ref(&imported), &[], false).unwrap();
    let turn = &trajectories[0].turns[0];
    assert_eq!(turn.request.as_ref().unwrap().len(), 1);
    let response = turn.response.as_ref().unwrap();
    assert!(response.response.is_some());
    assert_eq!(
        response.opaque_input[0].value,
        imported.opaque_input[0].value
    );
    assert_eq!(
        response.opaque_output[0].value,
        imported.opaque_output[0].value
    );
    assert_eq!(response.opaque_output[0].index, Some(1));
    assert!(!trajectories[0].metadata.contains_key("import_failures"));
}

fn check_preserved_metadata_fixture(fixture: &str) {
    let fixture: ImportFixture = serde_json::from_str(fixture).unwrap();
    let source = fixture.spans[0].clone();
    let strict = import_span(source.clone()).unwrap();
    assert!(strict.input.is_empty());
    assert!(!strict.errors.is_empty());
    assert!(strict.warnings.is_empty());
    let imported = import_span_with_options(source.clone(), fixture.import_options).unwrap();
    assert!(imported.errors.is_empty(), "{:?}", imported.errors);
    assert!(!imported.warnings.is_empty());
    assert!(matches!(imported.input.as_slice(), [Message::User { .. }]));
    assert_eq!(imported.opaque_input.len(), 1);
    assert_eq!(imported.opaque_input[0].index, Some(0));
    assert_eq!(imported.opaque_input[0].value, source.input.unwrap()[0]);
    assert!(imported.opaque_input[0].is_metadata);
    check_fixture(&fixture);
    let trajectories = run_fixture(&fixture, 1, false, &[]);
    assert!(trajectories[0].turns[0].opaque_request[0].is_metadata);
}

#[test]
fn preserves_unconvertible_tool_metadata() {
    check_preserved_metadata_fixture(include_str!("fixtures/responses-opaque-tool-metadata.json"));
}

#[test]
fn preserves_unconvertible_file_metadata() {
    check_preserved_metadata_fixture(include_str!("fixtures/responses-cached-file.json"));
}

#[test]
fn message_only_imports_preserve_readable_messages() {
    let spans: Vec<Span> =
        serde_json::from_str(include_str!("fixtures/unsupported-mixed-input.json")).unwrap();
    let expected = crate::processing::import::import_messages_from_spans(vec![Span {
        input: Some(
            crate::serde_json::json!([{ "role": "user", "content": "Acknowledge the report" }]),
        ),
        output: Some(
            crate::serde_json::json!([{ "role": "assistant", "content": "Partial reply" }]),
        ),
        other: Default::default(),
    }]);
    assert_eq!(expected.len(), 2);
    for import in [
        crate::processing::import::import_messages_from_spans,
        crate::processing::import::import_and_deduplicate_messages,
    ] {
        assert_eq!(
            serde_json::to_value(import(vec![spans[0].clone()])).unwrap(),
            serde_json::to_value(&expected).unwrap(),
        );
    }
}

#[test]
fn preserves_compaction_payloads_as_opaque_data() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/nested-compaction-payload.json")).unwrap();
    let source = fixture
        .spans
        .into_iter()
        .find(|span| span.other["id"] == "compact-call")
        .unwrap();
    let original_input = source.input.clone().unwrap();
    let original_output = source.output.clone().unwrap();
    let imported = import_span_with_options(source, fixture.import_options).unwrap();
    assert!(imported.output.is_empty());
    assert!(imported.errors.is_empty());
    assert_eq!(imported.opaque_output.len(), 1);
    assert_eq!(imported.opaque_output[0].index, None);
    assert_eq!(imported.opaque_output[0].value, original_output);
    assert!(imported.input.is_empty());
    assert_eq!(imported.opaque_input.len(), 1);
    assert_eq!(imported.opaque_input[0].index, None);
    assert_eq!(imported.opaque_input[0].value, original_input);
}

#[test]
fn preserves_custom_content_without_inventing_messages() {
    let source: Span =
        serde_json::from_str(include_str!("fixtures/custom-content-payload.json")).unwrap();
    let original_input = source.input.clone().unwrap();
    let original_output = source.output.clone().unwrap();
    let imported = import_span_with_options(
        source,
        ImportOptions {
            preserve_unsupported: true,
        },
    )
    .unwrap();
    assert!(imported.input.is_empty());
    assert!(imported.output.is_empty());
    assert!(imported.errors.is_empty());
    assert!(!imported.warnings.is_empty());
    assert_eq!(imported.opaque_input.len(), 3);
    for (index, item) in imported.opaque_input.iter().enumerate() {
        assert_eq!(item.index, Some(index));
        assert_eq!(item.value, original_input["messages"][index]);
    }
    assert_eq!(imported.opaque_output[0].value, original_output);
    let trajectories = assemble(&[imported], &[], false).unwrap();
    let turn = &trajectories[0].turns[0];
    assert!(turn.request.as_ref().unwrap().is_empty());
    assert_eq!(turn.opaque_request.len(), 3);
    let Work::AgentResponse(response) = &turn.work[0].work else {
        panic!("Expected agent response");
    };
    assert_eq!(response.opaque_output[0].value, original_output);
    assert!(!trajectories[0].metadata.contains_key("import_failures"));
}

macro_rules! import_fixture {
    ($test:ident, $file:literal) => {
        #[test]
        fn $test() {
            check_import_fixture(include_str!($file));
        }
    };
}

#[test]
fn structured_json_survives_trajectory_streaming() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/structured-json-payloads.json")).unwrap();
    check_fixture(&fixture);
    for batch_size in [1, 16] {
        let trajectories = run_fixture(&fixture, batch_size, false, &[]);
        let turn = &trajectories[0].turns[0];
        assert_eq!(
            turn.opaque_request[0].value,
            *fixture.spans[0].input.as_ref().unwrap()
        );
        assert_eq!(turn.opaque_request[0].index, None);
        assert!(!turn.opaque_request[0].is_metadata);
        let Work::AgentResponse(draft) = &turn.work[0].work else {
            panic!("Expected agent response")
        };
        assert_eq!(draft.opaque_input[0].value, turn.opaque_request[0].value);
        assert!(draft.opaque_output.is_empty());
        let Work::LLMAnalysis(analysis) = &turn.work[1].work else {
            panic!("Expected analysis")
        };
        assert_eq!(
            analysis.opaque_input[0].value,
            *fixture.spans[1].input.as_ref().unwrap()
        );
        assert_eq!(
            analysis.opaque_output[0].value,
            *fixture.spans[1].output.as_ref().unwrap()
        );
        assert_eq!(
            turn.response.as_ref().unwrap().opaque_input[0].value,
            *fixture.spans[2].input.as_ref().unwrap()
        );
    }
}

import_fixture!(
    nested_compaction_payload,
    "fixtures/nested-compaction-payload.json"
);
import_fixture!(replayed_abort_marker, "fixtures/replayed-abort-marker.json");
import_fixture!(invalid_turn_ids, "fixtures/invalid-turn-ids.json");
import_fixture!(header_import_errors, "fixtures/header-import-errors.json");
import_fixture!(
    invalid_compaction_hints,
    "fixtures/invalid-compaction-hints.json"
);
import_fixture!(completed_retry, "fixtures/completed-retry.json");
import_fixture!(
    unsupported_only_payloads,
    "fixtures/unsupported-only-payloads.json"
);

import_fixture!(responses_tool_cycle, "fixtures/responses-tool-cycle.json");
import_fixture!(
    responses_attachment_history,
    "fixtures/responses-attachment-history.json"
);
import_fixture!(
    tool_definitions_across_turns,
    "fixtures/tool-definitions-across-turns.json"
);
import_fixture!(
    responses_parent_turns,
    "fixtures/responses-parent-turns.json"
);
import_fixture!(chat_tool_cycle, "fixtures/chat-tool-cycle.json");
import_fixture!(
    anthropic_tool_results_are_not_user_turns,
    "fixtures/anthropic-tool-results-are-not-user-turns.json"
);
import_fixture!(
    nested_worker_and_embedding,
    "fixtures/nested-worker-and-embedding.json"
);
import_fixture!(
    compaction_and_resumed_parent,
    "fixtures/compaction-and-resumed-parent.json"
);
import_fixture!(task_only_trace, "fixtures/task-only-trace.json");
import_fixture!(non_final_task_output, "fixtures/non-final-task-output.json");
import_fixture!(
    standalone_continuation,
    "fixtures/standalone-continuation.json"
);
import_fixture!(
    continuation_with_rewritten_history,
    "fixtures/continuation-with-rewritten-history.json"
);
import_fixture!(
    repeated_standalone_prompts,
    "fixtures/repeated-standalone-prompts.json"
);
import_fixture!(
    repeated_prompts_in_explicit_task,
    "fixtures/repeated-prompts-in-explicit-task.json"
);
import_fixture!(
    task_conversation_with_scorers,
    "fixtures/task-conversation-with-scorers.json"
);
import_fixture!(
    task_conversation_with_analysis,
    "fixtures/task-conversation-with-analysis.json"
);
import_fixture!(
    task_conversation_with_worker,
    "fixtures/task-conversation-with-worker.json"
);
import_fixture!(
    task_conversation_with_root_llm,
    "fixtures/task-conversation-with-root-llm.json"
);
import_fixture!(
    task_parent_with_scorer,
    "fixtures/task-parent-with-scorer.json"
);
import_fixture!(task_parent_with_tool, "fixtures/task-parent-with-tool.json");
import_fixture!(task_parent_with_llm, "fixtures/task-parent-with-llm.json");
import_fixture!(
    task_wrapper_with_task,
    "fixtures/task-wrapper-with-task.json"
);
import_fixture!(
    opaque_history_tool_continuation,
    "fixtures/opaque-history-tool-continuation.json"
);
import_fixture!(
    provider_tools_and_final_answer,
    "fixtures/provider-tools-and-final-answer.json"
);
import_fixture!(
    turns_with_tied_starts,
    "fixtures/turns-with-tied-starts.json"
);
import_fixture!(
    excluded_scorer_import_failures,
    "fixtures/excluded-scorer-import-failures.json"
);
import_fixture!(
    steering_before_tool_result,
    "fixtures/steering-before-tool-result.json"
);
import_fixture!(
    instructions_across_turns,
    "fixtures/instructions-across-turns.json"
);
import_fixture!(empty_assistant_text, "fixtures/empty-assistant-text.json");
import_fixture!(
    analysis_overlapping_conversation,
    "fixtures/analysis-overlapping-conversation.json"
);
import_fixture!(simultaneous_starts, "fixtures/simultaneous-starts.json");
import_fixture!(task_with_empty_child, "fixtures/task-with-empty-child.json");
import_fixture!(
    task_boundary_with_child,
    "fixtures/task-boundary-with-child.json"
);
import_fixture!(
    task_boundary_after_llm,
    "fixtures/task-boundary-after-llm.json"
);

#[test]
fn task_dependencies_allow_non_task_prefetch() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/task-boundary-after-llm.json")).unwrap();
    let stream = stream_from_sources(fixture.spans).unwrap();
    assert_eq!(
        stream.pending_ids(4),
        ["request", "first", "retry", "answer"]
    );
}

import_fixture!(
    analysis_without_conversation,
    "fixtures/analysis-without-conversation.json"
);
import_fixture!(
    reasoning_without_answer,
    "fixtures/reasoning-without-answer.json"
);

#[test]
fn auxiliary_task_structure_does_not_change_the_conversation() {
    for fixture in [
        include_str!("fixtures/task-with-empty-child.json"),
        include_str!("fixtures/task-boundary-with-child.json"),
        include_str!("fixtures/task-parent-with-tool.json"),
        include_str!("fixtures/task-wrapper-with-task.json"),
    ] {
        for hint in [false, true] {
            let mut fixture: ImportFixture = serde_json::from_str(fixture).unwrap();
            let parent = fixture
                .spans
                .iter_mut()
                .find(|span| span.input.is_some())
                .unwrap();
            if hint && parent.other.contains_key("metadata") {
                continue;
            }
            if hint {
                parent.other.insert(
                    "metadata".into(),
                    crate::serde_json::json!({"turn_id": "turn"}),
                );
            }
            let parent_id = parent.other["span_id"].as_str().unwrap().to_string();
            for (kind, purpose) in [("task", None), ("score", None), ("task", Some("scorer"))] {
                let mut variant = fixture.clone();
                variant.spans.push(
                    Span::deserialize(json!({
                        "id": "auxiliary", "span_id": "auxiliary", "root_span_id": "root",
                        "span_parents": [parent_id],
                        "span_attributes": {"type": kind, "purpose": purpose},
                        "metrics": {"start": 2, "end": 3}
                    }))
                    .unwrap(),
                );
                check_fixture(&variant);
            }
        }
    }
}

#[test]
fn task_wrappers_preserve_conversations_and_worker_scopes() {
    for fixture in [
        include_str!("fixtures/responses-tool-cycle.json"),
        include_str!("fixtures/task-parent-with-tool.json"),
        include_str!("fixtures/task-wrapper-with-task.json"),
        include_str!("fixtures/compaction-and-resumed-parent.json"),
    ] {
        let mut fixture: ImportFixture = serde_json::from_str(fixture).unwrap();
        let mut wrappers = Vec::new();
        for (index, span) in fixture.spans.iter_mut().enumerate() {
            let id = format!("wrapper-{index}");
            let parents = span
                .other
                .insert("span_parents".into(), crate::serde_json::json!([id]));
            wrappers.push(
                Span::deserialize(json!({
                    "id": id, "span_id": id, "root_span_id": span.other["root_span_id"],
                    "span_parents": parents,
                    "span_attributes": {"type": "task"},
                    "metrics": span.other["metrics"]
                }))
                .unwrap(),
            );
        }
        fixture.spans.extend(wrappers);
        check_fixture(&fixture);
    }
}

#[test]
fn reviewer_calls_do_not_split_turns_or_replace_the_final_response() {
    let sources: Vec<Span> =
        serde_json::from_str(include_str!("fixtures/reviewer-continuation.json")).unwrap();
    let mut stream = stream_from_sources(sources.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    while let Some(id) = stream.pending_ids(1).first() {
        let source = sources
            .iter()
            .find(|source| source.other["id"].as_str() == Some(id.as_str()))
            .unwrap();
        collect(
            &mut collector,
            stream.push(import_span(source.clone()).unwrap()).unwrap(),
        );
    }
    collect(&mut collector, stream.finish().unwrap());
    let result = collector.snapshot().unwrap();
    assert_eq!(result[0].turns.len(), 2);
    let first = &result[0].turns[0];
    assert_eq!(first.request_id, "agent-call");
    assert_eq!(first.response_id.as_deref(), Some("agent-final"));
    assert!(serde_json::to_string(&first.response)
        .unwrap()
        .contains("First answer"));
    for id in ["review", "trailing-review"] {
        let step = first.work.iter().find(|step| step.id == id).unwrap();
        assert!(matches!(step.work, Work::LLMAnalysis(_)));
    }
    assert_eq!(
        result[0].turns[1].response_id.as_deref(),
        Some("second-final")
    );
}

#[test]
fn streaming_accepts_tool_bodies_after_headers() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/responses-tool-cycle.json")).unwrap();
    let tool = fixture
        .spans
        .iter()
        .find(|source| source.other["id"] == "tool")
        .unwrap();
    let mut stream = stream_from_sources(fixture.spans.clone()).unwrap();
    let events = stream.push(import_span(tool.clone()).unwrap());
    assert!(events.is_ok(), "Tool body was rejected: {events:?}");
}

#[test]
fn changed_end_time_is_rejected_without_consuming_events() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/responses-tool-cycle.json")).unwrap();
    let call = fixture
        .spans
        .iter()
        .find(|span| span.other["id"] == "call")
        .unwrap();
    for end in [None, Some(DateTime::from_timestamp(10, 0).unwrap())] {
        let mut stream = stream_from_sources(fixture.spans.clone()).unwrap();
        let mut collector = TrajectoryCollector::default();
        let tool = fixture
            .spans
            .iter()
            .find(|span| span.other["id"] == "tool")
            .unwrap();
        collect(
            &mut collector,
            stream.push(import_span(tool.clone()).unwrap()).unwrap(),
        );
        let mut changed = import_span(call.clone()).unwrap();
        changed.header.end = end;
        assert!(stream
            .push(changed)
            .unwrap_err()
            .contains("structure changed"));
        for span in &fixture.spans {
            if stream
                .pending_ids(fixture.spans.len())
                .contains(&span.other["id"].as_str().unwrap().to_string())
            {
                collect(
                    &mut collector,
                    stream.push(import_span(span.clone()).unwrap()).unwrap(),
                );
            }
        }
        collect(&mut collector, stream.finish().unwrap());
        let imported = fixture
            .spans
            .iter()
            .cloned()
            .map(import_span)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            serde_json::to_value(collector.snapshot().unwrap()).unwrap(),
            serde_json::to_value(assemble(&imported, &[], false).unwrap()).unwrap(),
        );
    }
}

#[test]
fn non_final_tasks_retain_tool_calls_errors_and_partial_text() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/non-final-task-output.json")).unwrap();
    let spans = fixture
        .spans
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    let turns = &result[0].turns;
    let Work::AgentResponse(response) = &turns[0].work[0].work else {
        panic!("Expected an agent response");
    };
    let Some(AssistantContent::Array(parts)) = &response.response else {
        panic!("Expected response content");
    };
    assert!(
        matches!(&parts[0], AssistantContentPart::ToolCall { tool_call_id, tool_name, .. }
        if tool_call_id == "lookup" && tool_name == "search")
    );
    assert_eq!(
        turns[1].work[0].error,
        Some(crate::serde_json::json!("Source unavailable"))
    );
    let Work::AgentResponse(response) = &turns[2].work[0].work else {
        panic!("Expected an agent response");
    };
    let Some(AssistantContent::Array(parts)) = &response.response else {
        panic!("Expected response content");
    };
    assert!(matches!(&parts[0], AssistantContentPart::Text(text) if text.text == "Partial answer"));
    assert!(turns[2].end_time.is_none());
}

#[test]
fn repeated_standalone_prompts_with_an_explicit_turn_stay_together() {
    let mut fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/repeated-standalone-prompts.json")).unwrap();
    for span in &mut fixture.spans {
        span.other.insert(
            "metadata".into(),
            crate::serde_json::json!({"turn_id": "turn"}),
        );
    }
    let spans = fixture
        .spans
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 1);
    assert_eq!(result[0].turns[0].response_id.as_deref(), Some("second"));
    assert_eq!(result[0].turns[0].work[0].id, "first");
}

#[test]
fn normalized_tool_spans_preserve_input_and_result() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/responses-tool-cycle.json")).unwrap();
    let spans = fixture
        .spans
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let trajectories = assemble(&spans, &[], false).unwrap();
    let turn = &trajectories[0].turns[0];
    assert_eq!(turn.response_id.as_deref(), Some("final"));
    let step = turn.work.iter().find(|step| step.id == "tool").unwrap();
    let Work::ToolResult(tool) = &step.work else {
        panic!("Expected a tool result, got {:?}", step.work);
    };
    let outputs: Vec<_> = tool
        .content
        .iter()
        .flatten()
        .filter_map(|part| match part {
            crate::universal::ToolContentPart::ToolResult(result) => {
                Some(serde_json::to_value(&result.output).unwrap())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        (serde_json::to_value(&tool.input).unwrap(), outputs),
        (json!({}), vec![json!("Result")]),
    );
}

#[test]
fn reviewer_work_preserves_request_and_response_messages() {
    let sources: Vec<Span> =
        serde_json::from_str(include_str!("fixtures/reviewer-continuation.json")).unwrap();
    let spans = sources
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let trajectories = assemble(&spans, &[], false).unwrap();
    let turn = &trajectories[0].turns[0];
    assert_eq!(turn.response_id.as_deref(), Some("agent-final"));
    for (id, request, response) in [
        ("review", "Review request", "Review result"),
        (
            "trailing-review",
            "Another review request",
            "Another review result",
        ),
    ] {
        let step = turn.work.iter().find(|step| step.id == id).unwrap();
        let Work::LLMAnalysis(analysis) = &step.work else {
            panic!("Expected analysis for {id}, got {:?}", step.work);
        };
        let messages = analysis.work.as_ref().expect("Missing reviewer messages");
        assert!(matches!(
            messages.first(),
            Some(Message::User { content: UserContent::String(text) }) if text == request
        ));
        assert!(matches!(
            messages.last(),
            Some(Message::Assistant { content: AssistantContent::Array(parts), .. })
                if matches!(parts.as_slice(), [AssistantContentPart::Text(text)] if text.text == response)
        ));
    }
}

#[test]
fn trajectory_preserves_reasoning_usage_in_work_and_final_response() {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/reasoning-token-usage.json")).unwrap();
    let result = import_rows(&rows);
    let turn = &result[0]["turns"][0];
    assert_eq!(turn["response_id"], "final");
    assert_eq!(
        (
            &turn["work"][0]["work"]["usage"],
            &turn["response"]["usage"],
        ),
        (
            &json!({
                "prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120,
                "prompt_cached_tokens": 80, "completion_reasoning_tokens": 15,
            }),
            &json!({
                "prompt_tokens": 120, "completion_tokens": 10, "total_tokens": 130,
                "prompt_cached_tokens": 100, "completion_reasoning_tokens": 6,
            }),
        ),
    );
}

#[test]
fn trajectory_preserves_cache_write_usage_by_ttl() {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/cache-write-token-usage.json")).unwrap();
    let result = import_rows(&rows);
    assert_eq!(
        result[0]["turns"][0]["response"]["usage"],
        json!({
            "prompt_tokens": 100, "completion_tokens": 10, "total_tokens": 110,
            "prompt_cached_tokens": 20, "prompt_cache_creation_tokens": 70,
            "prompt_cache_creation_5m_tokens": 30, "prompt_cache_creation_1h_tokens": 40,
        }),
    );
}

#[test]
fn unsupported_mixed_input_reports_failure_without_losing_healthy_spans() {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/unsupported-mixed-input.json")).unwrap();
    let imported = import_span(Span::deserialize(&rows[0]).unwrap()).unwrap();
    assert!(imported.input.is_empty());
    assert!(imported.output.is_empty());
    let result = import_rows(&rows);
    assert!(result[0]["turns"]
        .as_array()
        .unwrap()
        .iter()
        .any(|turn| turn["response_id"] == "healthy"));
    let failures = result[0]["metadata"]["import_failures"]
        .as_array()
        .expect("Unsupported input was silently treated as a successful import");
    assert!(failures.iter().any(|failure| {
        failure["span_id"] == "unsupported" && failure["message"].as_str().is_some_and(|message| {
            message == "Unsupported message item at index 0; Unsupported message item at index 1"
        })
    }));
}

#[test]
fn tool_output_without_a_call_id_stays_unpaired() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/chat-tool-cycle.json")).unwrap();
    let rows: Vec<Value> = fixture
        .spans
        .iter()
        .map(|span| serde_json::to_value(span).unwrap())
        .collect();
    let result = import_rows(&rows);
    let tool = result[0]["turns"][0]["work"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["id"] == "tool")
        .unwrap();
    assert_eq!(tool["work"]["input"], json!({}));
    assert_eq!(tool["work"]["output"], json!("Result"));
    assert!(tool["work"]["content"].is_null());
}

#[test]
fn premature_finish_preserves_pending_events() {
    let fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/responses-tool-cycle.json")).unwrap();
    let spans: Vec<_> = fixture
        .spans
        .into_iter()
        .map(|span| import_span(span).unwrap())
        .collect();
    let mut stream =
        TrajectoryStream::new(spans.iter().map(ImportedSpan::header_only).collect(), false)
            .unwrap();
    let mut collector = TrajectoryCollector::default();
    while let Some(id) = stream.pending_ids(1).first() {
        assert_eq!(
            stream.finish().unwrap_err(),
            "Trajectory stream has unresolved spans"
        );
        let span = spans
            .iter()
            .find(|span| span.header.id.as_ref() == Some(id))
            .unwrap();
        collect(&mut collector, stream.push(span.clone()).unwrap());
    }
    collect(&mut collector, stream.finish().unwrap());
    assert!(collector.is_complete());
    assert_eq!(
        serde_json::to_value(collector.snapshot().unwrap()).unwrap(),
        serde_json::to_value(assemble(&spans, &[], false).unwrap()).unwrap(),
    );
}

#[test]
fn emits_request_before_next_payload_and_defers_final_response() {
    let question = json!({"role": "user", "content": "Find the answer"});
    let answer = json!({"role": "assistant", "content": "The answer is 42"});
    let sources = vec![
        span("first", 1, json!([question]), json!([answer])),
        span(
            "second",
            3,
            json!([question, answer, {"role":"user", "content":"Why?"}]),
            json!([{ "role":"assistant", "content":"Because." }]),
        ),
    ];
    let mut stream = stream_from_sources(sources.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(
        &mut collector,
        stream
            .push(import_span(sources[0].clone()).unwrap())
            .unwrap(),
    );
    let partial = collector.snapshot().unwrap();
    assert_eq!(partial[0].turns.len(), 1);
    assert_eq!(partial[0].turns[0].request_id, "first");
    assert!(partial[0].turns[0].response.is_none());
    assert!(!collector.is_complete());
    assert!(stream.finish().is_err());
    collect(
        &mut collector,
        stream
            .push(import_span(sources[1].clone()).unwrap())
            .unwrap(),
    );
    assert_eq!(
        collector.snapshot().unwrap()[0].turns[0]
            .response_id
            .as_deref(),
        Some("first")
    );
    collect(&mut collector, stream.finish().unwrap());
    assert!(collector.is_complete());
    let completed = collector.snapshot().unwrap();
    assert_eq!(completed[0].turns.len(), 2);
    assert_eq!(completed[0].turns[1].request_id, "second");
    assert_eq!(completed[0].turns[1].response_id.as_deref(), Some("second"));
    assert!(completed[0].turns.iter().all(|turn| turn.work.is_empty()));
    assert!(stream.finish().is_err());
    assert!(collector.push(TrajectoryEvent::Done).is_err());
}

#[test]
fn out_of_order_payloads_do_not_reorder_turns() {
    let sources = vec![
        span(
            "first",
            1,
            json!([{"role":"user","content":"First"}]),
            json!([]),
        ),
        span(
            "second",
            3,
            json!([{"role":"user","content":"Second"}]),
            json!([]),
        ),
    ];
    let mut stream = stream_from_sources(sources.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(
        &mut collector,
        stream
            .push(import_span(sources[1].clone()).unwrap())
            .unwrap(),
    );
    assert!(collector.snapshot().unwrap()[0].turns.is_empty());
    collect(
        &mut collector,
        stream
            .push(import_span(sources[0].clone()).unwrap())
            .unwrap(),
    );
    assert_eq!(collector.snapshot().unwrap()[0].turns.len(), 2);
    assert!(stream
        .push(import_span(sources[0].clone()).unwrap())
        .is_err());
    collect(&mut collector, stream.finish().unwrap());
}

#[test]
fn malformed_timing_preserves_healthy_turns_and_reports_failure() {
    let healthy = span(
        "healthy",
        1,
        json!([{"role":"user","content":"Hello"}]),
        json!([]),
    );
    let mut broken = healthy.clone();
    broken
        .other
        .insert("id".into(), crate::serde_json::json!("broken"));
    broken
        .other
        .insert("metrics".into(), crate::serde_json::json!({}));
    let mut stream = stream_from_sources(vec![healthy.clone(), broken]).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(
        &mut collector,
        stream.push(import_span(healthy).unwrap()).unwrap(),
    );
    collect(&mut collector, stream.finish().unwrap());
    let result = collector.snapshot().unwrap();
    assert_eq!(result[0].turns.len(), 1);
    assert_eq!(
        result[0].metadata["import_failures"][0]["span_id"],
        "broken"
    );
}

#[test]
fn failed_payload_does_not_block_later_turns() {
    let mut broken = span("broken", 1, json!([]), json!([]));
    broken
        .other
        .insert("metadata".into(), crate::serde_json::json!({"model": {}}));
    let sources = vec![
        broken,
        span(
            "healthy",
            2,
            json!([{"role":"user", "content":"Hello"}]),
            json!([]),
        ),
    ];
    let mut stream = stream_from_sources(sources.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(
        &mut collector,
        stream
            .fail("broken", "Invalid payload".to_string())
            .unwrap(),
    );
    collect(
        &mut collector,
        stream
            .push(import_span(sources[1].clone()).unwrap())
            .unwrap(),
    );
    collect(&mut collector, stream.finish().unwrap());
    let result = collector.snapshot().unwrap();
    assert_eq!(result[0].turns.len(), 1);
    assert_eq!(result[0].turns[0].request_id, "healthy");
    assert_eq!(
        result[0].metadata["import_failures"],
        crate::serde_json::json!([
            {
                "span_id": "broken",
                "message": "Invalid metadata.model: invalid type: map, expected a string"
            },
            {"span_id": "broken", "message": "Invalid payload"}
        ])
    );
}

#[test]
fn interruption_in_later_request_marks_the_previous_turn() {
    let question = json!({"role": "user", "content": "Again"});
    let mut first = span("first", 1, json!([question]), json!([]));
    first.other.insert(
        "metadata".into(),
        crate::serde_json::json!({"turn_id": "one"}),
    );
    let mut tool = span("tool", 3, json!([]), json!([]));
    tool.other.insert(
        "span_attributes".into(),
        crate::serde_json::json!({"type": "tool"}),
    );
    tool.other.insert(
        "metadata".into(),
        crate::serde_json::json!({"turn_id": "two"}),
    );
    let mut request = span(
        "request",
        5,
        json!([{"role": "system", "content": "<turn_aborted>Interrupted</turn_aborted>"}, question]),
        json!([]),
    );
    request.other.insert(
        "metadata".into(),
        crate::serde_json::json!({"turn_id": "two"}),
    );
    let spans = vec![first, tool, request]
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let expected = assemble(&spans, &[], false).unwrap();
    assert_eq!(expected[0].turns.len(), 2);
    assert_eq!(expected[0].turns[0].interrupted, Some(true));
    assert_eq!(expected[0].turns[1].request_id, "request");
}

#[test]
fn compacted_history_does_not_reintroduce_old_user_requests() {
    let first = json!({"role": "user", "content": "First task"});
    let second = json!({"role": "user", "content": "Second task"});
    let third = json!({"role": "user", "content": "Third task"});
    let reply = json!([{"role": "assistant", "content": "Done"}]);
    let sources = vec![
        span("first", 1, json!([first]), reply.clone()),
        span("second", 3, json!([second]), reply.clone()),
        span("compacted", 5, json!([first, second]), json!([])),
        span("third", 7, json!([first, second, third]), reply),
    ];
    let spans = sources
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 3);
    assert_eq!(
        serde_json::to_value(&result[0].turns[2].request).unwrap(),
        json!([third])
    );
}

#[test]
fn repeated_requests_after_compaction_are_distinct_occurrences() {
    let question = json!({"role": "user", "content": "Again"});
    let sources = vec![
        span(
            "first",
            1,
            json!([question]),
            json!([{"role": "assistant", "content": "First answer"}]),
        ),
        span(
            "second",
            3,
            json!([question, question]),
            json!([{"role": "assistant", "content": "Second answer"}]),
        ),
    ];
    let spans = sources
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 2);
    assert_eq!(
        serde_json::to_value(&result[0].turns[1].request).unwrap(),
        json!([question])
    );
}

import_fixture!(
    resumed_call_does_not_pull_later_requests_before_their_work,
    "fixtures/overlapping-resumed-call.json"
);

#[test]
fn overlapping_calls_preserve_the_later_answer_unless_they_continue_it() {
    for explicit in [false, true] {
        for output in [
            json!([]),
            json!([{"role": "assistant", "content": "Older answer"}]),
        ] {
            let mut fixture: ImportFixture =
                serde_json::from_str(include_str!("fixtures/overlapping-resumed-call.json"))
                    .unwrap();
            fixture.spans[1].output = Some(crate::serde_json::Value::deserialize(output).unwrap());
            if explicit {
                for (span, turn) in fixture
                    .spans
                    .iter_mut()
                    .zip(["first", "first", "second", "third"])
                {
                    span.other.insert(
                        "metadata".into(),
                        crate::serde_json::json!({"turn_id": turn}),
                    );
                }
                fixture.turns[0].work_ids.push("resumed".into());
                fixture.turns[2].work_ids.clear();
                fixture.end_times = Some(
                    vec![10, 6, 8]
                        .into_iter()
                        .map(|time| DateTime::from_timestamp(time, 0))
                        .collect(),
                );
            }
            check_fixture(&fixture);
        }
    }
    let mut fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/overlapping-resumed-call.json")).unwrap();
    fixture.spans[1].input = Some(crate::serde_json::json!([
        {"role": "user", "content": "Third task"},
        {"role": "assistant", "content": "Done"}
    ]));
    fixture.spans[1].output =
        Some(crate::serde_json::json!([{"role": "assistant", "content": "Follow-up"}]));
    fixture.turns[2].work_ids = vec!["third".into()];
    fixture.turns[2].response_id = Some("resumed".into());
    fixture.turns[2].response_text = "Follow-up".into();
    check_fixture(&fixture);
}

import_fixture!(
    resumed_explicit_turn_owns_late_work,
    "fixtures/resumed-explicit-turn.json"
);

#[test]
fn resumed_calls_use_their_parent_tasks_turn_ids() {
    let mut fixture: ImportFixture =
        serde_json::from_str(include_str!("fixtures/resumed-explicit-turn.json")).unwrap();
    let mut parents = Vec::new();
    for source in &mut fixture.spans {
        let id = source.other["id"].as_str().unwrap();
        let parent_id = match id {
            "resumed" | "resumed-again" => "parent-first".to_string(),
            "resumed-after-steering" => "parent-second".to_string(),
            _ => format!("parent-{id}"),
        };
        if let Some(turn) = fixture.turns.iter_mut().find(|turn| turn.request_id == id) {
            parents.push(
                Span::deserialize(json!({
                    "id": parent_id, "span_id": parent_id, "root_span_id": "root",
                    "span_attributes": {"type": "task"}, "metrics": source.other["metrics"],
                    "metadata": source.other["metadata"], "input": source.input,
                }))
                .unwrap(),
            );
            turn.request_id = parent_id.clone();
            let start = source.other["metrics"]["start"].as_f64().unwrap();
            source.other["metrics"]["start"] = crate::serde_json::json!(start + 0.1);
        }
        source.other.remove("metadata");
        source
            .other
            .insert("span_parents".into(), crate::serde_json::json!([parent_id]));
    }
    fixture.spans.extend(parents);
    check_fixture(&fixture);
}

#[test]
fn replayed_history_before_a_known_message_is_not_a_new_request() {
    let first = json!({"role": "user", "content": "First task with image"});
    let replay = json!({"role": "user", "content": "First task, image omitted"});
    let second = json!({"role": "user", "content": "Second task"});
    let third = json!({"role": "user", "content": "Third task"});
    let sources = vec![
        span("first", 1, json!([first]), json!([])),
        span("second", 3, json!([second]), json!([])),
        span("replay", 5, json!([replay, second]), json!([])),
        span("third", 7, json!([replay, second, third]), json!([])),
    ];
    let spans = sources
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 3);
    assert_eq!(result[0].turns[1].work.len(), 2);
    assert_eq!(
        serde_json::to_value(&result[0].turns[2].request).unwrap(),
        json!([third])
    );
}

#[test]
fn resumed_parent_turn_keeps_reformatted_replay_out_of_the_request() {
    let question = json!({"role": "user", "content": "First"});
    let second = json!({"role": "user", "content": "Second with image"});
    let replay = json!({"role": "user", "content": "Second with image omitted"});
    let mut first = span("first", 1, json!([question]), json!([]));
    first.other.insert(
        "metadata".into(),
        crate::serde_json::json!({"turn_id": "one"}),
    );
    let mut resumed = span("resumed", 3, json!([question, replay]), json!([]));
    resumed
        .other
        .insert("metadata".into(), first.other["metadata"].clone());
    resumed.other["metrics"]["end"] = crate::serde_json::json!(10.0);
    let mut next = span("second", 5, json!([second]), json!([]));
    next.other.insert(
        "metadata".into(),
        crate::serde_json::json!({"turn_id": "two"}),
    );
    let spans = vec![first, resumed, next]
        .into_iter()
        .map(import_span)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 2);
    assert_eq!(
        result[0].turns[0]
            .work
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "resumed"]
    );
    assert_eq!(
        result[0].turns[1]
            .work
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>(),
        vec!["second"]
    );
    assert_eq!(
        serde_json::to_value(&result[0].turns[1].request).unwrap(),
        json!([second])
    );
}

fn trajectory_row(id: &str, start: i64, input: Value, output: Value) -> Value {
    serde_json::json!({
        "id": id, "span_id": id, "root_span_id": "root", "span_parents": ["root"],
        "metrics": { "start": start, "end": start + 1 },
        "span_attributes": { "type": "llm", "name": id },
        "input": input, "output": output,
    })
}

fn import_rows(rows: &[Value]) -> Value {
    let mut spans = Vec::new();
    let mut failures = Vec::new();
    for row in rows {
        match Span::deserialize(row)
            .map_err(|error| error.to_string())
            .and_then(import_span)
        {
            Ok(span) => spans.push(span),
            Err(error) => failures.push(ImportFailure {
                root_span_id: row["root_span_id"].as_str().unwrap().into(),
                span_id: row["id"].as_str().unwrap().into(),
                message: error,
            }),
        }
    }
    serde_json::to_value(assemble(&spans, &failures, false).unwrap()).unwrap()
}

#[test]
fn trajectory_preserves_interruption_markers_without_matching_user_quotes() {
    use serde_json::json;
    let user = json!({ "role": "user", "content": "Search" });
    let reply = json!({ "role": "assistant", "content": "Working" });
    for role in ["system", "developer", "user"] {
        let rows = [
            trajectory_row("first", 1, json!([user]), json!([reply])),
            trajectory_row(
                "second",
                3,
                json!([user, reply, {
                "role": role, "content": "<turn_aborted>Previous turn interrupted</turn_aborted>"
            }, { "role": "user", "content": "Try again" }]),
                json!(null),
            ),
        ];
        let result = import_rows(&rows);
        assert_eq!(
            result[0]["turns"][0]["interrupted"],
            if role == "user" {
                json!(null)
            } else {
                json!(true)
            }
        );
    }
}

#[test]
fn trajectory_direct_messages_override_inherited_session_turn_ids() {
    use serde_json::json;
    let first = json!({ "role": "user", "content": "Hello" });
    let second = json!({ "role": "user", "content": "Search" });
    let reply = json!({ "role": "assistant", "content": "Hello back" });
    let mut direct = trajectory_row("user", 1, json!([first]), json!(null));
    direct["span_attributes"]["type"] = json!("task");
    direct["metadata"] = json!({ "turn_id": "user" });
    let mut next = trajectory_row("next", 9, json!([second]), json!(null));
    next["span_attributes"]["type"] = json!("task");
    next["metadata"] = json!({ "turn_id": "next" });
    let mut greeting = trajectory_row("greeting", 3, json!([first]), json!([reply]));
    greeting["metrics"]["end"] = json!(10);
    let mut call = trajectory_row("call", 12, json!([first, reply, second]), json!([]));
    call["metadata"] = json!({ "turn_id": "session" });
    let rows = [
        json!({ "id": "root", "span_id": "root", "root_span_id": "root", "metrics": { "start": 0, "end": 30 }, "span_attributes": { "type": "task" }, "metadata": { "turn_id": "session" } }),
        direct,
        greeting,
        next,
        call,
    ];
    let result = import_rows(&rows);
    let turns = result[0]["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0]["request_id"], "user");
    assert_eq!(turns[0]["response_id"], "greeting");
    assert_eq!(turns[1]["request_id"], "next");
    assert_eq!(turns[1]["work"][0]["id"], "call", "{result}");
}

#[test]
fn trajectory_does_not_promote_an_acknowledgement_before_a_tool_call() {
    use serde_json::json;
    let user = json!({ "role": "user", "content": "Search" });
    let reply = json!({ "role": "assistant", "content": "I'll check" });
    let rows = [
        trajectory_row("ack", 1, json!([user]), json!([reply])),
        trajectory_row(
            "call",
            3,
            json!([user, reply]),
            json!([{
                "role": "assistant", "tool_calls": [{ "id": "call", "type": "function", "function": { "name": "search", "arguments": "{}" } }]
            }]),
        ),
    ];
    let result = import_rows(&rows);
    assert!(result[0]["turns"][0].get("response_id").is_none());
    assert_eq!(result[0]["turns"][0]["work"].as_array().unwrap().len(), 2);
}

#[test]
fn trajectory_promotes_responses_api_final_output() {
    use serde_json::json;
    let messages = json!([{
        "type": "message", "role": "assistant", "status": "completed",
        "content": [{ "type": "output_text", "text": "Final answer", "annotations": [] }]
    }]);
    for output in [messages.clone(), json!({ "output": messages })] {
        let row = trajectory_row(
            "final",
            1,
            json!([{ "role": "user", "content": "Question" }]),
            output,
        );
        let result = import_rows(&[row]);
        assert_eq!(result[0]["turns"][0]["response_id"], "final");
        assert_eq!(
            result[0]["turns"][0]["response"]["response"][0]["text"],
            "Final answer"
        );
    }
}

#[test]
fn trajectory_keeps_payloads_and_valid_hints_when_metadata_is_malformed() {
    use serde_json::json;
    for metadata in [
        json!(true),
        json!({ "model": 7, "compaction": "invalid", "turn_id": "turn", "trajectory_role": "agent" }),
    ] {
        let mut row = trajectory_row(
            "valid",
            1,
            json!([{ "role": "user", "content": "Hello" }]),
            json!([{ "role": "assistant", "content": "Answer" }]),
        );
        row["metadata"] = metadata;
        let result = import_rows(&[row]);
        assert_eq!(result[0]["turns"][0]["response_id"], "valid");
        assert_eq!(
            result[0]["turns"][0]["response"]["response"][0]["text"],
            "Answer"
        );
        assert_eq!(
            result[0]["metadata"]["import_failures"][0]["span_id"],
            "valid"
        );
    }
    let mut wrapper = trajectory_row("wrapper", 1, json!(null), json!(null));
    wrapper["span_attributes"]["type"] = json!("task");
    wrapper["metadata"] = json!({ "model": {}, "compaction": true });
    let mut child = trajectory_row(
        "child",
        2,
        json!([]),
        json!([{ "role": "assistant", "content": "Summary" }]),
    );
    child["span_parents"] = json!(["wrapper"]);
    let result = import_rows(&[wrapper, child]);
    assert_eq!(result[0]["turns"][0]["compaction"]["id"], "wrapper");
    assert!(result[0]["turns"][0]["response_id"].is_null());
}

#[test]
fn trajectory_uses_created_when_start_is_missing_or_out_of_range() {
    use serde_json::json;
    for start in [Value::Null, json!(1e100)] {
        let mut row = trajectory_row(
            "valid",
            1,
            json!([{ "role": "user", "content": "Hello" }]),
            json!([]),
        );
        row["metrics"]["start"] = start;
        row["created"] = json!("2026-09-26T00:00:00Z");
        let result = import_rows(&[row]);
        assert_eq!(result[0]["turns"][0]["start_time"], "2026-09-26T00:00:00Z");
        assert!(result[0]["turns"][0]["end_time"].is_null());
        assert!(result[0]["metadata"]["import_failures"].is_null());
    }
}

#[test]
fn trajectory_isolates_parent_cycles() {
    use serde_json::json;
    let mut first = trajectory_row("first", 1, json!([]), json!([]));
    first["span_parents"] = json!(["second"]);
    let mut second = trajectory_row("second", 2, json!([]), json!([]));
    second["span_parents"] = json!(["first"]);
    let healthy = trajectory_row(
        "healthy",
        3,
        json!([{ "role": "user", "content": "Hello" }]),
        json!([]),
    );
    let result = import_rows(&[first, second, healthy]);
    assert_eq!(result[0]["turns"][0]["request_id"], "healthy");
    assert_eq!(
        result[0]["metadata"]["import_failures"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn trajectory_accepts_boolean_and_structured_compaction_metadata() {
    use serde_json::json;
    for (compaction, expected) in [
        (json!(true), json!({ "id": "wrapper" })),
        (json!(false), Value::Null),
        (json!({}), Value::Null),
        (
            json!({ "replaced_message_count": 12 }),
            json!({ "id": "wrapper", "replaced_message_count": 12 }),
        ),
        (Value::Null, Value::Null),
    ] {
        let mut child = trajectory_row(
            "child",
            2,
            json!([{ "role": "user", "content": "Summarize" }]),
            json!([{ "role": "assistant", "content": "Summary" }]),
        );
        child["span_parents"] = json!(["wrapper"]);
        let rows = [
            json!({ "id": "wrapper", "span_id": "wrapper", "root_span_id": "root", "metrics": { "start": 1, "end": 3 }, "span_attributes": { "type": "task", "name": "Context update" }, "metadata": { "compaction": compaction } }),
            child,
        ];
        let result = import_rows(&rows);
        let turns = result[0]["turns"].as_array().unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0]["compaction"], expected);
        assert_eq!(turns[0]["response_id"].is_null(), !expected.is_null());
    }
}

#[test]
fn trajectory_separates_compaction_and_excludes_scorer_descendants() {
    use serde_json::json;
    let mut compact = trajectory_row(
        "compact",
        3,
        json!([{ "role": "user", "content": "Summarize" }]),
        json!([]),
    );
    compact["span_parents"] = json!(["compaction"]);
    let mut score = compact.clone();
    score["id"] = json!("score-child");
    score["span_id"] = json!("score-child");
    score["span_parents"] = json!(["score"]);
    let rows = [
        trajectory_row(
            "first",
            1,
            json!([{ "role": "user", "content": "Request" }]),
            json!([]),
        ),
        json!({ "id": "compaction", "span_id": "compaction", "root_span_id": "root", "span_parents": ["first"], "metrics": { "start": 2, "end": 4 }, "span_attributes": { "type": "task", "name": "compaction" } }),
        compact,
        json!({ "id": "score", "span_id": "score", "root_span_id": "root", "metrics": { "start": 2, "end": 4 }, "span_attributes": { "type": "score" } }),
        score,
    ];
    let result = import_rows(&rows);
    assert_eq!(result[0]["turns"].as_array().unwrap().len(), 2);
    assert_eq!(result[0]["turns"][1]["compaction"]["id"], "compaction");
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("score-child"));
}

#[test]
fn trajectory_keeps_retries_and_tool_continuations_in_one_turn() {
    use serde_json::json;
    let user = json!({ "role": "user", "content": "Look it up" });
    let call = json!({ "role": "assistant", "content": "Working", "tool_calls": [{
        "id": "call", "type": "function", "function": { "name": "lookup", "arguments": "{}" }
    }] });
    let rows = [
        trajectory_row("first", 1, json!([user]), json!([call])),
        trajectory_row(
            "retry",
            3,
            json!([user, {
                "role": "user", "content": "<external_braintrust.runtime>Updated context</external_braintrust.runtime>"
            }]),
            json!([call]),
        ),
        trajectory_row(
            "done",
            5,
            json!([user, call, {
                "role": "tool", "tool_call_id": "call", "content": "Found it"
            }]),
            json!([{ "role": "assistant", "content": "Here it is" }]),
        ),
    ];
    let result = import_rows(&rows);
    assert_eq!(result[0]["turns"].as_array().unwrap().len(), 1);
    assert_eq!(result[0]["turns"][0]["work"].as_array().unwrap().len(), 2);
    assert_eq!(result[0]["turns"][0]["response_id"], "done");
}

fn voice_fixture_trajectories(fixture: &str) -> Vec<crate::universal::Trajectory> {
    voice_trajectories_in_order(fixture, 4, false)
}

fn voice_trajectories_in_order(
    fixture: &str,
    limit: usize,
    reverse: bool,
) -> Vec<crate::universal::Trajectory> {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let spans: Vec<Span> = serde_json::from_value(fixture["spans"].clone()).unwrap();
    let mut stream = stream_from_sources(spans.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    loop {
        let mut ids = stream.pending_ids(limit);
        if ids.is_empty() {
            break;
        }
        if reverse {
            ids.reverse();
        }
        for id in ids {
            let source = spans
                .iter()
                .find(|span| {
                    span.other.get("id").and_then(|value| value.as_str()) == Some(id.as_str())
                })
                .cloned()
                .unwrap();
            collect(
                &mut collector,
                stream.push(import_span(source).unwrap()).unwrap(),
            );
        }
    }
    collect(&mut collector, stream.finish().unwrap());
    collector.snapshot().unwrap()
}

#[test]
fn voice_trajectory_lists_recordings_from_headers() {
    let trajectories = voice_fixture_trajectories(include_str!(
        "fixtures/voice/livekit-realtime-order-lookup.json"
    ));
    assert_eq!(trajectories.len(), 1);
    let recordings = trajectories[0].recordings.as_ref().unwrap();
    assert_eq!(recordings.len(), 1);
    assert_eq!(recordings[0].id, "call-0000");
    assert_eq!(
        recordings[0].attachment_span_id.as_deref(),
        Some("ec099e46fa1da84e")
    );
    assert_eq!(recordings[0].channels.len(), 2);

    let edges =
        voice_fixture_trajectories(include_str!("fixtures/voice/fictional-edge-cases.json"));
    let ids: Vec<_> = edges[0]
        .recordings
        .as_ref()
        .unwrap()
        .iter()
        .map(|recording| (recording.id.as_str(), recording.state.as_deref()))
        .collect();
    assert_eq!(
        ids,
        vec![
            ("seg-0", Some("ready")),
            ("seg-1", Some("ready")),
            ("agent-clip", Some("pending"))
        ]
    );
}

#[test]
fn trajectories_without_voice_have_no_recordings() {
    let stream = stream_from_sources(vec![span("a", 1, json!([]), json!([]))]);
    let mut collector = TrajectoryCollector::default();
    let mut stream = stream.unwrap();
    collect(&mut collector, stream.finish().unwrap_or_default());
    for trajectory in collector.snapshot().unwrap_or_default() {
        assert!(trajectory.recordings.is_none());
    }
}

const LIVEKIT_VOICE: &str = include_str!("fixtures/voice/livekit-realtime-order-lookup.json");
const CASCADE_VOICE: &str = include_str!("fixtures/voice/fictional-cascade-turns.json");
const EDGE_VOICE: &str = include_str!("fixtures/voice/fictional-edge-cases.json");

struct PlacedSpeech<'a> {
    turn: &'a Turn,
    step: &'a WorkStep,
    speech: &'a SpeechWork,
}

fn placed_speech(trajectory: &Trajectory) -> Vec<PlacedSpeech<'_>> {
    trajectory
        .turns
        .iter()
        .flat_map(|turn| {
            turn.work.iter().filter_map(move |step| match &step.work {
                Work::Speech(speech) => Some(PlacedSpeech { turn, step, speech }),
                _ => None,
            })
        })
        .collect()
}

fn speech_by_id<'a>(placed: &'a [PlacedSpeech<'a>], id: &str) -> &'a PlacedSpeech<'a> {
    placed.iter().find(|placed| placed.step.id == id).unwrap()
}

fn observed_step(speech: &SpeechWork) -> Option<(&str, &str)> {
    match &speech.placement {
        SpeechPlacement::Observed {
            relationship,
            step_id,
            ..
        } => Some((relationship.as_str(), step_id.as_str())),
        SpeechPlacement::Inferred { .. } => None,
    }
}

fn fixture_headers(fixture: &str) -> HashMap<String, SpanContext> {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let spans: Vec<Span> = serde_json::from_value(fixture["spans"].clone()).unwrap();
    spans
        .into_iter()
        .map(|span| {
            let header = import_span(span).unwrap().header;
            (header.id.clone().unwrap(), header)
        })
        .collect()
}

fn has_link(header: &SpanContext, relationship: &str, target: &SpanContext) -> bool {
    header
        .voice
        .iter()
        .flat_map(|voice| &voice.links)
        .any(|link| {
            link.relationship == relationship && Some(&link.span_id) == target.span_id.as_ref()
        })
}

fn assert_speech_matches_links(fixture: &str, trajectory: &Trajectory) {
    let headers = fixture_headers(fixture);
    for placed in placed_speech(trajectory) {
        let speech_header = &headers[&placed.step.id];
        if let SpeechPlacement::Observed {
            relationship,
            step_id,
            via,
        } = &placed.speech.placement
        {
            let step = &headers[step_id];
            let mut chain = vec![speech_header];
            chain.extend(via.iter().map(|id| &headers[id]));
            chain.push(step);
            match relationship.as_str() {
                "generated_by" => {
                    for pair in chain.windows(2) {
                        assert!(has_link(pair[0], "generated_by", pair[1]));
                    }
                }
                "input_context" => assert!(has_link(step, "input_context", speech_header)),
                "turn.reply_to" => {}
                other => panic!("Unexpected relationship {other}"),
            }
        }
        for other in &placed.speech.context_step_ids {
            assert!(has_link(&headers[other], "input_context", speech_header));
        }
    }
}

fn voice_fixture_as_import(fixture: &str) -> ImportFixture {
    let mut fixture: Value = serde_json::from_str(fixture).unwrap();
    fixture["turns"] = json!([]);
    serde_json::from_value(fixture).unwrap()
}

#[test]
fn voice_speech_is_placed_once_in_the_realtime_model_turns() {
    let trajectories = voice_fixture_trajectories(LIVEKIT_VOICE);
    let trajectory = &trajectories[0];
    let placed = placed_speech(trajectory);
    let ids: HashSet<_> = placed
        .iter()
        .map(|placed| placed.step.id.as_str())
        .collect();
    assert_eq!(placed.len(), 7);
    assert_eq!(ids.len(), 7);
    assert_eq!(trajectory.turns.len(), 2);

    let texts: Vec<_> = placed
        .iter()
        .map(|placed| {
            (
                placed.speech.speaker,
                placed.speech.text.as_deref().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        texts,
        vec![
            (SpeechSpeaker::Agent, "Hi, how can I help?"),
            (SpeechSpeaker::User, " Hi, can you tell me where a"),
            (SpeechSpeaker::User, " demo 1042 is"),
            (SpeechSpeaker::Agent, " Sure, I'll check that."),
            (
                SpeechSpeaker::Agent,
                " Order 1042 is in transit and expected to arrive Friday."
            ),
            (SpeechSpeaker::User, " Alright, thank you"),
            (SpeechSpeaker::Agent, " You're welcome."),
        ]
    );
    let headers = fixture_headers(LIVEKIT_VOICE);
    for placed in &placed {
        let voice = headers[&placed.step.id].voice.as_ref().unwrap();
        assert_eq!(placed.speech.selections, voice.selections);
        let channel = match placed.speech.speaker {
            SpeechSpeaker::User => 0,
            SpeechSpeaker::Agent => {
                assert!(placed.speech.selections.len() >= 3);
                1
            }
        };
        assert!(placed
            .speech
            .selections
            .iter()
            .all(|selection| selection.channel_index == Some(channel)
                && selection.recording_id == "call-0000"));
    }
    assert_speech_matches_links(LIVEKIT_VOICE, trajectory);

    let greeting = speech_by_id(&placed, "68e031159f3feede");
    assert_eq!(greeting.turn.request_id, "f34aaa489e63f67e");
    assert_eq!(
        observed_step(greeting.speech),
        Some(("generated_by", "f34aaa489e63f67e"))
    );

    let first = speech_by_id(&placed, "8d9f25e48b7888cd");
    assert_eq!(
        observed_step(first.speech),
        Some(("input_context", "d855d11a6e8ee505"))
    );
    assert_eq!(
        first.speech.context_step_ids,
        vec!["cf4ad22a4fce2f17", "fb1ff83447ce40a4"]
    );

    let order_number = speech_by_id(&placed, "8bb24c01384ebf11");
    assert_eq!(
        observed_step(order_number.speech),
        Some(("input_context", "cf4ad22a4fce2f17"))
    );
    assert_eq!(
        order_number.speech.context_step_ids,
        vec!["fb1ff83447ce40a4"]
    );
    for placed in &placed {
        assert!(!placed
            .speech
            .context_step_ids
            .iter()
            .any(|id| id == "d855d11a6e8ee505" && placed.step.id == "8bb24c01384ebf11"));
    }

    let thanks = speech_by_id(&placed, "2cb6e251a0e3e227");
    assert_eq!(
        thanks.speech.placement,
        SpeechPlacement::Inferred {
            basis: "time_within_turn".into()
        }
    );

    let check = speech_by_id(&placed, "62acd8e8ee191cbd");
    assert_eq!(
        observed_step(check.speech),
        Some(("generated_by", "d855d11a6e8ee505"))
    );
    let welcome = speech_by_id(&placed, "f5e0b9990bdd4e42");
    assert_eq!(
        observed_step(welcome.speech),
        Some(("generated_by", "fb1ff83447ce40a4"))
    );
    assert!(trajectory
        .turns
        .iter()
        .all(|turn| turn.interrupted.is_none()));
    for placed in &placed {
        if let Some((_, step)) = observed_step(placed.speech) {
            assert!(!ids.contains(step), "Speech placed by other speech");
        }
    }

    let order: Vec<_> = trajectory.turns[1]
        .work
        .iter()
        .map(|step| step.id.as_str())
        .collect();
    assert_eq!(
        order,
        vec![
            "8d9f25e48b7888cd",
            "8bb24c01384ebf11",
            "d855d11a6e8ee505",
            "62acd8e8ee191cbd",
            "46685b2fb3905fc6",
            "cf4ad22a4fce2f17",
            "3a80360be3632430",
            "2cb6e251a0e3e227",
            "f5e0b9990bdd4e42",
        ]
    );
    assert_eq!(
        trajectory.turns[1].response_id.as_deref(),
        Some("fb1ff83447ce40a4")
    );
}

#[test]
fn voice_speech_keeps_model_turns_unchanged() {
    let with_speech = voice_fixture_trajectories(LIVEKIT_VOICE);
    let mut fixture: Value = serde_json::from_str(LIVEKIT_VOICE).unwrap();
    fixture["spans"].as_array_mut().unwrap().retain(|span| {
        !matches!(
            span["span_attributes"]["name"].as_str(),
            Some("user_speech" | "agent_speech")
        )
    });
    let without_speech = voice_fixture_trajectories(&fixture.to_string());
    let mut stripped = serde_json::to_value(&with_speech).unwrap();
    for turn in stripped[0]["turns"].as_array_mut().unwrap() {
        turn["work"]
            .as_array_mut()
            .unwrap()
            .retain(|step| step["work"]["type"] != "speech");
    }
    assert_eq!(
        stripped[0]["turns"],
        serde_json::to_value(&without_speech).unwrap()[0]["turns"]
    );
}

#[test]
fn voice_speech_in_cascade_follows_tts_and_keeps_played_text() {
    let trajectories = voice_fixture_trajectories(CASCADE_VOICE);
    let trajectory = &trajectories[0];
    let placed = placed_speech(trajectory);
    assert_eq!(placed.len(), 5);
    assert_speech_matches_links(CASCADE_VOICE, trajectory);
    assert!(trajectory
        .turns
        .iter()
        .all(|turn| turn.interrupted.is_none()));

    let played = speech_by_id(&placed, "row-A2");
    assert_eq!(played.speech.text.as_deref(), Some("Yes, I can change it."));
    assert_eq!(
        played.speech.placement,
        SpeechPlacement::Observed {
            relationship: "generated_by".into(),
            step_id: "row-M3".into(),
            via: vec!["row-TT2".into()],
        }
    );
    assert_eq!(played.speech.framework_turn_id.as_deref(), Some("AT2"));
    let generated = serde_json::to_value(&played.turn.response).unwrap();
    assert_eq!(
        generated["response"][0]["text"],
        "Yes, I can change it. What is the new address?"
    );

    let first_answer = speech_by_id(&placed, "row-A1");
    assert_eq!(first_answer.turn.request_id, "row-M1");
    assert_eq!(
        observed_step(first_answer.speech),
        Some(("generated_by", "row-M2"))
    );
    assert_eq!(first_answer.speech.context_step_ids, vec!["row-M3"]);

    let question = speech_by_id(&placed, "row-U2");
    assert_eq!(question.turn.request_id, "row-M3");
    assert_eq!(question.speech.framework_turn_id.as_deref(), Some("UT2"));

    let overlap = speech_by_id(&placed, "row-U3");
    assert_eq!(overlap.turn.request_id, "row-M3");
    assert_eq!(
        overlap.speech.placement,
        SpeechPlacement::Inferred {
            basis: "time_after_turn".into()
        }
    );
    assert_eq!(overlap.speech.framework_turn_id.as_deref(), Some("UT3"));
}

#[test]
fn voice_user_speech_falls_back_to_framework_reply_to() {
    let mut fixture: Value = serde_json::from_str(CASCADE_VOICE).unwrap();
    for span in fixture["spans"].as_array_mut().unwrap() {
        if span["span_attributes"]["type"] == "llm" {
            span["metadata"]["links"] = json!([]);
        }
    }
    let trajectories = voice_fixture_trajectories(&fixture.to_string());
    let placed = placed_speech(&trajectories[0]);
    let question = speech_by_id(&placed, "row-U2");
    assert_eq!(
        observed_step(question.speech),
        Some(("turn.reply_to", "row-M3"))
    );
    assert!(question.speech.context_step_ids.is_empty());
    let first = speech_by_id(&placed, "row-U1");
    assert_eq!(
        observed_step(first.speech),
        Some(("turn.reply_to", "row-M1"))
    );
    assert!(matches!(
        speech_by_id(&placed, "row-U3").speech.placement,
        SpeechPlacement::Inferred { .. }
    ));
}

#[test]
fn voice_speech_edge_cases_are_placed_without_panicking() {
    let trajectories = voice_fixture_trajectories(EDGE_VOICE);
    let trajectory = &trajectories[0];
    let placed = placed_speech(trajectory);
    assert_eq!(placed.len(), 6);
    assert_speech_matches_links(EDGE_VOICE, trajectory);
    assert!(trajectory
        .turns
        .iter()
        .all(|turn| turn.interrupted.is_none()));

    let silent = speech_by_id(&placed, "row-V1");
    assert_eq!(silent.speech.text, None);
    assert_eq!(
        silent.speech.placement,
        SpeechPlacement::Inferred {
            basis: "time_before_first_turn".into()
        }
    );
    let unrecorded = speech_by_id(&placed, "row-V3");
    assert!(unrecorded.speech.selections.is_empty());
    assert_eq!(
        observed_step(unrecorded.speech),
        Some(("generated_by", "row-L1"))
    );
    let segmented = speech_by_id(&placed, "row-V4");
    let segments: Vec<_> = segmented
        .speech
        .selections
        .iter()
        .map(|selection| (selection.recording_id.as_str(), selection.start_offset_ms))
        .collect();
    assert_eq!(segments, vec![("seg-0", 5500.0), ("seg-1", 0.0)]);
    for id in ["row-V2", "row-V5", "row-V6"] {
        assert!(matches!(
            speech_by_id(&placed, id).speech.placement,
            SpeechPlacement::Inferred { .. }
        ));
    }
    assert_eq!(
        serde_json::to_value(&trajectory.metadata["import_warnings"]).unwrap(),
        json!([{"span_id": "row-L1", "message": "Unresolved input_context link to span MISSING-SPAN"}])
    );
    assert!(trajectory
        .recordings
        .as_ref()
        .unwrap()
        .iter()
        .any(|recording| recording.state.as_deref() == Some("pending")));
}

#[test]
fn voice_speech_does_not_depend_on_load_order() {
    for fixture in [LIVEKIT_VOICE, CASCADE_VOICE, EDGE_VOICE] {
        let expected =
            serde_json::to_value(voice_trajectories_in_order(fixture, 1, false)).unwrap();
        for (limit, reverse) in [(16, false), (1, true), (16, true), (usize::MAX, true)] {
            assert_eq!(
                serde_json::to_value(voice_trajectories_in_order(fixture, limit, reverse)).unwrap(),
                expected
            );
        }
        let imported = voice_fixture_as_import(fixture);
        for (batch_size, bodies) in [(1, false), (16, false), (1, true), (16, true)] {
            assert_eq!(
                serde_json::to_value(run_fixture(&imported, batch_size, bodies, &[])).unwrap(),
                expected
            );
        }
        let reversed: Vec<_> = (0..imported.spans.len() as u64).rev().collect();
        assert_eq!(
            serde_json::to_value(run_fixture(&imported, 3, false, &reversed)).unwrap(),
            expected
        );
    }
}

#[test]
fn voice_speech_with_failed_payload_keeps_its_place() {
    let fixture: Value = serde_json::from_str(LIVEKIT_VOICE).unwrap();
    let spans: Vec<Span> = serde_json::from_value(fixture["spans"].clone()).unwrap();
    let mut stream = stream_from_sources(spans.clone()).unwrap();
    let mut collector = TrajectoryCollector::default();
    while let Some(id) = stream.pending_ids(1).first().cloned() {
        let events = if id == "8bb24c01384ebf11" {
            stream.fail(&id, "payload unavailable".into()).unwrap()
        } else {
            let source = spans
                .iter()
                .find(|span| span.other["id"].as_str() == Some(id.as_str()))
                .cloned()
                .unwrap();
            stream.push(import_span(source).unwrap()).unwrap()
        };
        collect(&mut collector, events);
    }
    collect(&mut collector, stream.finish().unwrap());
    let trajectories = collector.snapshot().unwrap();
    let placed = placed_speech(&trajectories[0]);
    assert_eq!(placed.len(), 7);
    let failed = speech_by_id(&placed, "8bb24c01384ebf11");
    assert_eq!(failed.speech.text, None);
    assert_eq!(
        observed_step(failed.speech),
        Some(("input_context", "cf4ad22a4fce2f17"))
    );
}
