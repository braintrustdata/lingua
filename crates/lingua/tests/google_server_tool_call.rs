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

fn google_response_with_media_processing_tool_call() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [{
                "toolCall": {
                    "id": "srv_2",
                    "toolType": "MEDIA_PROCESSING",
                    "toolName": "media_processing",
                    "args": {"fileUri": "https://example.com/clip.mp4"}
                }
            }]},
            "finishReason": "STOP"
        }]
    }))
}

fn assert_unsupported(error: &TransformError) {
    let TransformError::ToUniversalFailed(reason) = error else {
        panic!("expected a to-universal conversion failure, got {error:?}");
    };
    assert!(
        reason.contains("Unsupported mapping"),
        "rejection must use the unsupported-mapping category: {reason}"
    );
    assert!(
        reason.contains("toolCall"),
        "rejection must name toolCall: {reason}"
    );
}

#[test]
fn google_server_tool_call_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(google_request_with_server_tool_call(), target, None)
            .expect_err("provider-executed tool calls have no cross-provider mapping");
        assert_unsupported(&error);

        let error = lingua::transform_response(google_response_with_server_tool_call(), target)
            .expect_err("provider-executed tool calls have no cross-provider mapping");
        assert_unsupported(&error);

        let error = lingua::transform_stream_chunk(google_response_with_server_tool_call(), target)
            .expect_err("provider-executed tool calls have no cross-provider stream mapping");
        assert_unsupported(&error);
    }
}

#[test]
fn google_media_processing_tool_call_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error =
            lingua::transform_response(google_response_with_media_processing_tool_call(), target)
                .expect_err(
                    "provider-hosted media processing tools have no cross-provider mapping",
                );
        assert_unsupported(&error);

        let error = lingua::transform_stream_chunk(
            google_response_with_media_processing_tool_call(),
            target,
        )
        .expect_err("provider-hosted media processing tools have no cross-provider stream mapping");
        assert_unsupported(&error);
    }
}

#[test]
fn google_media_processing_tool_call_survives_native_passthrough() {
    let response = google_response_with_media_processing_tool_call();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
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
