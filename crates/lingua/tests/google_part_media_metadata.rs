use lingua::providers::google::generated::{
    DocumentMetadataContentLayer, GenerateContentRequest, MediaResolutionLevel,
    SpreadsheetMetadataContentLayer, VideoMetadataContentLayer,
};
use lingua::{
    Bytes, ProviderFormat, TransformError, TransformResult, serde_json, serde_json::json,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

/// The Google ingestion directives this revision added to `Part`, paired with the
/// camelCase name a cross-provider rejection must name.
fn user_part_metadata() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "audioMetadata",
            json!({"startOffset": "1.5s", "endOffset": "9s"}),
        ),
        (
            "documentMetadata",
            json!({"contentLayers": ["PAGE_TEXT"], "startPage": 2, "endPage": 7}),
        ),
        (
            "spreadsheetMetadata",
            json!({"contentLayers": ["FORMULAS"], "sheetIndex": 1, "visibleCells": "A1:D10"}),
        ),
        (
            "videoMetadata",
            json!({"contentLayers": ["TRANSCRIPT"], "startOffset": "0s"}),
        ),
    ]
}

fn google_request_with_part_metadata(field: &str, metadata: &serde_json::Value) -> Bytes {
    to_bytes(json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "fileData": {"fileUri": "https://example.com/asset", "mimeType": "video/mp4"},
                field: metadata
            }]
        }]
    }))
}

fn google_response_with_part_metadata(field: &str, metadata: &serde_json::Value) -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {
                "role": "model",
                "parts": [{
                    "inlineData": {"mimeType": "video/mp4", "data": "AAAA"},
                    field: metadata
                }]
            },
            "finishReason": "STOP"
        }]
    }))
}

/// The directives this revision added to the parts nested inside a `functionResponse`.
fn function_response_part_metadata() -> Vec<(&'static str, serde_json::Value)> {
    let mut metadata = user_part_metadata();
    metadata.push(("mediaResolution", json!({"level": "MEDIA_RESOLUTION_LOW"})));
    metadata
}

fn google_request_with_function_response_part_metadata(
    field: &str,
    metadata: &serde_json::Value,
) -> Bytes {
    to_bytes(json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "functionResponse": {
                    "id": "call_1",
                    "name": "render_chart",
                    "response": {"output": "ok"},
                    "parts": [{
                        "inlineData": {"mimeType": "image/png", "data": "AAAA"},
                        field: metadata
                    }]
                }
            }]
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

fn assert_passes_through_request(request: Bytes) {
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);
}

