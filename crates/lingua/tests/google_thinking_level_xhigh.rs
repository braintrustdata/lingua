use lingua::{serde_json, serde_json::json, Bytes, ProviderFormat, TransformResult};

fn google_request_with_xhigh_thinking_level() -> Bytes {
    Bytes::from(
        serde_json::to_vec(&json!({
            "model": "gemini-3.5-flash",
            "contents": [{
                "role": "user",
                "parts": [{"text": "Prove the result."}]
            }],
            "generationConfig": {
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "XHIGH"
                }
            }
        }))
        .expect("payload serializes"),
    )
}

#[test]
fn google_thinking_level_xhigh_transforms_to_anthropic_top_effort() {
    // thinkingLevel is a portable reasoning-depth knob, so the transform must
    // succeed and carry the top-tier effort through instead of downgrading it.
    let result = lingua::transform_request(
        google_request_with_xhigh_thinking_level(),
        ProviderFormat::Anthropic,
        Some("claude-opus-5"),
    )
    .expect("XHIGH thinking level has a cross-provider mapping");

    let TransformResult::Transformed { bytes, .. } = result.result else {
        panic!("a Google request must be transformed for an Anthropic target");
    };
    let transformed: serde_json::Value =
        serde_json::from_slice(&bytes).expect("transformed request is JSON");

    assert_eq!(
        transformed["output_config"]["effort"],
        json!("xhigh"),
        "Google XHIGH must reach Anthropic's top effort: {transformed}"
    );
}
