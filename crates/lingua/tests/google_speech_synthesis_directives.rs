//! `Part.speechMetadata` and `VoiceConfig.voice` drive Google's hosted
//! speech-synthesis stack, and the voice field also accepts opaque
//! provider-scoped handles. Both survive same-format passthrough and are
//! refused uniformly by every transforming path.

use lingua::{
    transform_request, transform_response, Bytes, ProviderFormat, TransformError, TransformResult,
};

const SPEECH_METADATA_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"text":"Welcome aboard.","speechMetadata":{"speaker":"Narrator","style":"calm"}}]}]}"#;

const PREBUILT_VOICE_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"text":"Read this aloud."}]}],"generationConfig":{"speechConfig":{"voiceConfig":{"voice":"Orus"}}}}"#;

const STORED_VOICE_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"text":"Read this aloud."}]}],"generationConfig":{"speechConfig":{"voiceConfig":{"voice":"voicekey_xxx"}}}}"#;

const SPEECH_METADATA_RESPONSE: &[u8] = br#"{"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"Welcome aboard.","speechMetadata":{"speaker":"Narrator","style":"calm"}}]},"finishReason":"STOP"}]}"#;

const CROSS_PROVIDER_TARGETS: [ProviderFormat; 3] = [
    ProviderFormat::ChatCompletions,
    ProviderFormat::Anthropic,
    ProviderFormat::Responses,
];

const SPEECH_REQUESTS: [&[u8]; 3] = [
    SPEECH_METADATA_REQUEST,
    PREBUILT_VOICE_REQUEST,
    STORED_VOICE_REQUEST,
];

fn assert_unsupported_speech_directive(error: TransformError) {
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
        message.contains("speechMetadata") || message.contains("voiceConfig.voice"),
        "{message}"
    );
}

#[test]
fn google_speech_directives_request_passes_through_unchanged_for_google_target() {
    for payload in SPEECH_REQUESTS {
        let body = Bytes::from_static(payload);
        let result = transform_request(body.clone(), ProviderFormat::Google, None)
            .expect("a native Google request passes through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("a native Google request must pass through");
        };
        assert_eq!(actual, body);
    }

    let response = Bytes::from_static(SPEECH_METADATA_RESPONSE);
    let result = transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google response must pass through");
    };
    assert_eq!(actual, response);
}

#[test]
fn google_speech_directives_cross_provider_transform_is_explicitly_unsupported() {
    for target in CROSS_PROVIDER_TARGETS {
        // Prebuilt names and opaque voice handles are refused the same way; the
        // decision must not depend on sniffing the identifier's shape.
        for payload in SPEECH_REQUESTS {
            let error = transform_request(Bytes::from_static(payload), target, None)
                .expect_err("hosted speech synthesis has no portable mapping");
            assert_unsupported_speech_directive(error);
        }

        let error = transform_response(Bytes::from_static(SPEECH_METADATA_RESPONSE), target)
            .expect_err("a per-part synthesis directive has no portable mapping");
        assert_unsupported_speech_directive(error);
    }
}
