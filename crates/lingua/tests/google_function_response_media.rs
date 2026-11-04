use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

/// A Files API handle, resolvable only by Google's own backend.
fn file_data_part() -> serde_json::Value {
    json!({
        "fileData": {
            "fileUri": "https://generativelanguage.googleapis.com/v1beta/files/abc123",
            "mimeType": "video/mp4",
            "displayName": "clip.mp4"
        }
    })
}

/// `AGENTIC` delegates video navigation to Google-managed execution.
fn media_processing_part() -> serde_json::Value {
    json!({
        "inlineData": {"mimeType": "video/mp4", "data": "AAAA"},
        "mediaProcessing": "AGENTIC"
    })
}

/// `displayName` is the target of the `{"$ref": ...}` indirection that
/// `FunctionResponse.response` uses to point at a multimedia part.
fn blob_display_name_part() -> serde_json::Value {
    json!({
        "inlineData": {
            "mimeType": "image/png",
            "data": "iVBORw0KGgo=",
            "displayName": "chart.png"
        }
    })
}

fn function_response(part: serde_json::Value) -> serde_json::Value {
    json!({
        "functionResponse": {
            "id": "call_0",
            "name": "render_chart",
            "response": {"chart": {"$ref": "chart.png"}},
            "parts": [part]
        }
    })
}

fn google_request_with(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "contents": [
            {"role": "user", "parts": [{"text": "Chart the quarterly results."}]},
            {"role": "model", "parts": [{
                "functionCall": {"name": "render_chart", "args": {"quarter": "Q3"}}
            }]},
            {"role": "user", "parts": [function_response(part)]}
        ]
    }))
}

fn google_response_with(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [function_response(part)]},
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
    // `functionResponse` distinguishes the nested tool-result part from the
    // identically named fields on the outer `Part`.
    assert!(
        reason.contains("functionResponse"),
        "rejection must name the functionResponse part: {reason}"
    );
    assert!(
        reason.contains(field),
        "rejection must name {field}: {reason}"
    );
}

#[test]
fn google_function_response_media_part_cross_provider_transform_is_unsupported() {
    let cases = [
        (file_data_part(), "fileData"),
        (media_processing_part(), "mediaProcessing"),
        (blob_display_name_part(), "displayName"),
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
            .expect_err("provider-hosted tool-result media has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_response(google_response_with(part.clone()), target)
                .expect_err("provider-hosted tool-result media has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_stream_chunk(google_response_with(part.clone()), target)
                .expect_err(
                    "provider-hosted tool-result media has no cross-provider stream mapping",
                );
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_function_response_media_part_survives_native_passthrough() {
    for part in [
        file_data_part(),
        media_processing_part(),
        blob_display_name_part(),
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
