use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

/// The server-side invocation of Google's invisible media-processing tool.
fn media_processing_tool_call_part() -> serde_json::Value {
    json!({
        "toolCall": {
            "id": "srv_media_1",
            "toolType": "MEDIA_PROCESSING",
            "toolName": "media_processing",
            "args": {"fileUri": "https://generativelanguage.googleapis.com/v1beta/files/abc123"}
        }
    })
}

/// The echoed result of that invocation, which carries no caller-visible
/// contract of its own.
fn media_processing_tool_response_part() -> serde_json::Value {
    json!({
        "toolResponse": {
            "id": "srv_media_1",
            "toolType": "MEDIA_PROCESSING",
            "response": {"frames": 42}
        }
    })
}

fn google_request_with(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "What happens in the video?"}]},
            {"role": "model", "parts": [part]}
        ]
    }))
}

fn google_response_with(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [part]},
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
fn google_tool_type_media_processing_cross_provider_transform_is_unsupported() {
    let cases = [
        (media_processing_tool_call_part(), "toolCall"),
        (media_processing_tool_response_part(), "toolResponse"),
    ];

    for (part, field) in cases {
        for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
            // A model is supplied so the request is otherwise valid for the
            // target: the only reason it can fail is the rejected part.
            let error = lingua::transform_request(
                google_request_with(part.clone()),
                target,
                Some("gemini-2.5-flash"),
            )
            .expect_err("provider-executed media processing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_response(google_response_with(part.clone()), target)
                .expect_err("provider-executed media processing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_stream_chunk(google_response_with(part.clone()), target)
                .expect_err(
                    "provider-executed media processing has no cross-provider stream mapping",
                );
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_tool_type_media_processing_survives_native_passthrough() {
    for part in [
        media_processing_tool_call_part(),
        media_processing_tool_response_part(),
    ] {
        let request = google_request_with(part.clone());
        let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
            .expect("a native Google request must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google requests must not be re-serialized");
        };
        assert_eq!(actual, request);

        let response = google_response_with(part);
        let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
            .expect("a native Google response must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google responses must not be re-serialized");
        };
        assert_eq!(actual, response);
    }
}
