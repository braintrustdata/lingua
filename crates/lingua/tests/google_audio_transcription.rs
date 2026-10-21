//! Google's hosted transcription pipeline is provider-only: the request-side
//! `audioTranscriptionConfig` and the diarized, word-timestamped
//! `Part.audioTranscription` output survive same-format passthrough and are
//! refused by every transforming path.

use lingua::{
    transform_request, transform_response, transform_stream_chunk, Bytes, ProviderFormat,
    TransformError, TransformResult,
};

const TRANSCRIPTION_CONFIG_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"inlineData":{"mimeType":"audio/wav","data":"UklGRg=="}}]}],"generationConfig":{"audioTranscriptionConfig":{"customVocabulary":["Lingua"],"diarization":true,"languageHints":{"languageCodes":["en-US"]},"mode":"SMART","wordTimestamp":true}}}"#;

const TRANSCRIPTION_RESPONSE: &[u8] = br#"{"candidates":[{"index":0,"content":{"role":"model","parts":[{"audioTranscription":{"speakerLabel":"spk_1","text":"hello there","words":[{"word":"hello","startOffset":"0s","endOffset":"0.4s"}]}}]},"finishReason":"STOP"}],"modelVersion":"gemini-3.5-flash"}"#;

const CROSS_PROVIDER_TARGETS: [ProviderFormat; 3] = [
    ProviderFormat::ChatCompletions,
    ProviderFormat::Anthropic,
    ProviderFormat::Responses,
];

fn assert_unsupported_transcription(error: TransformError) {
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
        message.contains("audioTranscription") || message.contains("audioTranscriptionConfig"),
        "{message}"
    );
}

#[test]
fn google_audio_transcription_request_passes_through_unchanged_for_google_target() {
    let body = Bytes::from_static(TRANSCRIPTION_CONFIG_REQUEST);
    let result = transform_request(body.clone(), ProviderFormat::Google, None)
        .expect("a native Google request passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google request must pass through");
    };
    assert_eq!(actual, body);

    let response = Bytes::from_static(TRANSCRIPTION_RESPONSE);
    let result = transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google response must pass through");
    };
    assert_eq!(actual, response);

    let chunk = Bytes::from_static(TRANSCRIPTION_RESPONSE);
    let result = transform_stream_chunk(chunk.clone(), ProviderFormat::Google)
        .expect("a native Google stream chunk passes through");
    let TransformResult::PassThrough(actual) = result else {
        panic!("a native Google stream chunk must pass through");
    };
    assert_eq!(actual, chunk);
}

#[test]
fn google_audio_transcription_cross_provider_transform_is_explicitly_unsupported() {
    for target in CROSS_PROVIDER_TARGETS {
        let error = transform_request(
            Bytes::from_static(TRANSCRIPTION_CONFIG_REQUEST),
            target,
            None,
        )
        .expect_err("a hosted transcription pipeline has no portable configuration");
        assert_unsupported_transcription(error);

        let error = transform_response(Bytes::from_static(TRANSCRIPTION_RESPONSE), target)
            .expect_err("a diarized transcript part has no portable representation");
        assert_unsupported_transcription(error);

        let error = transform_stream_chunk(Bytes::from_static(TRANSCRIPTION_RESPONSE), target)
            .expect_err("a streamed transcript part has no portable representation");
        assert_unsupported_transcription(error);
    }
}
