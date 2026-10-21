use super::*;
use crate::processing::import::{import_span, Span};
use crate::serde_json::{self as json, json, Value};
use serde::Deserialize;

fn fixture_spans(fixture: &str) -> Vec<Span> {
    let fixture: Value = json::from_str(fixture).unwrap();
    Vec::<Span>::deserialize(&fixture["spans"]).unwrap()
}

pub(super) fn prepared(spans: &[Span]) -> Vec<PreparedSpan> {
    spans
        .iter()
        .map(|span| PreparedSpan::new(import_span(span.clone()).unwrap()).unwrap())
        .collect()
}

pub(super) fn cascade() -> Vec<Span> {
    fixture_spans(include_str!("../fixtures/voice-cascade.json"))
}

pub(super) fn spans(values: Vec<Value>) -> Vec<Span> {
    values
        .into_iter()
        .map(|span| Span::deserialize(span).unwrap())
        .collect()
}

fn build_one(spans: &[Span]) -> VoiceBuild {
    let mut builds = build(&prepared(spans));
    assert_eq!(builds.len(), 1);
    builds.remove("s-session").unwrap()
}

fn turn<'a>(voice: &'a Voice, id: &str) -> &'a VoiceTurn {
    voice.turns.iter().find(|turn| turn.id == id).unwrap()
}

fn utterance<'a>(voice: &'a Voice, id: &str) -> &'a Utterance {
    voice
        .utterances
        .iter()
        .find(|utterance| utterance.id == id)
        .unwrap()
}

pub(super) fn span_json(
    id: &str,
    name: &str,
    kind: &str,
    parent: Option<&str>,
    start: f64,
    metadata: Value,
) -> Value {
    let mut span = json!({
        "id": format!("r-{id}"),
        "span_id": format!("s-{id}"),
        "root_span_id": "s-session",
        "span_attributes": {"type": kind, "name": name},
        "metrics": {"start": start, "end": start + 1.0},
        "metadata": metadata,
    });
    if let Some(parent) = parent {
        span["span_parents"] = json!([format!("s-{parent}")]);
    }
    span
}

#[test]
fn native_turns_own_speech_and_steps() {
    let build = build_one(&cascade());
    let voice = &build.voice;
    assert_eq!(
        voice
            .turns
            .iter()
            .map(|turn| turn.id.as_str())
            .collect::<Vec<_>>(),
        ["r-u1", "r-a1", "r-u2", "r-a2", "r-u3", "r-a3", "r-u4"]
    );
    let u1 = turn(voice, "r-u1");
    assert_eq!(u1.speaker, "user");
    assert_eq!(u1.utterance_ids, ["r-u1-a", "r-u1-b", "r-u1-c"]);
    assert_eq!(u1.step_ids, ["r-stt1a", "r-stt1b"]);
    let a1 = turn(voice, "r-a1");
    assert_eq!(a1.speaker, "agent");
    assert_eq!(a1.interrupted, Some(true));
    assert_eq!(a1.utterance_ids, ["r-a1-speech"]);
    assert_eq!(a1.step_ids, ["r-llm1", "r-tool1", "r-llm2", "r-tts1"]);
    assert!(build.warnings.is_empty(), "{:?}", build.warnings);
}

#[test]
fn shared_stt_text_is_not_copied_to_speech() {
    let voice = build_one(&cascade()).voice;
    assert_eq!(
        utterance(&voice, "r-u1-a").text.as_deref(),
        Some("Draw how a voice agent works.")
    );
    assert_eq!(utterance(&voice, "r-u1-b").text, None);
    assert_eq!(utterance(&voice, "r-u1-c").text, None);
    assert_eq!(utterance(&voice, "r-u2-b").text, None);
    assert_eq!(turn(&voice, "r-u2").step_ids, ["r-stt2"]);
}

#[test]
fn interrupted_speech_keeps_its_own_text() {
    let voice = build_one(&cascade()).voice;
    let speech = utterance(&voice, "r-a1-speech");
    assert_eq!(speech.speaker, "agent");
    assert_eq!(speech.interrupted, Some(true));
    assert_eq!(
        speech.text.as_deref(),
        Some("The diagram shows: 1. User speaks. 2. Recognized")
    );
    let a2 = turn(&voice, "r-a2");
    assert_eq!(a2.interrupted, Some(true));
    assert!(a2.utterance_ids.is_empty());
    assert_eq!(a2.step_ids, ["r-llm3", "r-tool2", "r-llm4"]);
    let u4 = turn(&voice, "r-u4");
    assert!(u4.step_ids.is_empty());
    assert_eq!(u4.utterance_ids, ["r-u4-a", "r-u4-b"]);
    assert!(u4
        .utterance_ids
        .iter()
        .all(|id| utterance(&voice, id).text.is_none()));
}

#[test]
fn header_only_spans_keep_records_without_text() {
    let headers: Vec<_> = cascade()
        .into_iter()
        .map(|mut span| {
            span.input = None;
            span.output = None;
            span
        })
        .collect();
    let voice = build_one(&headers).voice;
    assert_eq!(voice.utterances.len(), 11);
    assert!(voice
        .utterances
        .iter()
        .all(|utterance| utterance.text.is_none()));
    assert_eq!(turn(&voice, "r-u1").step_ids.len(), 2);
}

