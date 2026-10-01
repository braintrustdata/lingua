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
