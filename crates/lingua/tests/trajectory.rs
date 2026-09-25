use lingua::{serde_json, universal::Work, Trajectory, UniversalParams};

#[test]
fn trajectory_preserves_json_values_and_provider_params() {
    let source = r#"{
        "scope": [{ "type": "trace", "trace_id": "trace" }],
        "agent": { "metadata": { "id": 123456789012345678901234567890 } },
        "metadata": { "id": 123456789012345678901234567890 },
        "turns": [{
            "request_id": "request",
            "start_time": "2026-09-25T12:00:00Z",
            "params": {
                "extras": {
                    "openai": { "n": 3, "logit_bias": { "42": 1 } },
                    "anthropic": { "inference_geo": "us" }
                }
            },
            "work": [{
                "id": "analysis",
                "span_type": "llm",
                "start_time": "2026-09-25T12:00:01Z",
                "work": {
                    "type": "llm_analysis",
                    "params": { "extras": { "responses": { "background": true } } }
                }
            }, {
                "id": "tool",
                "span_type": "tool",
                "start_time": "2026-09-25T12:00:02Z",
                "error": { "id": 123456789012345678901234567890 },
                "work": {
                    "type": "tool_result",
                    "input": { "id": 123456789012345678901234567890 }
                }
            }]
        }]
    }"#;
    let original: Trajectory = serde_json::from_str(source).unwrap();
    let encoded = serde_json::to_string(&original).unwrap();
    let decoded: Trajectory = serde_json::from_str(&encoded).unwrap();
    let expected =
        serde_json::from_str::<serde_json::Value>(r#"{ "id": 123456789012345678901234567890 }"#)
            .unwrap();

    assert_eq!(serde_json::to_value(&decoded.metadata).unwrap(), expected);
    assert_eq!(
        serde_json::to_value(&decoded.agent.metadata).unwrap(),
        expected
    );
    let original_turn = &original.turns[0];
    let decoded_turn = &decoded.turns[0];
    assert_eq!(
        decoded_turn.params.as_ref().unwrap().extras,
        original_turn.params.as_ref().unwrap().extras
    );
    assert_eq!(decoded_turn.params.as_ref().unwrap().extras.len(), 2);
    let (Work::LLMAnalysis(original_analysis), Work::LLMAnalysis(decoded_analysis)) =
        (&original_turn.work[0].work, &decoded_turn.work[0].work)
    else {
        panic!("expected LLM analysis");
    };
    assert_eq!(
        decoded_analysis.params.as_ref().unwrap().extras,
        original_analysis.params.as_ref().unwrap().extras
    );
    assert_eq!(decoded_analysis.params.as_ref().unwrap().extras.len(), 1);
    assert_eq!(decoded_turn.work[1].error.as_ref(), Some(&expected));
    let Work::ToolResult(tool) = &decoded_turn.work[1].work else {
        panic!("expected tool result");
    };
    assert_eq!(tool.input.as_ref(), Some(&expected));
}

#[test]
fn params_without_extras_remain_compatible() {
    let params: UniversalParams = serde_json::from_str(r#"{"temperature":0.5}"#).unwrap();
    assert!(params.extras.is_empty());
    assert_eq!(params.temperature, Some(0.5));
    let serialized = serde_json::to_value(&params).unwrap();
    assert!(!serialized.as_object().unwrap().contains_key("extras"));
}
