use super::*;
use proptest::prelude::*;

#[test]
fn request_attachment_and_turn_boundaries() {
    use BoundaryAction::{AttachRequest, Continue, NewTurn};
    use ConversationObservation::{Compaction, Current};
    use RequestEvidence::{Absent, New, Replayed};

    let states = [
        TurnState::default(),
        TurnState {
            id: Some("current".into()),
            ..Default::default()
        },
        TurnState {
            id: Some("current".into()),
            request_found: true,
            ..Default::default()
        },
    ];
    for (conversation, expected) in [
        (Current(Absent), [NewTurn, Continue, Continue]),
        (Current(Replayed), [NewTurn, AttachRequest, Continue]),
        (Current(New), [NewTurn, NewTurn, NewTurn]),
        (
            Compaction { has_request: false },
            [NewTurn, Continue, Continue],
        ),
        (
            Compaction { has_request: true },
            [NewTurn, AttachRequest, Continue],
        ),
    ] {
        let observation = Observation {
            conversation,
            initial_request: false,
            interrupts: false,
            history: Vec::new(),
            request_filter: None,
        };
        for (state, expected) in states.iter().zip(expected) {
            assert_eq!(decide(state, &observation), expected, "{conversation:?}");
        }
    }
}

proptest! {
    #[test]
    fn replay_marks_only_new_occurrences(
        history in proptest::collection::vec(0u64..8, 0..30),
        new in proptest::collection::vec(0u64..8, 0..15),
    ) {
        let input: Vec<_> = history.iter().chain(&new).copied().collect();
        let (merged, fresh) = merge_history(&history, &input, &[]);
        prop_assert_eq!(merged, input);
        prop_assert!(fresh[..history.len()].iter().all(|value| !value));
        prop_assert!(fresh[history.len()..].iter().all(|value| *value));
    }

    #[test]
    fn omitted_history_prefix_does_not_create_new_requests(
        history in proptest::collection::vec(0u64..8, 1..30),
        offset in any::<usize>(),
    ) {
        let (_, fresh) = merge_history(&history, &history[offset % history.len()..], &[99]);
        prop_assert!(fresh.iter().all(|value| !value));
    }

    #[test]
    fn repeated_messages_are_distinct_occurrences(old in 0usize..20, extra in 1usize..20) {
        let (_, fresh) = merge_history(&vec![1; old], &vec![1; old + extra], &[]);
        prop_assert_eq!(fresh.iter().filter(|value| **value).count(), extra);
    }
}

#[test]
fn emitted_payloads_and_per_span_history_are_released() {
    use crate::processing::import::{import_span, Span};
    use serde_json::json;

    for kind in ["llm", "task"] {
        let mut history = Vec::new();
        let mut sources = Vec::new();
        for index in 0..64 {
            history.push(json!({ "role": "user", "content": format!("Request {index}") }));
            let reply = json!({ "role": "assistant", "content": format!("Answer {index}") });
            let source: Span = serde_json::from_value(json!({
                "id": index.to_string(), "root_span_id": format!("root-{}", index / 8),
                "span_parents": [format!("root-{}", index / 8)],
                "span_attributes": { "type": kind },
                "metrics": { "start": index * 2, "end": index * 2 + 1 },
                "metadata": { "turn_id": index.to_string() },
                "error": (index % 8 == 0).then(|| json!({ "detail": "logged error" })),
                "input": history, "output": [reply],
            }))
            .unwrap();
            sources.push(source);
            history.push(reply);
        }
        if kind == "task" {
            for index in 0..8 {
                sources.push(
                    serde_json::from_value(json!({
                        "id": format!("root-{index}"), "root_span_id": format!("root-{index}"),
                        "span_attributes": { "type": "task" },
                        "metrics": { "start": index * 16 - 1, "end": index * 16 + 16 },
                        "input": [{ "role": "user", "content": "Wrapper request" }],
                        "output": [{ "role": "assistant", "content": "Wrapper answer" }],
                    }))
                    .unwrap(),
                );
            }
        }
        let headers = sources
            .iter()
            .cloned()
            .map(|mut source| {
                source.input = None;
                source.output = None;
                source.other.remove("error");
                import_span(source).unwrap()
            })
            .collect();
        let mut stream = TrajectoryStream::new(headers, false).unwrap();
        let mut fetched = HashSet::new();
        while let Some(id) = stream.pending_ids(1).first() {
            assert!(fetched.insert(id.clone()));
            assert!(!id.starts_with("root-"));
            let source = sources
                .iter()
                .find(|source| source.other["id"].as_str() == Some(id))
                .unwrap();
            let events = stream.push(import_span(source.clone()).unwrap()).unwrap();
            if fetched.len() == 1 {
                assert!(events
                    .iter()
                    .any(|event| matches!(event, TrajectoryEvent::Turn { .. })));
            }
            assert!(stream.spans.iter().all(|span| span.input.is_empty()
                && span.tool_result.is_none()
                && span.source.error.is_none()));
            assert!(
                stream
                    .spans
                    .iter()
                    .map(|span| span.input_keys.len())
                    .sum::<usize>()
                    <= 1
            );
            assert_eq!(
                stream
                    .spans
                    .iter()
                    .map(|span| span.output.len())
                    .sum::<usize>(),
                stream
                    .states
                    .values()
                    .filter(|state| state.candidate.is_some())
                    .count()
            );
        }
        assert_eq!(fetched.len(), 64);
        stream.finish().unwrap();
        assert!(stream
            .spans
            .iter()
            .all(|span| span.output.is_empty() && span.input_keys.is_empty()));
    }
}