#[test]
fn discarded_synthesis_stays_in_its_turn_without_speech() {
    let mut tts = span_json(
        "tts",
        "tts",
        "task",
        Some("a1"),
        2.0,
        json!({"turn.id": "s-a1"}),
    );
    tts["input"] = json!("Discarded reply");
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "a1",
            "agent_turn",
            "task",
            Some("session"),
            1.0,
            json!({"turn.id": "s-a1"}),
        ),
        span_json(
            "llm",
            "llm_response",
            "llm",
            Some("a1"),
            1.0,
            json!({"turn.id": "s-a1"}),
        ),
        tts,
    ]);
    let voice = build_one(&spans).voice;
    assert!(voice.utterances.is_empty());
    let a1 = turn(&voice, "r-a1");
    assert!(a1.utterance_ids.is_empty());
    assert_eq!(a1.step_ids, ["r-llm", "r-tts"]);
}

#[test]
fn unowned_and_conflicting_spans_are_reported_not_assigned() {
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "u1",
            "user_turn",
            "task",
            Some("session"),
            1.0,
            json!({"turn.id": "s-u1"}),
        ),
        span_json(
            "u2",
            "user_turn",
            "task",
            Some("session"),
            3.0,
            json!({"turn.id": "s-u2"}),
        ),
        span_json("stt", "stt", "task", Some("session"), 1.5, json!({})),
        span_json(
            "speech",
            "user_speech",
            "task",
            Some("u1"),
            1.2,
            json!({"turn.id": "s-u2"}),
        ),
        span_json(
            "missing",
            "user_speech",
            "task",
            Some("session"),
            1.3,
            json!({"turn.id": "s-gone"}),
        ),
    ]);
    let build = build_one(&spans);
    assert!(build
        .voice
        .turns
        .iter()
        .all(|turn| turn.utterance_ids.is_empty() && turn.step_ids.is_empty()));
    assert_eq!(build.voice.utterances.len(), 2);
    let mut warnings: Vec<_> = build
        .warnings
        .iter()
        .map(|warning| (warning.span_id.as_str(), warning.message.as_str()))
        .collect();
    warnings.sort();
    assert_eq!(
        warnings,
        [
            (
                "r-missing",
                "Unresolved voice turn.id s-gone: no native voice turn has that span ID"
            ),
            (
                "r-speech",
                "Conflicting voice turn ownership: turn.id names r-u2 but the nearest native voice turn ancestor is r-u1"
            ),
            (
                "r-stt",
                "Unsupported voice operation stt: no recorded native voice turn owns it"
            ),
        ]
    );
}

#[test]
fn traces_without_voice_spans_have_no_voice() {
    let spans = fixture_spans(include_str!("../fixtures/task-parent-with-tool.json"));
    assert!(build(&prepared(&spans)).is_empty());
}

#[test]
fn speech_only_traces_do_not_become_model_turns() {
    let mut speech = span_json(
        "speech",
        "user_speech",
        "task",
        Some("u1"),
        1.0,
        json!({"turn.id": "s-u1"}),
    );
    speech["input"] = json!([{"role": "user", "content": "Hello"}]);
    let mut stt = span_json(
        "stt",
        "stt",
        "task",
        Some("u1"),
        1.0,
        json!({"turn.id": "s-u1"}),
    );
    stt["output"] = json!([{"role": "user", "content": "Hello"}]);
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "u1",
            "user_turn",
            "task",
            Some("session"),
            1.0,
            json!({"turn.id": "s-u1"}),
        ),
        speech,
        stt,
    ]);
    let trajectories = super::super::assemble(
        &spans
            .iter()
            .cloned()
            .map(|span| import_span(span).unwrap())
            .collect::<Vec<_>>(),
        &[],
        false,
    )
    .unwrap();
    assert_eq!(trajectories.len(), 1);
    assert!(trajectories[0].turns.is_empty());
    assert_eq!(
        build_one(&spans).voice.utterances[0].text.as_deref(),
        Some("Hello")
    );
}

#[test]
fn explicit_audio_selections_keep_recording_and_channel_offsets() {
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "speech",
            "user_speech",
            "task",
            Some("session"),
            1.0,
            json!({"audio.selections": [{
                "recording_span_id": "s-session",
                "recording_id": "call",
                "start_offset_ms": 750,
                "end_offset_ms": 1500.5,
                "channel_index": 0
            }]}),
        ),
    ]);
    let voice = build_one(&spans).voice;
    assert_eq!(
        voice.utterances[0].audio,
        [AudioSelection {
            row_id: "r-session".to_string(),
            recording_id: "call".to_string(),
            channel: Some(0),
            start_ms: 750.0,
            end_ms: 1500.5,
        }]
    );
    assert!(build_one(&cascade())
        .voice
        .utterances
        .iter()
        .all(|utterance| utterance.audio.is_empty()));
}

#[test]
fn audio_selections_without_a_recording_row_are_reported() {
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "speech",
            "user_speech",
            "task",
            Some("session"),
            1.0,
            json!({"audio.selections": [
                {"recording_id": "call", "start_offset_ms": 0, "end_offset_ms": 10},
                {"recording_span_id": "s-gone", "recording_id": "call", "start_offset_ms": 0, "end_offset_ms": 10}
            ]}),
        ),
    ]);
    let build = build_one(&spans);
    assert!(build.voice.utterances[0].audio.is_empty());
    assert_eq!(build.warnings.len(), 2);
    assert!(build.warnings.iter().all(|warning| {
        warning.message
        == "Unresolved audio selection for recording call: recording_span_id is not in this trace"
    }));
}
