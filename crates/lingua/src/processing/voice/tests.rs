use super::*;
use crate::serde_json as json;

fn read(value: Value) -> Result<VoiceCall, String> {
    let rows = json::from_value::<Vec<VoiceSpan>>(value).map_err(|e| e.to_string())?;
    import_voice_call(&rows)
}

fn attachment() -> Value {
    json::json!({"type":"braintrust_attachment", "filename":"call.wav", "content_type":"audio/wav", "key":"fixture"})
}

#[test]
fn speaking_text_audio_timing_and_interruption() {
    let call = read(json::json!([
        {"span_id":"session", "span_attributes":{"name":"agent_session"}},
        {"span_id":"recording", "span_attributes":{"name":"call_recording"}, "metrics":{"start":10, "end":20}, "output":attachment()},
        {"span_id":"agent", "span_parents":["turn"], "span_attributes":{"name":"agent_speaking"}, "metrics":{"start":18,"end":22}, "output":{"text":" spoken ", "audio":attachment()}, "metadata":{"interrupted":false}},
        {"span_id":"turn", "span_attributes":{"name":"agent_turn"}, "output":[{"role":"assistant", "content":"generated"}], "metadata":{"lk.interrupted":true}},
        {"span_id":"user-turn", "span_attributes":{"name":"user_turn"}, "input":[{"role":"user", "content":" Hello "}], "metadata":{"lk.pii.user_transcript":"unused"}},
        {"span_id":"user", "span_parents":["user-turn"], "span_attributes":{"name":"user_speaking"}, "metrics":{"start":9, "end":11}, "input":{"audio":attachment()}}
    ])).unwrap();
    assert_eq!(call.recordings.len(), 1);
    assert_eq!(call.utterances.len(), 2);
    let user = &call.utterances[0];
    assert_eq!(user.speaker, Speaker::User);
    assert_eq!(user.text.as_deref(), Some("Hello"));
    assert!(user.clip.is_some());
    assert_eq!(user.start_ms, Some(9000.0));
    let range = user.range.as_ref().unwrap();
    assert_eq!((range.start_ms, range.end_ms), (0.0, 1000.0));
    let agent = &call.utterances[1];
    assert_eq!(agent.text.as_deref(), Some("spoken"));
    assert_eq!(agent.interrupted, Some(false));
    assert_eq!(agent.range.as_ref().unwrap().end_ms, 10000.0);
    assert!(
        matches!(&call.messages[0], Message::User { content: UserContent::String(text) } if text == "Hello")
    );
    assert!(
        matches!(&call.messages[1], Message::Assistant { content: AssistantContent::String(text), .. } if text == "spoken")
    );
}

#[test]
fn ambiguous_turn_text_has_no_invented_timing() {
    let call = read(json::json!([
        {"span_id":"turn", "span_attributes":{"name":"user_turn"}, "metrics":{"start":1,"end":4}, "metadata":{"lk.pii.user_transcript":" hello "}},
        {"span_id":"first", "span_parents":["turn"], "span_attributes":{"name":"user_speaking"}, "metrics":{"start":2,"end":3}},
        {"span_id":"second", "span_parents":["turn"], "span_attributes":{"name":"user_speaking"}, "metrics":{"start":3,"end":4}}
    ])).unwrap();
    assert_eq!(call.utterances.len(), 3);
    assert_eq!(call.utterances[0].text.as_deref(), Some("hello"));
    assert!(call.utterances[0].start_ms.is_none());
    assert!(call.utterances[0].end_ms.is_none());
    assert!(call.utterances[1].text.is_none());
    assert!(call.utterances[2].text.is_none());
    assert_eq!(call.messages.len(), 1);
}

