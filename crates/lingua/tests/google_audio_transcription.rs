use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn to_bytes(value: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).expect("payload serializes"))
}

fn google_request_with_audio_transcription_config() -> Bytes {
    to_bytes(json!({
        "contents": [{"role": "user", "parts": [{"text": "Transcribe the call."}]}],
        "generationConfig": {
            "audioTranscriptionConfig": {"diarization": true, "languageCodes": ["en-US"]}
        }
    }))
}

fn google_response_with_audio_transcription_part() -> Bytes {
    to_bytes(json!({
        "candidates": [{
            "index": 0,
            "content": {
                "role": "model",
                "parts": [{
                    "audioTranscription": {"speakerLabel": "A", "text": "Hello there"}
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
fn google_audio_transcription_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(
            google_request_with_audio_transcription_config(),
            target,
            None,
        )
        .expect_err("hosted transcription config has no cross-provider mapping");
        assert_unsupported(&error, "audioTranscriptionConfig");

        let error =
            lingua::transform_response(google_response_with_audio_transcription_part(), target)
                .expect_err("hosted transcription output has no cross-provider mapping");
        assert_unsupported(&error, "audioTranscription");

        let error =
            lingua::transform_stream_chunk(google_response_with_audio_transcription_part(), target)
                .expect_err("hosted transcription output has no cross-provider stream mapping");
        assert_unsupported(&error, "audioTranscription");
    }
}

#[test]
fn google_audio_transcription_survives_native_passthrough() {
    let request = google_request_with_audio_transcription_config();
    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);

    let response = google_response_with_audio_transcription_part();
    let result = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response must pass through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google responses must not be re-serialized");
    };
    assert_eq!(actual, response);
}
