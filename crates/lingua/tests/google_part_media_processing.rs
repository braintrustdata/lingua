use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn google_request_with_media_processing() -> Bytes {
    to_bytes(json!({
        "contents": [{
            "role": "user",
            "parts": [
                {"text": "Summarize this video."},
                {
                    "fileData": {
                        "fileUri": "https://example.com/video.mp4",
                        "mimeType": "video/mp4"
                    },
                    "mediaProcessing": "AGENTIC"
                }
            ]
        }]
    }))
}

fn google_response_with_media_processing() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {
                "role": "model",
                "parts": [{
                    "inlineData": {"mimeType": "video/mp4", "data": "AAAA"},
                    "mediaProcessing": "STATIC"
                }]
            },
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
        reason.contains("mediaProcessing"),
        "rejection must name mediaProcessing: {reason}"
    );
}

#[test]
fn google_media_processing_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(google_request_with_media_processing(), target, None)
            .expect_err("provider media processing has no cross-provider mapping");
        assert_unsupported(&error);

        let error = lingua::transform_response(google_response_with_media_processing(), target)
            .expect_err("provider media processing has no cross-provider mapping");
        assert_unsupported(&error);

        let error = lingua::transform_stream_chunk(google_response_with_media_processing(), target)
            .expect_err("provider media processing has no cross-provider stream mapping");
        assert_unsupported(&error);
    }
}

#[test]
fn google_media_processing_survives_native_passthrough() {
    let request = google_request_with_media_processing();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_media_processing();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}
