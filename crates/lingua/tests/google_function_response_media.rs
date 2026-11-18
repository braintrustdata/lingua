use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

/// A tool result whose `response` addresses attached media by display name;
/// Google resolves the `{"$ref": ...}` binding server-side.
fn function_response_part() -> serde_json::Value {
    json!({
        "functionResponse": {
            "id": "call_1",
            "name": "render_chart",
            "response": {"image": {"$ref": "chart.png"}},
            "parts": [
                {
                    "inlineData": {
                        "displayName": "chart.png",
                        "mimeType": "image/png",
                        "data": "AAAA"
                    }
                },
                {
                    "fileData": {
                        "displayName": "source.csv",
                        "fileUri": "gs://b/source.csv",
                        "mimeType": "text/csv"
                    }
                }
            ]
        }
    })
}

fn google_request_with_function_response_media() -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "Chart the data."}]},
            {"role": "user", "parts": [function_response_part()]}
        ]
    }))
}

fn google_response_with_function_response_media() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [function_response_part()]},
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
        reason.contains("functionResponse.parts"),
        "rejection must name functionResponse.parts: {reason}"
    );
}

#[test]
fn google_function_response_media_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error =
            lingua::transform_request(google_request_with_function_response_media(), target, None)
                .expect_err("tool-result media handles have no cross-provider mapping");
        assert_unsupported(&error);

        let error =
            lingua::transform_response(google_response_with_function_response_media(), target)
                .expect_err("tool-result media handles have no cross-provider mapping");
        assert_unsupported(&error);

        let error =
            lingua::transform_stream_chunk(google_response_with_function_response_media(), target)
                .expect_err("tool-result media handles have no cross-provider stream mapping");
        assert_unsupported(&error);
    }
}

#[test]
fn google_function_response_media_survives_native_passthrough() {
    let request = google_request_with_function_response_media();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_function_response_media();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}
