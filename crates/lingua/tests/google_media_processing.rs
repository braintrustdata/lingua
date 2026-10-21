//! `Part.mediaProcessing` selects Google-side processing of an attachment, with
//! `AGENTIC` handing the media to provider-driven navigation. It survives
//! same-format passthrough and is refused by every transforming path, because
//! keeping the media while dropping the directive is a silent downgrade.

use lingua::{
    transform_request, Bytes, ProviderFormat, RequestTransformResult, TransformError,
    TransformResult,
};

const AGENTIC_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"inlineData":{"mimeType":"video/mp4","data":"AAAA"},"mediaProcessing":"AGENTIC"},{"text":"What happens at the end?"}]}]}"#;

const STATIC_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"inlineData":{"mimeType":"video/mp4","data":"AAAA"},"mediaProcessing":"STATIC"}]}]}"#;

const CROSS_PROVIDER_TARGETS: [ProviderFormat; 3] = [
    ProviderFormat::ChatCompletions,
    ProviderFormat::Anthropic,
    ProviderFormat::Responses,
];

fn assert_passes_through(payload: &'static [u8]) {
    let body = Bytes::from_static(payload);
    let RequestTransformResult { result, .. } =
        transform_request(body.clone(), ProviderFormat::Google, None)
            .expect("a native Google request passes through");
    let TransformResult::PassThrough(actual) = result else {
        panic!("a native Google request must pass through");
    };
    assert_eq!(actual, body);
}

fn assert_unsupported_media_processing(error: TransformError) {
    assert!(
        matches!(
            error,
            TransformError::ToUniversalFailed(_) | TransformError::FromUniversalFailed(_)
        ),
        "expected a conversion failure category, got {error:?}"
    );
    let message = error.to_string();
    assert!(message.contains("Unsupported mapping"), "{message}");
    assert!(message.contains("mediaProcessing"), "{message}");
}

#[test]
fn google_media_processing_request_passes_through_unchanged_for_google_target() {
    assert_passes_through(AGENTIC_REQUEST);
    assert_passes_through(STATIC_REQUEST);
}

#[test]
fn google_media_processing_cross_provider_transform_is_explicitly_unsupported() {
    for target in CROSS_PROVIDER_TARGETS {
        // The directive is refused for every value: its meaning comes from the
        // contract, not from whether the selected mode happens to look static.
        for payload in [AGENTIC_REQUEST, STATIC_REQUEST] {
            let error = transform_request(Bytes::from_static(payload), target, None)
                .expect_err("provider-side media processing has no portable mapping");
            assert_unsupported_media_processing(error);
        }
    }
}
