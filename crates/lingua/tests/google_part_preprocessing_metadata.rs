use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn google_request_with_part(part: serde_json::Value) -> Bytes {
    to_bytes(json!({
        "contents": [{
            "role": "user",
            "parts": [{"text": "Analyze this."}, part]
        }]
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

/// Each case pairs a Google-side media-preprocessing directive with the field
/// token the rejection must name.
fn preprocessing_parts() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "audioMetadata",
            json!({
                "inlineData": {"mimeType": "audio/mpeg", "data": "AAAA"},
                "audioMetadata": {"startOffset": "1s", "endOffset": "9s"}
            }),
        ),
        (
            "documentMetadata",
            json!({
                "fileData": {"fileUri": "gs://b/d.pdf", "mimeType": "application/pdf"},
                "documentMetadata": {
                    "contentLayers": ["PAGE_TEXT"],
                    "startPage": 3,
                    "endPage": 7
                }
            }),
        ),
        (
            "spreadsheetMetadata",
            json!({
                "fileData": {"fileUri": "gs://b/s.xlsx", "mimeType": "application/vnd.ms-excel"},
                "spreadsheetMetadata": {
                    "contentLayers": ["DATA", "FORMULAS"],
                    "sheetIndex": 1,
                    "visibleCells": "A1:D10"
                }
            }),
        ),
        (
            "videoMetadata.contentLayers",
            json!({
                "fileData": {"fileUri": "gs://b/v.mp4", "mimeType": "video/mp4"},
                "videoMetadata": {"contentLayers": ["TRANSCRIPT"], "startOffset": "0s"}
            }),
        ),
    ]
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
fn google_part_preprocessing_metadata_cross_provider_transform_is_unsupported() {
    for (field, part) in preprocessing_parts() {
        for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
            let error =
                lingua::transform_request(google_request_with_part(part.clone()), target, None)
                    .expect_err("Google media preprocessing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_response(google_response_with_part(part.clone()), target)
                .expect_err("Google media preprocessing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error =
                lingua::transform_stream_chunk(google_response_with_part(part.clone()), target)
                    .expect_err("Google media preprocessing has no cross-provider stream mapping");
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_part_preprocessing_metadata_survives_native_passthrough() {
    for (_, part) in preprocessing_parts() {
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
