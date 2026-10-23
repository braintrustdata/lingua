use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn google_request_with_continuation_token() -> Bytes {
    to_bytes(json!({
        "contents": [{"role": "user", "parts": [{"text": "Write a long essay."}]}],
        "generationConfig": {"maxOutputTokens": 2048},
        "continuationToken": "opaque-continuation-state"
    }))
}

fn google_response_with_continuation_token() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [{"text": "The first part"}]},
            "finishReason": "CONTINUATION",
            "continuationToken": "opaque-continuation-state"
        }],
        "modelVersion": "gemini-3.5-flash"
    }))
}

fn google_response_with_continuation_finish_reason() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {"role": "model", "parts": [{"text": "The first part"}]},
            "finishReason": "CONTINUATION"
        }],
        "modelVersion": "gemini-3.5-flash"
    }))
}

fn assert_unsupported(error: &TransformError, fields: &[&str]) {
    let TransformError::ToUniversalFailed(reason) = error else {
        panic!("expected a to-universal conversion failure, got {error:?}");
    };
    assert!(
        reason.contains("Unsupported mapping"),
        "rejection must use the unsupported-mapping category: {reason}"
    );
    assert!(
        fields.iter().any(|field| reason.contains(field)),
        "rejection must name one of {fields:?}: {reason}"
    );
}

#[test]
fn google_continuation_token_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error =
            lingua::transform_request(google_request_with_continuation_token(), target, None)
                .expect_err("continuation state has no cross-provider mapping");
        assert_unsupported(&error, &["continuationToken"]);

        let error = lingua::transform_response(google_response_with_continuation_token(), target)
            .expect_err("continuation state has no cross-provider mapping");
        assert_unsupported(&error, &["continuationToken", "CONTINUATION"]);

        let error =
            lingua::transform_response(google_response_with_continuation_finish_reason(), target)
                .expect_err("a CONTINUATION finish reason has no cross-provider mapping");
        assert_unsupported(&error, &["CONTINUATION"]);

        let error =
            lingua::transform_stream_chunk(google_response_with_continuation_token(), target)
                .expect_err("continuation state has no cross-provider stream mapping");
        assert_unsupported(&error, &["continuationToken", "CONTINUATION"]);
    }
}

#[test]
fn google_continuation_token_survives_native_passthrough() {
    let request = google_request_with_continuation_token();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_continuation_token();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);

    let chunk = google_response_with_continuation_token();
    let TransformResult::PassThrough(actual) =
        lingua::transform_stream_chunk(chunk.clone(), ProviderFormat::Google)
            .expect("a native Google stream chunk must pass through")
    else {
        panic!("native Google stream chunks must not be re-serialized");
    };
    assert_eq!(actual, chunk);
}
