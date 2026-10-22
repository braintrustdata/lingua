use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn google_request_with_voice() -> Bytes {
    to_bytes(json!({
        "contents": [{"role": "user", "parts": [{"text": "Read this aloud."}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {"voiceConfig": {"voice": "voice_abc123"}}
        }
    }))
}

fn google_request_with_multi_speaker_voice() -> Bytes {
    to_bytes(json!({
        "contents": [{"role": "user", "parts": [{"text": "Read this dialogue."}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {
                "multiSpeakerVoiceConfig": {
                    "speakerVoiceConfigs": [
                        {"speaker": "Joe", "voiceConfig": {"voice": "voicekey_xyz"}}
                    ]
                }
            }
        }
    }))
}

fn google_request_with_speech_metadata() -> Bytes {
    to_bytes(json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "text": "Say hello cheerfully.",
                "speechMetadata": {"speaker": "Joe", "style": "cheerful"}
            }]
        }]
    }))
}

fn google_response_with_speech_metadata() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {
                "role": "model",
                "parts": [{
                    "text": "Hello!",
                    "speechMetadata": {"speaker": "Joe"}
                }]
            },
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
    assert!(
        reason.contains(field),
        "rejection must name {field}: {reason}"
    );
}

#[test]
fn google_speech_metadata_voice_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        for request in [
            google_request_with_voice(),
            google_request_with_multi_speaker_voice(),
        ] {
            let error = lingua::transform_request(request, target, None)
                .expect_err("provider voice resources have no cross-provider mapping");
            assert_unsupported(&error, "voice");
        }

        let error = lingua::transform_request(google_request_with_speech_metadata(), target, None)
            .expect_err("speech metadata has no cross-provider mapping");
        assert_unsupported(&error, "speechMetadata");

        let error = lingua::transform_response(google_response_with_speech_metadata(), target)
            .expect_err("speech metadata has no cross-provider mapping");
        assert_unsupported(&error, "speechMetadata");

        let error = lingua::transform_stream_chunk(google_response_with_speech_metadata(), target)
            .expect_err("speech metadata has no cross-provider stream mapping");
        assert_unsupported(&error, "speechMetadata");
    }
}

#[test]
fn google_speech_metadata_voice_survives_native_passthrough() {
    for request in [
        google_request_with_voice(),
        google_request_with_multi_speaker_voice(),
        google_request_with_speech_metadata(),
    ] {
        let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
            .expect("a native Google request must pass through");
        let TransformResult::PassThrough(actual) = result.result else {
            panic!("native Google requests must not be re-serialized");
        };
        assert_eq!(actual, request);
    }

    let response = google_response_with_speech_metadata();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}
