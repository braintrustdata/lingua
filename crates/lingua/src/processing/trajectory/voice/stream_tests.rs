use super::super::{TrajectoryCollector, TrajectoryStream};
use super::tests::{cascade, hydrate, prepared, row_id, span_json, spans, Load};
use super::*;
use crate::processing::import::{import_span, Span};
use crate::serde_json::{self as json, json, Value};
use serde::Deserialize;

fn snapshot(collector: &TrajectoryCollector) -> Value {
    json::to_value(collector.snapshot().unwrap()).unwrap()
}

fn expected_voice(spans: &[Span]) -> Value {
    json::to_value(&build(&prepared(spans)).remove("s-session").unwrap().voice).unwrap()
}

#[test]
fn streamed_voice_matches_the_builder_for_any_load_order() {
    let spans = cascade();
    let (first, _) = hydrate(&spans, 0, |_| Load::Full);
    let first = snapshot(&first);
    assert_eq!(first[0]["voice"], expected_voice(&spans));
    for seed in 1..12 {
        let (collector, _) = hydrate(&spans, seed, |_| Load::Full);
        assert!(collector.is_complete());
        assert_eq!(
            snapshot(&collector),
            first,
            "snapshot depends on load order (seed {seed})"
        );
    }
}

#[test]
fn streamed_voice_preserves_trace_semantics() {
    let (collector, _) = hydrate(&cascade(), 5, |_| Load::Full);
    let trajectory = collector.snapshot().unwrap().remove(0);
    let voice = trajectory.voice.unwrap();
    let text = |id: &str| {
        voice
            .utterances
            .iter()
            .find(|utterance| utterance.id == id)
            .unwrap()
            .text
            .clone()
    };
    assert_eq!(
        text("r-u1-a").as_deref(),
        Some("Draw how a voice agent works.")
    );
    assert_eq!(text("r-u1-b"), None);
    assert_eq!(text("r-u4-a"), None);
    assert_eq!(
        text("r-a3-speech").as_deref(),
        Some("You're welcome! Let me know if you need anything")
    );
    let a2 = voice.turns.iter().find(|turn| turn.id == "r-a2").unwrap();
    assert!(a2.utterance_ids.is_empty());
    assert_eq!(a2.step_ids, ["r-llm3", "r-tool2", "r-llm4"]);
    assert!(voice.recordings.is_empty());
    assert!(!trajectory.turns.is_empty());
    assert!(trajectory.metadata.get("import_warnings").is_none());
}

#[test]
fn voice_is_visible_before_payloads_arrive() {
    let spans = cascade();
    let headers: Vec<_> = spans
        .iter()
        .map(|span| import_span(span.clone()).unwrap().header_only())
        .collect();
    let mut stream = TrajectoryStream::new(headers, false).unwrap();
    let mut collector = TrajectoryCollector::default();
    let first = stream.pending_ids(1).remove(0);
    let source = spans.iter().find(|span| row_id(span) == first).unwrap();
    for event in stream.push(import_span(source.clone()).unwrap()).unwrap() {
        collector.push(event).unwrap();
    }
    assert!(!collector.is_complete());
    let voice = collector.snapshot().unwrap().remove(0).voice.unwrap();
    assert_eq!(voice.utterances.len(), 11);
    assert_eq!(voice.turns.len(), 7);
    assert!(voice
        .utterances
        .iter()
        .all(|utterance| utterance.text.is_none()));
}

#[test]
fn omitted_and_failed_speech_payloads_finish_without_text() {
    let spans = cascade();
    let (collector, _) = hydrate(&spans, 9, |id| match id {
        "r-u1-a" => Load::Fail,
        "r-a1-speech" => Load::HeaderOnly,
        _ => Load::Full,
    });
    assert!(collector.is_complete());
    let trajectory = collector.snapshot().unwrap().remove(0);
    let voice = trajectory.voice.unwrap();
    let utterance = |id: &str| {
        voice
            .utterances
            .iter()
            .find(|utterance| utterance.id == id)
            .unwrap()
    };
    assert_eq!(utterance("r-u1-a").text, None);
    assert_eq!(utterance("r-a1-speech").text, None);
    assert_eq!(utterance("r-a1-speech").interrupted, Some(true));
    assert_eq!(
        utterance("r-a3-speech").text.as_deref(),
        Some("You're welcome! Let me know if you need anything")
    );
    assert_eq!(
        trajectory.metadata["import_failures"],
        json!([{"span_id": "r-u1-a", "message": "Payload unavailable"}])
    );
}

#[test]
fn unsupported_voice_operations_are_reported_in_the_snapshot() {
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json("stt", "stt", "task", Some("session"), 1.0, json!({})),
    ]);
    let (collector, _) = hydrate(&spans, 0, |_| Load::Full);
    let trajectory = collector.snapshot().unwrap().remove(0);
    assert!(trajectory.voice.unwrap().turns.is_empty());
    assert!(trajectory.turns.is_empty());
    assert_eq!(
        trajectory.metadata["import_warnings"],
        json!([{"span_id": "r-stt", "message": "Unsupported voice operation stt: no recorded native voice turn owns it"}])
    );
}

#[test]
fn non_voice_traces_have_no_voice() {
    let fixture: Value =
        json::from_str(include_str!("../fixtures/task-parent-with-tool.json")).unwrap();
    let spans = Vec::<Span>::deserialize(&fixture["spans"]).unwrap();
    let (collector, _) = hydrate(&spans, 0, |_| Load::Full);
    assert!(snapshot(&collector)[0].get("voice").is_none());
}
