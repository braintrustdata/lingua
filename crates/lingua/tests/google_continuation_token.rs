//! Google's continuation protocol is provider-only: the token is an opaque
//! replay handle bound to a byte-identical `contents` replay, so it survives
//! same-format passthrough and is refused by every transforming path.

use lingua::{
    transform_request, transform_response, transform_stream_chunk, Bytes, ProviderFormat,
    TransformError, TransformResult,
};

const CONTINUATION_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"text":"Write a long essay."}]}],"continuationToken":"ct_opaque_handle","generationConfig":{"maxOutputTokens":4096}}"#;

const CONTINUATION_RESPONSE: &[u8] = br#"{"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"Part one"}]},"finishReason":"CONTINUATION","continuationToken":"ct_opaque_handle"}],"modelVersion":"gemini-3.5-flash"}"#;

const CROSS_PROVIDER_TARGETS: [ProviderFormat; 3] = [
    ProviderFormat::ChatCompletions,
    ProviderFormat::Anthropic,
    ProviderFormat::Responses,
];

fn assert_unsupported_continuation(error: TransformError) {
    assert!(
        matches!(
            error,
            TransformError::ToUniversalFailed(_) | TransformError::FromUniversalFailed(_)
        ),
        "expected a conversion failure category, got {error:?}"
    );
    let message = error.to_string();
    assert!(message.contains("Unsupported mapping"), "{message}");
    assert!(
        message.contains("continuationToken") || message.contains("CONTINUATION"),
        "{message}"
    );
}

#[test]
fn google_continuation_token_request_passes_through_unchanged_for_google_target() {
    let body = Bytes::from_static(CONTINUATION_REQUEST);
    let result = transform_request(body.clone(), ProviderFormat::Google, None)
        .expect("a native Google request passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google request must pass through");
    };
    assert_eq!(actual, body);

    let response = Bytes::from_static(CONTINUATION_RESPONSE);
    let result = transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google response must pass through");
    };
    assert_eq!(actual, response);

    let chunk = Bytes::from_static(CONTINUATION_RESPONSE);
    let result = transform_stream_chunk(chunk.clone(), ProviderFormat::Google)
        .expect("a native Google stream chunk passes through");
    let TransformResult::PassThrough(actual) = result else {
        panic!("a native Google stream chunk must pass through");
    };
    assert_eq!(actual, chunk);
}

#[test]
fn google_continuation_token_cross_provider_transform_is_explicitly_unsupported() {
    for target in CROSS_PROVIDER_TARGETS {
        let error = transform_request(Bytes::from_static(CONTINUATION_REQUEST), target, None)
            .expect_err("an opaque continuation handle has no portable mapping");
        assert_unsupported_continuation(error);

        let error = transform_response(Bytes::from_static(CONTINUATION_RESPONSE), target)
            .expect_err("a suspended turn has no portable terminal state");
        assert_unsupported_continuation(error);

        let error = transform_stream_chunk(Bytes::from_static(CONTINUATION_RESPONSE), target)
            .expect_err("a resumable stream is not an ordinary terminal chunk");
        assert_unsupported_continuation(error);
    }
}
