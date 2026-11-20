use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn server_tool_call_part() -> serde_json::Value {
    json!({
        "toolCall": {
            "id": "srv_1",
            "toolType": "GOOGLE_SEARCH_WEB",
            "toolName": "google_search",
            "args": {"query": "weather in Paris"}
        }
    })
}

fn google_request_with_server_tool_call() -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "What is the weather in Paris?"}]},
            {"role": "model", "parts": [server_tool_call_part()]}
        ]
    }))
}

fn google_response_with_server_tool_call() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [server_tool_call_part()]},
            "finishReason": "STOP"
        }]
    }))
}

fn media_processing_tool_response_part() -> serde_json::Value {
    json!({
        "toolResponse": {
            "id": "srv_2",
            "toolType": "MEDIA_PROCESSING",
            "response": {"summary": "a cat chasing a laser pointer"}
        }
    })
}

fn google_request_with_tool_response() -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "Describe the clip."}]},
            {"role": "user", "parts": [media_processing_tool_response_part()]}
        ]
    }))
}

fn google_response_with_tool_response() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [media_processing_tool_response_part()]},
            "finishReason": "STOP"
        }]
    }))
}

fn assert_unsupported(error: &TransformError, field: &str) {
    let TransformError::ToUniversalFailed(reason) = error else {
        panic!("expected a to-universal conversion failure, got {error:?}");
    };
    assert!(
        reason.contains("Unsupported mapping"),
        "rejection must use the unsupported-mapping category: {reason}"
    );
    assert!(
        reason.contains(field),
        "rejection must name {field}: {reason}"
    );
}

#[test]
fn google_server_tool_call_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(google_request_with_server_tool_call(), target, None)
            .expect_err("provider-executed tool calls have no cross-provider mapping");
        assert_unsupported(&error, "toolCall");

        let error = lingua::transform_response(google_response_with_server_tool_call(), target)
            .expect_err("provider-executed tool calls have no cross-provider mapping");
        assert_unsupported(&error, "toolCall");

        let error = lingua::transform_stream_chunk(google_response_with_server_tool_call(), target)
            .expect_err("provider-executed tool calls have no cross-provider stream mapping");
        assert_unsupported(&error, "toolCall");
    }
}

#[test]
fn google_media_processing_tool_response_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(google_request_with_tool_response(), target, None)
            .expect_err("provider-executed tool responses have no cross-provider mapping");
        assert_unsupported(&error, "toolResponse");

        let error = lingua::transform_response(google_response_with_tool_response(), target)
            .expect_err("provider-executed tool responses have no cross-provider mapping");
        assert_unsupported(&error, "toolResponse");

        let error = lingua::transform_stream_chunk(google_response_with_tool_response(), target)
            .expect_err("provider-executed tool responses have no cross-provider stream mapping");
        assert_unsupported(&error, "toolResponse");
    }
}

#[test]
fn google_server_tool_call_survives_native_passthrough() {
    let request = google_request_with_server_tool_call();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_server_tool_call();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}

#[test]
fn google_media_processing_tool_response_survives_native_passthrough() {
    let request = google_request_with_tool_response();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_tool_response();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}
