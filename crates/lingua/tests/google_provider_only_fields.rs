//! Offline end-to-end coverage for Google part fields that only Google can execute.
//!
//! `mediaProcessing` selects Google-hosted video navigation, `speechMetadata` drives
//! Google's speech-synthesis engine, and `toolCall`/`toolResponse` are the provider-hosted
//! toolset echo protocol. Google-to-Google traffic must keep them byte for byte, and any
//! cross-provider transform must fail explicitly rather than drop them.

use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn request_bytes(payload: serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&payload).expect("request serializes"))
}

fn media_processing_request() -> serde_json::Value {
    json!({
        "contents": [{
            "role": "user",
            "parts": [
                {"text": "Summarize the clip."},
                {
                    "fileData": {
                        "fileUri": "https://example.com/clip.mp4",
                        "mimeType": "video/mp4"
                    },
                    "mediaProcessing": "AGENTIC"
                }
            ]
        }]
    })
}

fn speech_metadata_request() -> serde_json::Value {
    json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "text": "Welcome aboard.",
                "speechMetadata": {"speaker": "Ana", "style": "cheerful"}
            }]
        }],
        "generationConfig": {
            "speechConfig": {
                "voiceConfig": {"voice": "voicekey_abc123"}
            }
        }
    })
}

fn hosted_tool_call_response() -> serde_json::Value {
    json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "toolCall": {
                        "id": "tool_call_1",
                        "toolName": "web_search",
                        "toolType": "GOOGLE_SEARCH_WEB",
                        "args": {"query": "lingua"}
                    }
                }]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 12,
            "candidatesTokenCount": 5,
            "totalTokenCount": 17
        }
    })
}

fn audio_transcription_request() -> serde_json::Value {
    json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "audioTranscription": {
                    "speakerLabel": "spk_1",
                    "text": "Hello there.",
                    "words": [{"word": "Hello", "startOffset": "0s", "endOffset": "0.5s"}]
                }
            }]
        }]
    })
}

fn audio_transcription_response() -> serde_json::Value {
    json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "audioTranscription": {"speakerLabel": "spk_1", "text": "Hello there."}
                }]
            },
            "finishReason": "STOP"
        }]
    })
}

fn audio_transcription_config_request() -> serde_json::Value {
    json!({
        "contents": [{"role": "user", "parts": [{"text": "Transcribe this audio."}]}],
        "generationConfig": {
            "audioTranscriptionConfig": {"diarization": true, "wordTimestamp": true}
        }
    })
}

fn labels_request() -> serde_json::Value {
    json!({
        "contents": [{"role": "user", "parts": [{"text": "Hello."}]}],
        "labels": {"safety_identifier": "test123"}
    })
}

fn assert_to_universal_failure_mentions(error: TransformError, google_field: &str) {
    assert!(
        matches!(&error, TransformError::ToUniversalFailed(reason) if reason.contains("Unsupported mapping") && reason.contains(google_field)),
        "expected an unsupported mapping naming {google_field}, got: {error:?}"
    );
}

#[test]
fn google_media_processing_request_passes_through_unchanged_for_google() {
    let input = request_bytes(media_processing_request());

    let transformed = lingua::transform_request(input.clone(), ProviderFormat::Google, None)
        .expect("google to google must not fail");

    match transformed.result {
        TransformResult::PassThrough(bytes) => assert_eq!(bytes, input),
        other => panic!("expected byte-preserving passthrough, got {other:?}"),
    }
}

#[test]
fn google_media_processing_request_is_rejected_for_non_google_target() {
    let error = lingua::transform_request(
        request_bytes(media_processing_request()),
        ProviderFormat::Anthropic,
        None,
    )
    .expect_err("mediaProcessing must not be silently dropped");

    assert_to_universal_failure_mentions(error, "mediaProcessing");
}

#[test]
fn google_speech_metadata_request_passes_through_unchanged_for_google() {
    let input = request_bytes(speech_metadata_request());

    let transformed = lingua::transform_request(input.clone(), ProviderFormat::Google, None)
        .expect("google to google must not fail");

    match transformed.result {
        TransformResult::PassThrough(bytes) => assert_eq!(bytes, input),
        other => panic!("expected byte-preserving passthrough, got {other:?}"),
    }
}

