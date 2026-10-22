use super::*;
use crate::processing::import::{import_span, Span};
use crate::serde_json::{self as json, json, Value};
use serde::Deserialize;

fn fixture_spans(fixture: &str) -> Vec<Span> {
    let fixture: Value = json::from_str(fixture).unwrap();
    Vec::<Span>::deserialize(&fixture["spans"]).unwrap()
}

pub(super) fn prepared(spans: &[Span]) -> Vec<PreparedSpan> {
    let mut spans: Vec<_> = spans
        .iter()
        .map(|span| import_span(span.clone()).unwrap())
        .collect();
    super::super::stream::adapt_voice_traces(&mut spans);
    spans
        .into_iter()
        .map(|span| PreparedSpan::new(span).unwrap())
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

fn utterance_ids<'a>(voice: &'a Voice, turn_id: &str) -> Vec<&'a str> {
    voice
        .utterances
        .iter()
        .filter(|utterance| utterance.turn_id.as_deref() == Some(turn_id))
        .map(|utterance| utterance.id.as_str())
        .collect()
}

fn step_ids<'a>(voice: &'a Voice, turn_id: &str) -> Vec<(&'a str, &'a str)> {
    voice
        .steps
        .iter()
        .filter(|step| step.turn_id.as_deref() == Some(turn_id))
        .map(|step| (step.id.as_str(), step.kind.as_str()))
        .collect()
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
fn speech_and_steps_name_their_turn() {
    let build = build_one(&cascade());
    let voice = &build.voice;
    assert_eq!(
        voice
            .turns
            .iter()
            .map(|turn| (turn.id.as_str(), turn.speaker.as_str()))
            .collect::<Vec<_>>(),
        [
            ("r-u1", "user"),
            ("r-a1", "agent"),
            ("r-u2", "user"),
            ("r-a2", "agent"),
            ("r-u3", "user"),
            ("r-a3", "agent"),
            ("r-u4", "user"),
        ]
    );
    assert_eq!(utterance_ids(voice, "r-u1"), ["r-u1-a", "r-u1-b", "r-u1-c"]);
    assert_eq!(
        step_ids(voice, "r-u1"),
        [("r-stt1a", "stt"), ("r-stt1b", "stt")]
    );
    assert_eq!(utterance_ids(voice, "r-a1"), ["r-a1-speech"]);
    assert_eq!(
        step_ids(voice, "r-a1"),
        [
            ("r-llm1", "llm"),
            ("r-tool1", "tool"),
            ("r-llm2", "llm"),
            ("r-tts1", "tts"),
        ]
    );
    assert_eq!(voice.utterances.len(), 11);
    assert_eq!(voice.steps.len(), 13);
    assert!(voice.steps.iter().all(|step| step.turn_id.is_some()));
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
    assert_eq!(step_ids(&voice, "r-u2"), [("r-stt2", "stt")]);
}

#[test]
fn interrupted_speech_keeps_its_own_text() {
    let voice = build_one(&cascade()).voice;
    assert_eq!(
        voice
            .utterances
            .iter()
            .filter(|utterance| utterance.interrupted == Some(true))
            .map(|utterance| utterance.id.as_str())
            .collect::<Vec<_>>(),
        ["r-a1-speech", "r-a3-speech"]
    );
    assert_eq!(
        utterance(&voice, "r-a1-speech").text.as_deref(),
        Some("The diagram shows: 1. User speaks. 2. Recognized")
    );
    assert!(utterance_ids(&voice, "r-a2").is_empty());
    assert_eq!(
        step_ids(&voice, "r-a2"),
        [("r-llm3", "llm"), ("r-tool2", "tool"), ("r-llm4", "llm")]
    );
    assert!(step_ids(&voice, "r-u4").is_empty());
    assert_eq!(utterance_ids(&voice, "r-u4"), ["r-u4-a", "r-u4-b"]);
    assert!(utterance_ids(&voice, "r-u4")
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
    assert_eq!(step_ids(&voice, "r-u1").len(), 2);
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
    assert_eq!(
        step_ids(&voice, "r-a1"),
        [("r-llm", "llm"), ("r-tts", "tts")]
    );
}

#[test]
fn unresolved_turn_ids_are_reported_not_assigned() {
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
        span_json("stt", "stt", "task", Some("u1"), 1.5, json!({})),
        span_json(
            "speech",
            "user_speech",
            "task",
            Some("u1"),
            1.2,
            json!({"turn.id": "s-session"}),
        ),
        span_json(
            "missing",
            "user_speech",
            "task",
            Some("u1"),
            1.3,
            json!({"turn.id": "s-gone"}),
        ),
    ]);
    let build = build_one(&spans);
    assert!(build
        .voice
        .utterances
        .iter()
        .all(|utterance| utterance.turn_id.is_none()));
    assert_eq!(build.voice.steps.len(), 1);
    assert_eq!(build.voice.steps[0].turn_id, None);
    let warnings: Vec<_> = build
        .warnings
        .iter()
        .map(|warning| (warning.span_id.as_str(), warning.message.as_str()))
        .collect();
    assert_eq!(
        warnings,
        [
            (
                "r-speech",
                "Unresolved voice turn.id s-session: no voice turn has that span ID"
            ),
            (
                "r-missing",
                "Unresolved voice turn.id s-gone: no voice turn has that span ID"
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
        [AudioClip {
            recording_id: "r-session".to_string(),
            recording_name: "call".to_string(),
            channel_index: Some(0),
            start_offset_ms: 750.0,
            end_offset_ms: 1500.5,
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

#[test]
fn speech_interruption_comes_from_metadata() {
    let spans = spans(vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "speech",
            "agent_speech",
            "task",
            Some("session"),
            1.0,
            json!({"contrib.livekit.interrupted": true}),
        ),
        span_json(
            "bad",
            "user_speech",
            "task",
            Some("session"),
            2.0,
            json!({"contrib.livekit.interrupted": "yes"}),
        ),
    ]);
    let build = build_one(&spans);
    assert_eq!(utterance(&build.voice, "r-speech").interrupted, Some(true));
    assert_eq!(utterance(&build.voice, "r-bad").interrupted, None);
    assert_eq!(build.warnings.len(), 1);
    assert_eq!(build.warnings[0].span_id, "r-bad");
    assert!(build.warnings[0]
        .message
        .starts_with("Invalid metadata.contrib.livekit.interrupted"));
}

fn interrupted_agent_turn(speech_outputs: &[Option<&str>], transcripts: &[&str]) -> VoiceBuild {
    let events: Vec<_> = transcripts
        .iter()
        .map(|transcript| {
            json!({
                "name": "playbackFinished",
                "attributes": {"interrupted": true, "synchronizedTranscript": transcript},
            })
        })
        .collect();
    let mut values = vec![
        span_json("session", "session", "task", None, 0.0, json!({})),
        span_json(
            "a1",
            "agent_turn",
            "task",
            Some("session"),
            1.0,
            json!({"turn.id": "s-a1", "contrib.livekit.playback_events": events}),
        ),
    ];
    for (index, output) in speech_outputs.iter().enumerate() {
        let mut speech = span_json(
            &format!("speech{index}"),
            "agent_speech",
            "task",
            Some("a1"),
            2.0 + index as f64,
            json!({"turn.id": "s-a1", "contrib.livekit.interrupted": true}),
        );
        if let Some(output) = output {
            speech["output"] = json!([{"role": "assistant", "content": output}]);
        }
        values.push(speech);
    }
    build_one(&spans(values))
}

fn texts(build: &VoiceBuild) -> Vec<Option<&str>> {
    build
        .voice
        .utterances
        .iter()
        .map(|utterance| utterance.text.as_deref())
        .collect()
}

#[test]
fn interrupted_speech_without_output_uses_the_playback_transcript() {
    let build = interrupted_agent_turn(&[None], &["Spoken so far"]);
    assert_eq!(texts(&build), [Some("Spoken so far")]);
}

#[test]
fn interrupted_speech_with_output_keeps_it() {
    let build = interrupted_agent_turn(&[Some("Logged text")], &["Spoken so far"]);
    assert_eq!(texts(&build), [Some("Logged text")]);
}

#[test]
fn only_the_last_speech_of_a_turn_gets_a_playback_transcript() {
    let build = interrupted_agent_turn(&[None, None], &["First", "Second"]);
    assert_eq!(texts(&build), [None, Some("Second")]);
    let ambiguous = interrupted_agent_turn(&[None, None], &["Only one"]);
    assert_eq!(texts(&ambiguous), [None, None]);
}