#[test]
fn google_media_metadata_cross_provider_transform_is_unsupported() {
    for target in [
        ProviderFormat::Anthropic,
        ProviderFormat::ChatCompletions,
        ProviderFormat::Responses,
    ] {
        for (field, metadata) in user_part_metadata() {
            let error = lingua::transform_request(
                google_request_with_part_metadata(field, &metadata),
                target,
                None,
            )
            .expect_err("Google media preprocessing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let response = google_response_with_part_metadata(field, &metadata);
            let error = lingua::transform_response(response.clone(), target)
                .expect_err("Google media preprocessing has no cross-provider mapping");
            assert_unsupported(&error, field);

            let error = lingua::transform_stream_chunk(response, target)
                .expect_err("Google media preprocessing has no cross-provider stream mapping");
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_function_response_part_media_metadata_cross_provider_transform_is_unsupported() {
    for target in [
        ProviderFormat::Anthropic,
        ProviderFormat::ChatCompletions,
        ProviderFormat::Responses,
    ] {
        for (field, metadata) in function_response_part_metadata() {
            let request = google_request_with_function_response_part_metadata(field, &metadata);
            let error = lingua::transform_request(request, target, None)
                .expect_err("tool-result media preprocessing has no cross-provider mapping");
            assert_unsupported(&error, field);
        }
    }
}

#[test]
fn google_media_metadata_survives_native_passthrough() {
    for (field, metadata) in function_response_part_metadata() {
        assert_passes_through_request(google_request_with_function_response_part_metadata(
            field, &metadata,
        ));
    }

    for (field, metadata) in user_part_metadata() {
        assert_passes_through_request(google_request_with_part_metadata(field, &metadata));

        let response = google_response_with_part_metadata(field, &metadata);
        let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
            .expect("a native Google response must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google responses must not be re-serialized");
        };
        assert_eq!(actual, response);

        let chunk = lingua::transform_stream_chunk(response.clone(), ProviderFormat::Google)
            .expect("a native Google stream chunk must pass through");
        let TransformResult::PassThrough(actual) = chunk else {
            panic!("native Google stream chunks must not be re-serialized");
        };
        assert_eq!(actual, response);
    }
}

#[test]
fn google_media_metadata_content_layer_wire_names_roundtrip() {
    // Pin the wire spellings of the enums this revision added, so native acceptance of
    // each content layer is proven by the generated types rather than assumed.
    let wire = json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "fileData": {"fileUri": "https://example.com/asset", "mimeType": "video/mp4"},
                "audioMetadata": {"startOffset": "1.5s", "endOffset": "9s"},
                "documentMetadata": {
                    "contentLayers": ["PAGE_IMAGE", "PAGE_TEXT"],
                    "startPage": 2,
                    "endPage": 7
                },
                "spreadsheetMetadata": {
                    "contentLayers": ["DATA", "FORMULAS", "VISUAL"],
                    "sheetIndex": 1,
                    "visibleCells": "A1:D10"
                },
                "videoMetadata": {"contentLayers": ["FRAMES", "AUDIO", "TRANSCRIPT"]}
            }]
        }]
    });

    let request: GenerateContentRequest =
        serde_json::from_value(wire.clone()).expect("the generated types accept the native shape");
    let part = &request.contents.as_ref().expect("contents")[0]
        .parts
        .as_ref()
        .expect("parts")[0];
    assert_eq!(
        part.document_metadata
            .as_ref()
            .and_then(|metadata| metadata.content_layers.clone()),
        Some(vec![
            DocumentMetadataContentLayer::PageImage,
            DocumentMetadataContentLayer::PageText
        ])
    );
    assert_eq!(
        part.spreadsheet_metadata
            .as_ref()
            .and_then(|metadata| metadata.content_layers.clone()),
        Some(vec![
            SpreadsheetMetadataContentLayer::Data,
            SpreadsheetMetadataContentLayer::Formulas,
            SpreadsheetMetadataContentLayer::Visual
        ])
    );
    assert_eq!(
        part.video_metadata
            .as_ref()
            .and_then(|metadata| metadata.content_layers.clone()),
        Some(vec![
            VideoMetadataContentLayer::Frames,
            VideoMetadataContentLayer::Audio,
            VideoMetadataContentLayer::Transcript
        ])
    );

    assert_eq!(
        serde_json::to_value(&request).expect("the request re-serializes"),
        wire
    );
    assert_passes_through_request(to_bytes(wire));
}

#[test]
fn google_function_response_part_media_resolution_wire_names_roundtrip() {
    let wire = json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "functionResponse": {
                    "name": "render_chart",
                    "parts": [{
                        "inlineData": {"mimeType": "image/png", "data": "AAAA"},
                        "mediaResolution": {"level": "MEDIA_RESOLUTION_ULTRA_HIGH"}
                    }]
                }
            }]
        }]
    });

    let request: GenerateContentRequest =
        serde_json::from_value(wire.clone()).expect("the generated types accept the native shape");
    let nested_part = &request.contents.as_ref().expect("contents")[0]
        .parts
        .as_ref()
        .expect("parts")[0]
        .function_response
        .as_ref()
        .expect("functionResponse")
        .parts
        .as_ref()
        .expect("functionResponse parts")[0];
    assert_eq!(
        nested_part
            .media_resolution
            .as_ref()
            .and_then(|resolution| resolution.level.clone()),
        Some(MediaResolutionLevel::MediaResolutionUltraHigh)
    );

    assert_eq!(
        serde_json::to_value(&request).expect("the request re-serializes"),
        wire
    );
    assert_passes_through_request(to_bytes(wire));
}
