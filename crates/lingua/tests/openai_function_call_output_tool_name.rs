use lingua::universal::{Message, ToolContentPart};
use lingua::{serde_json, serde_json::json, Bytes, ProviderFormat, TransformResult};

fn responses_response_with_function_call_output() -> Bytes {
    let response = json!({
        "id": "resp_tool_name",
        "object": "response",
        "created_at": 1_759_000_000u64,
        "status": "completed",
        "model": "gpt-5.1",
        "output": [
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_list_databases",
                "name": "list_databases",
                "arguments": "{}"
            },
            {
                "type": "function_call_output",
                "id": "fco_1",
                "call_id": "call_list_databases",
                "name": "list_databases",
                "output": "{\"databases\":[\"admin\"]}"
            }
        ],
        "usage": {"input_tokens": 12, "output_tokens": 5, "total_tokens": 17}
    });

    Bytes::from(serde_json::to_vec(&response).expect("response serializes"))
}

#[test]
fn responses_response_function_call_output_imports_as_universal_tool_result() {
    let universal = lingua::response_to_universal(responses_response_with_function_call_output())
        .expect("Responses response parses");

    let tool_result = universal
        .messages
        .iter()
        .find_map(|message| match message {
            Message::Tool { content } => content.iter().find_map(|part| match part {
                ToolContentPart::ToolResult(result) => Some(result),
                _ => None,
            }),
            _ => None,
        })
        .expect("function_call_output must import instead of being dropped");

    assert_eq!(tool_result.tool_call_id, "call_list_databases");
    assert_eq!(tool_result.tool_name, "list_databases");
    assert_eq!(tool_result.output, json!({"databases": ["admin"]}));
}

#[test]
fn responses_response_with_function_call_output_transforms_to_google_tool_result() {
    let result = lingua::transform_response(
        responses_response_with_function_call_output(),
        ProviderFormat::Google,
    )
    .expect("Responses response with a tool output converts to Google");

    let TransformResult::Transformed { bytes, .. } = result.result else {
        panic!("cross-provider response conversion must transform");
    };

    let converted: serde_json::Value =
        serde_json::from_slice(&bytes).expect("converted response is JSON");
    let function_response = converted["candidates"]
        .as_array()
        .expect("Google responses carry candidates")
        .iter()
        .flat_map(|candidate| {
            candidate["content"]["parts"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .find(|part| part.get("functionResponse").is_some())
        .expect("the tool output must survive as a Google functionResponse part");

    assert_eq!(
        function_response["functionResponse"]["name"],
        json!("list_databases"),
        "tool name must be carried across providers: {converted}"
    );
    assert_eq!(
        function_response["functionResponse"]["response"],
        json!({"databases": ["admin"]})
    );
}
