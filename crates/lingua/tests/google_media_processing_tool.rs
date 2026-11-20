use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

/// MEDIA_PROCESSING names an invisible Google-hosted toolset; the model emits the
/// call and the client echoes the result back, so both halves are provider-owned.
fn media_processing_parts() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "toolCall",
            json!({
                "toolCall": {
                    "id": "srv_mp_1",
                    "toolType": "MEDIA_PROCESSING",
                    "toolName": "google_understanding_video",
                    "args": {"fileUri": "gs://b/v.mp4"}
                }
            }),
        ),
        (
            "toolResponse",
            json!({
                "toolResponse": {
                    "id": "srv_mp_1",
                    "toolType": "MEDIA_PROCESSING",
                    "toolName": "google_understanding_video",
                    "response": {"summary": "A cat on a skateboard."}
                }
            }),
        ),
    ]
}

fn google_request_with_part(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "Describe the clip."}]},
            {"role": "model", "parts": [part]}
        ]
    }))
}

fn google_response_with_part(part: serde_json::Value) -> Bytes {
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
fn google_media_processing_tool_cross_provider_transform_is_unsupported() {
    for (field, part) in media_processing_parts() {
        for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
            let error =
                lingua::transform_request(google_request_with_part(part.clone()), target, None)
                    .expect_err("a hosted Google toolset has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_response(google_response_with_part(part.clone()), target)
                .expect_err("a hosted Google toolset has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error =
                lingua::transform_stream_chunk(google_response_with_part(part.clone()), target)
                    .expect_err("a hosted Google toolset has no cross-provider stream mapping");
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_media_processing_tool_survives_native_passthrough() {
    for (_, part) in media_processing_parts() {
        let request = google_request_with_part(part.clone());
        let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
            .expect("a native Google request must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google requests must not be re-serialized");
        };
        assert_eq!(actual, request);

        let response = google_response_with_part(part);
        let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
            .expect("a native Google response must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google responses must not be re-serialized");
        };
        assert_eq!(actual, response);
    }
}