#[test]
fn google_speech_metadata_request_is_rejected_for_non_google_target() {
    let error = lingua::transform_request(
        request_bytes(speech_metadata_request()),
        ProviderFormat::Anthropic,
        None,
    )
    .expect_err("speechMetadata must not be flattened into plain text");

    assert_to_universal_failure_mentions(error, "speechMetadata");
}

#[test]
fn google_hosted_tool_call_response_passes_through_unchanged_for_google() {
    let input = request_bytes(hosted_tool_call_response());

    let transformed = lingua::transform_response(input.clone(), ProviderFormat::Google)
        .expect("google to google must not fail");

    match transformed.result {
        TransformResult::PassThrough(bytes) => assert_eq!(bytes, input),
        other => panic!("expected byte-preserving passthrough, got {other:?}"),
    }
}

#[test]
fn google_hosted_tool_call_response_is_rejected_for_non_google_target() {
    let error = lingua::transform_response(
        request_bytes(hosted_tool_call_response()),
        ProviderFormat::ChatCompletions,
    )
    .expect_err("a hosted toolCall turn must not vanish from the transformed response");

    assert_to_universal_failure_mentions(error, "toolCall");
}

#[test]
fn google_hosted_tool_call_stream_is_rejected_for_non_google_target() {
    let error = lingua::transform_stream_chunk(
        request_bytes(hosted_tool_call_response()),
        ProviderFormat::ChatCompletions,
    )
    .expect_err("hosted toolCall stream part must not be dropped");
    assert_to_universal_failure_mentions(error, "toolCall");
}

#[test]
fn google_audio_transcription_parts_preserve_native_bytes_and_reject_cross_provider() {
    let request = request_bytes(audio_transcription_request());
    let native = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("native request passthrough");
    assert!(matches!(native.result, TransformResult::PassThrough(bytes) if bytes == request));
    let error = lingua::transform_request(request, ProviderFormat::Anthropic, None)
        .expect_err("transcript request must be rejected");
    assert_to_universal_failure_mentions(error, "audioTranscription");

    let response = request_bytes(audio_transcription_response());
    let native = lingua::transform_response(response.clone(), ProviderFormat::Google)
        .expect("native response passthrough");
    assert!(matches!(native.result, TransformResult::PassThrough(bytes) if bytes == response));
    let error = lingua::transform_response(response.clone(), ProviderFormat::ChatCompletions)
        .expect_err("transcript response must be rejected");
    assert_to_universal_failure_mentions(error, "audioTranscription");

    let native = lingua::transform_stream_chunk(response.clone(), ProviderFormat::Google)
        .expect("native stream passthrough");
    assert!(matches!(native, TransformResult::PassThrough(bytes) if bytes == response));
    let error = lingua::transform_stream_chunk(response, ProviderFormat::ChatCompletions)
        .expect_err("transcript stream chunk must be rejected");
    assert_to_universal_failure_mentions(error, "audioTranscription");
}

#[test]
fn google_audio_transcription_config_preserves_native_bytes_and_rejects_cross_provider() {
    let request = request_bytes(audio_transcription_config_request());
    let native = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("native config passthrough");
    assert!(matches!(native.result, TransformResult::PassThrough(bytes) if bytes == request));
    let error = lingua::transform_request(request, ProviderFormat::Anthropic, None)
        .expect_err("transcription settings must be rejected");
    assert_to_universal_failure_mentions(error, "audioTranscriptionConfig");
}

#[test]
fn google_request_labels_preserve_native_bytes_and_reject_cross_provider() {
    let request = request_bytes(labels_request());
    let native = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("native labels passthrough");
    assert!(matches!(native.result, TransformResult::PassThrough(bytes) if bytes == request));
    let error = lingua::transform_request(request, ProviderFormat::Anthropic, None)
        .expect_err("labels must be rejected");
    assert_to_universal_failure_mentions(error, "labels");
}
