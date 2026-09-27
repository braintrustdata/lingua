use super::*;
use serde_json::json;

fn span(id: &str, start: i64, input: Value, output: Value) -> SourceSpan {
    SourceSpan::parse(json!({
        "id": id, "span_id": id, "root_span_id": "root", "span_parents": ["root"],
        "span_attributes": {"type": "llm"}, "metrics": {"start": start, "end": start + 1},
        "input": input, "output": output,
    }))
    .unwrap()
}

fn collect(collector: &mut TrajectoryCollector, events: Vec<TrajectoryEvent>) {
    for event in events {
        let wire = serde_json::to_value(event).unwrap();
        collector
            .push(serde_json::from_value(wire).unwrap())
            .unwrap();
    }
}

#[derive(Deserialize)]
struct ImportFixture {
    spans: Vec<SourceSpan>,
    turns: Vec<ExpectedTurn>,
    #[serde(default)]
    worker_responses: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedTurn {
    request_id: String,
    response_id: Option<String>,
    response_text: String,
    work_ids: Vec<String>,
    user_texts: Vec<String>,
}

fn check_import_fixture(fixture: &str) {
    let fixture: ImportFixture = serde_json::from_str(fixture).unwrap();
    for batch_size in [1, 16] {
        let mut stream = TrajectoryStream::new(fixture.spans.clone(), false).unwrap();
        let mut collector = TrajectoryCollector::default();
        loop {
            let ids = stream.pending_ids(batch_size);
            if ids.is_empty() {
                break;
            }
            for id in ids.iter().rev() {
                let span = fixture.spans.iter().find(|span| &span.id == id).unwrap();
                collect(&mut collector, stream.push(span.clone()).unwrap());
            }
        }
        collect(&mut collector, stream.finish().unwrap());
        assert!(collector.is_complete());
        let trajectories = collector.snapshot().unwrap();
        assert_eq!(trajectories.len(), 1);
        assert!(!trajectories[0].metadata.contains_key("import_failures"));
        let turns: Vec<_> = trajectories[0]
            .turns
            .iter()
            .map(|turn| ExpectedTurn {
                request_id: turn.request_id.clone(),
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
}

macro_rules! import_fixture {
    ($test:ident, $file:literal) => {
        #[test]
        fn $test() {
            check_import_fixture(include_str!($file));
        }
    };
}

import_fixture!(responses_tool_cycle, "fixtures/responses-tool-cycle.json");
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
import_fixture!(
    opaque_history_tool_continuation,
    "fixtures/opaque-history-tool-continuation.json"
);

#[test]
fn reviewer_calls_do_not_split_turns_or_replace_the_final_response() {
    let sources: Vec<SourceSpan> =
        serde_json::from_str(include_str!("fixtures/reviewer-continuation.json")).unwrap();
    let mut stream = TrajectoryStream::new(sources.clone(), false).unwrap();
    let mut collector = TrajectoryCollector::default();
    while let Some(id) = stream.pending_ids(1).first() {
        let source = sources.iter().find(|source| &source.id == id).unwrap();
        collect(&mut collector, stream.push(source.clone()).unwrap());
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
    let expected = legacy::assemble(
        &sources
            .iter()
            .cloned()
            .map(TrajectorySpan::new)
            .collect::<Result<Vec<_>>>()
            .unwrap(),
        &[],
        false,
    )
    .unwrap();
    let mut stream = TrajectoryStream::new(sources.clone(), false).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(&mut collector, stream.push(sources[0].clone()).unwrap());
    let partial = collector.snapshot().unwrap();
    assert_eq!(partial[0].turns.len(), 1);
    assert_eq!(partial[0].turns[0].request_id, "first");
    assert!(partial[0].turns[0].response.is_none());
    assert!(!collector.is_complete());
    assert!(stream.finish().is_err());
    collect(&mut collector, stream.push(sources[1].clone()).unwrap());
    assert_eq!(
        collector.snapshot().unwrap()[0].turns[0]
            .response_id
            .as_deref(),
        Some("first")
    );
    collect(&mut collector, stream.finish().unwrap());
    assert!(collector.is_complete());
    assert_eq!(
        serde_json::to_value(collector.snapshot().unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
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
    let mut stream = TrajectoryStream::new(sources.clone(), false).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(&mut collector, stream.push(sources[1].clone()).unwrap());
    assert!(collector.snapshot().unwrap()[0].turns.is_empty());
    collect(&mut collector, stream.push(sources[0].clone()).unwrap());
    assert_eq!(collector.snapshot().unwrap()[0].turns.len(), 2);
    assert!(stream.push(sources[0].clone()).is_err());
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
    broken.id = "broken".to_string();
    broken.metrics.start = None;
    let mut stream = TrajectoryStream::new(vec![healthy.clone(), broken], false).unwrap();
    let mut collector = TrajectoryCollector::default();
    collect(&mut collector, stream.push(healthy).unwrap());
    collect(&mut collector, stream.finish().unwrap());
    let result = collector.snapshot().unwrap();
    assert_eq!(result[0].turns.len(), 1);
    assert_eq!(
        result[0].metadata["import_failures"][0]["span_id"],
        "broken"
    );
}

#[test]
fn interruption_in_later_request_marks_the_previous_turn() {
    let question = json!({"role": "user", "content": "Again"});
    let mut first = span("first", 1, json!([question]), json!([]));
    first.metadata = Some(crate::serde_json::json!({"turn_id": "one"}));
    let mut tool = span("tool", 3, json!([]), json!([]));
    tool.span_attributes.kind = Some("tool".to_string());
    tool.metadata = Some(crate::serde_json::json!({"turn_id": "two"}));
    let mut request = span(
        "request",
        5,
        json!([{"role": "system", "content": "<turn_aborted>Interrupted</turn_aborted>"}, question]),
        json!([]),
    );
    request.metadata = Some(crate::serde_json::json!({"turn_id": "two"}));
    let spans = vec![first, tool, request]
        .into_iter()
        .map(TrajectorySpan::new)
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
        .map(TrajectorySpan::new)
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
        .map(TrajectorySpan::new)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 2);
    assert_eq!(
        serde_json::to_value(&result[0].turns[1].request).unwrap(),
        json!([question])
    );
}

#[test]
fn resumed_call_does_not_pull_later_requests_before_their_work() {
    let first = json!({"role": "user", "content": "First task"});
    let second = json!({"role": "user", "content": "Second task"});
    let third = json!({"role": "user", "content": "Third task"});
    let reply = json!([{"role": "assistant", "content": "Done"}]);
    let mut resumed = span("resumed", 3, json!([first, second, third]), json!([]));
    resumed.metrics.end = Some(10.0);
    let sources = vec![
        span("first", 1, json!([first]), reply.clone()),
        resumed,
        span("second", 5, json!([second]), reply.clone()),
        span("third", 7, json!([third]), reply),
    ];
    let spans = sources
        .into_iter()
        .map(TrajectorySpan::new)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 3);
    assert_eq!(result[0].turns[1].request_id, "second");
    assert_eq!(result[0].turns[2].request_id, "third");
    assert_eq!(result[0].turns[2].work.last().unwrap().id, "resumed");
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
        .map(TrajectorySpan::new)
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
    first.metadata = Some(crate::serde_json::json!({"turn_id": "one"}));
    let mut resumed = span("resumed", 3, json!([question, replay]), json!([]));
    resumed.metadata = first.metadata.clone();
    resumed.metrics.end = Some(10.0);
    let mut next = span("second", 5, json!([second]), json!([]));
    next.metadata = Some(crate::serde_json::json!({"turn_id": "two"}));
    let spans = vec![first, resumed, next]
        .into_iter()
        .map(TrajectorySpan::new)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let result = assemble(&spans, &[], false).unwrap();
    assert_eq!(result[0].turns.len(), 2);
    assert_eq!(result[0].turns[1].work.len(), 2);
    assert_eq!(
        serde_json::to_value(&result[0].turns[1].request).unwrap(),
        json!([second])
    );
}
