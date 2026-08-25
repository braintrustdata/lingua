//! Temporary scratch check (deleted after running): confirms the regenerated Anthropic
//! request/response types accept the new synchronized-spec surface and still accept the
//! payloads Lingua accepted before, in both directions.

use lingua::serde_json::{self, json, Value};

fn round_trip_request(payload: Value) {
    let text = serde_json::to_string(&payload).unwrap();
    let parsed = lingua::validation::anthropic::validate_anthropic_request(&text)
        .unwrap_or_else(|e| panic!("request rejected: {e:?}\n{text}"));
    let reserialized = serde_json::to_value(&parsed).unwrap();
    assert_subset(&payload, &reserialized, "");
}

fn round_trip_response(payload: Value) {
    let text = serde_json::to_string(&payload).unwrap();
    let parsed = lingua::validation::anthropic::validate_anthropic_response(&text)
        .unwrap_or_else(|e| panic!("response rejected: {e:?}\n{text}"));
    let reserialized = serde_json::to_value(&parsed).unwrap();
    assert_subset(&payload, &reserialized, "");
}

/// Every field of the original payload must survive parse + serialize.
fn assert_subset(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, v) in e {
                let child = a
                    .get(k)
                    .unwrap_or_else(|| panic!("dropped field at {path}/{k}"));
                assert_subset(v, child, &format!("{path}/{k}"));
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len(), "array length changed at {path}");
            for (i, (ev, av)) in e.iter().zip(a).enumerate() {
                assert_subset(ev, av, &format!("{path}/{i}"));
            }
        }
        (e, a) => assert_eq!(e, a, "value changed at {path}"),
    }
}

#[test]
fn retained_mid_conv_system_block_still_parses() {
    round_trip_request(json!({
        "model": "claude-opus-4-8",
        "max_tokens": 16,
        "messages": [
            { "role": "user", "content": "hi" },
            {
                "role": "system",
                "content": [
                    {
                        "type": "mid_conv_system",
                        "cache_control": { "type": "ephemeral" },
                        "content": [{ "type": "text", "text": "updated policy" }]
                    }
                ]
            }
        ]
    }));
}

#[test]
fn new_spec_request_surface_round_trips() {
    // container params with skills
    round_trip_request(json!({
        "model": "claude-opus-4-8", "max_tokens": 16,
        "container": { "id": "c_1", "skills": [{ "skill_id": "pdf", "type": "anthropic", "version": "latest" }] },
        "messages": [{ "role": "user", "content": "hi" }]
    }));
    // image transformations + file source
    round_trip_request(json!({
        "model": "claude-opus-4-8", "max_tokens": 16,
        "messages": [{ "role": "user", "content": [
            {
                "type": "image",
                "source": { "type": "file", "file_id": "file_1" },
                "transformations": { "oversized_image": "error" }
            }
        ]}]
    }));
    // browser / computer toolsets
    round_trip_request(json!({
        "model": "claude-opus-4-8", "max_tokens": 16,
        "tools": [
            { "type": "browser_toolset_20260801", "configs": { "navigate": { "enabled": true } } },
            { "type": "computer_toolset_20260801", "configs": { "zoom": { "enabled": false } } }
        ],
        "messages": [{ "role": "user", "content": "hi" }]
    }));
    // toolset member tool_use / tool_result with browser_state block
    round_trip_request(json!({
        "model": "claude-opus-4-8", "max_tokens": 16,
        "messages": [
            { "role": "assistant", "content": [
                { "type": "tool_use", "id": "tu_1", "name": "navigate", "toolset_name": "browser", "input": { "url": "https://example.com" } }
            ]},
            { "role": "user", "content": [
                { "type": "tool_result", "tool_use_id": "tu_1", "toolset_name": "browser", "content": [
                    { "type": "text", "text": "ok" },
                    {
                        "type": "browser_state",
                        "tabs": [{ "tab_id": "t1", "title": "Example", "url": "https://example.com", "active": true }],
                        "state_changes": [{ "type": "tab_opened", "tab_id": "t1" }]
                    }
                ]}
            ]}
        ]
    }));
}

#[test]
fn new_spec_response_surface_round_trips() {
    round_trip_response(json!({
        "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-4-8",
        "stop_reason": "end_turn",
        "container": { "id": "c_1", "expires_at": "2026-01-01T00:00:00Z",
                       "skills": [{ "skill_id": "pdf", "type": "anthropic", "version": "1" }] },
        "content": [
            { "type": "text", "text": "hi" },
            { "type": "tool_use", "id": "tu_1", "name": "navigate", "toolset_name": "browser", "input": {} }
        ],
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    }));
}

fn probe(payload: Value) {
    use lingua::providers::anthropic::generated;
    use lingua::universal::convert::TryFromLLM;
    use lingua::universal::Message;

    let input: generated::InputMessage = serde_json::from_value(payload.clone()).unwrap();
    let universal = <Message as TryFromLLM<generated::InputMessage>>::try_from(input).unwrap();
    let back = <generated::InputMessage as TryFromLLM<Message>>::try_from(universal).unwrap();
    println!(
        "ROUNDTRIP: {}",
        serde_json::to_string_pretty(&serde_json::to_value(&back).unwrap()).unwrap()
    );
}

#[test]
fn probe_pairs() {
    println!("--- baseline: tool_result only (no new fields)");
    probe(json!({"role":"user","content":[
        {"type":"tool_result","tool_use_id":"tu_1","content":[{"type":"text","text":"ok"}]}]}));
    println!("--- new: tool_result + toolset_name");
    probe(json!({"role":"user","content":[
        {"type":"tool_result","tool_use_id":"tu_1","toolset_name":"browser","content":[{"type":"text","text":"ok"}]}]}));
    println!("--- baseline: base64 image");
    probe(json!({"role":"user","content":[
        {"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":"AAAA"}}]}));
    println!("--- new: base64 image + transformations");
    probe(json!({"role":"user","content":[
        {"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":"AAAA"},"transformations":{"oversized_image":"error"}}]}));
    println!("--- new: file image source");
    probe(json!({"role":"user","content":[
        {"type":"image","source":{"type":"file","file_id":"file_1"}}]}));
}