#[test]
fn root_recording_is_unbounded_and_non_audio_is_ignored() {
    let call = read(json::json!([
        {"span_id":"root", "span_attributes":{"name":"livekit_agent_session"}, "span_parents":null, "metrics":{"start":1,"end":10}, "input":{"audio":attachment()}},
        {"span_id":"speech", "span_parents":["root"], "span_attributes":{"name":"agent_speaking"}, "metrics":{"start":2,"end":3}},
        {"span_id":"image", "span_attributes":{"name":"call_recording"}, "output":{"type":"braintrust_attachment", "filename":"image.png", "content_type":"image/png", "key":"fixture"}}
    ])).unwrap();
    assert_eq!(call.recordings.len(), 1);
    assert!(call.recordings[0].start_ms.is_none());
    assert!(call.recordings[0].end_ms.is_none());
    assert!(call.utterances[0].range.is_none());
    assert!(call.messages.is_empty());
}

#[test]
fn multiple_bounded_recordings_do_not_guess_a_range() {
    let call = read(json::json!([
        {"span_id":"session", "span_attributes":{"name":"agent_session"}},
        {"span_id":"a", "span_attributes":{"name":"call_recording"}, "metrics":{"start":1,"end":10}, "output":attachment()},
        {"span_id":"b", "span_attributes":{"name":"call_recording"}, "metrics":{"start":1,"end":10}, "output":attachment()},
        {"span_id":"speech", "span_parents":["session"], "span_attributes":{"name":"agent_speaking"}, "metrics":{"start":2,"end":3}}
    ])).unwrap();
    assert_eq!(call.recordings.len(), 2);
    assert!(call.utterances[0].range.is_none());
}

#[test]
fn missing_times_and_external_audio() {
    let call = read(json::json!([
        {"span_id":"root", "metadata":{"lk.sdk":"livekit"}, "output":{"audio":{"type":"external_attachment", "filename":"call.ogg", "content_type":"audio/ogg", "url":"https://example.com/call.ogg"}}},
        {"span_id":"speech", "span_parents":["root"], "span_attributes":{"name":"agent_speaking"}, "output":{"text":" hello "}}
    ])).unwrap();
    assert!(matches!(
        call.recordings[0].attachment,
        VoiceAttachment::External { .. }
    ));
    assert!(call.utterances[0].start_ms.is_none());
    assert_eq!(call.utterances[0].text.as_deref(), Some("hello"));
}

#[test]
fn empty_and_unrecognized_traces() {
    for rows in [
        json::json!([]),
        json::json!([{"span_id":"speech", "span_attributes":{"name":"agent_speaking"}, "output":{"text":"not recognized"}}]),
    ] {
        let call = read(rows).unwrap();
        assert!(call.recordings.is_empty());
        assert!(call.utterances.is_empty());
        assert!(call.messages.is_empty());
    }
}

#[test]
fn invalid_known_fields_are_errors() {
    let error = read(json::json!([
        {"span_id":"root", "span_attributes":{"name":"agent_session"}, "metadata":{"lk.interrupted":"yes"}}
    ])).unwrap_err();
    assert!(error.contains("Invalid voice metadata in span root"));
    let error = read(json::json!([
        {"span_id":"root", "span_attributes":{"name":"agent_session"}, "output":{"audio":{"type":"braintrust_attachment", "content_type":"audio/wav", "filename":"call.wav"}}}
    ])).unwrap_err();
    assert!(error.contains("Invalid voice audio attachment"));
    assert!(error.contains("key"));
    assert!(
        read(json::json!([{"span_attributes":{"name":"agent_session"}}]))
            .unwrap_err()
            .contains("span_id")
    );
}

#[test]
fn original_livekit_fixture() {
    let rows: Vec<VoiceSpan> = json::from_str(include_str!("fixtures/livekit-call.json")).unwrap();
    let call = import_voice_call(&rows).unwrap();
    assert_eq!(call.recordings.len(), 1);
    assert_eq!(call.utterances.len(), 10);
    assert_eq!(call.messages.len(), 6);
    assert_eq!(call.utterances[5].interrupted, Some(true));
    assert_eq!(call.utterances[5].range.as_ref().unwrap().start_ms, 24199.0);
}
