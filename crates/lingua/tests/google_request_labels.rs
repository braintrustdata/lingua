use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn google_request_with_labels() -> Bytes {
    let request = json!({
        "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
        "labels": {"team": "search", "env": "prod"}
    });

    Bytes::from(serde_json::to_vec(&request).expect("request serializes"))
}

fn chat_completions_request_with_labels() -> Bytes {
    let request = json!({
        "model": "gemini-2.5-flash",
        "messages": [{"role": "user", "content": "Hello"}],
        "labels": {"bt_project": "openai-format", "team": "cx"}
    });

    Bytes::from(serde_json::to_vec(&request).expect("request serializes"))
}

#[test]
fn chat_completions_labels_reach_google_request() {
    let result = lingua::transform_request(
        chat_completions_request_with_labels(),
        ProviderFormat::Google,
        None,
    )
    .expect("Chat Completions billing labels should reach Google");

    let TransformResult::Transformed { bytes, .. } = result.result else {
        panic!("expected a Google request transform");
    };
    let actual: serde_json::Value =
        serde_json::from_slice(&bytes).expect("Google request serializes");
    assert_eq!(
        actual,
        json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "model": "gemini-2.5-flash",
            "labels": {"bt_project": "openai-format", "team": "cx"}
        })
    );
}

#[test]
fn chat_completions_labels_survive_native_passthrough() {
    let request = chat_completions_request_with_labels();
    let result = lingua::transform_request(request.clone(), ProviderFormat::ChatCompletions, None)
        .expect("native Chat Completions request must pass through");

    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Chat Completions request must not be re-serialized");
    };
    assert_eq!(actual, request);
}

#[test]
fn google_request_labels_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::ChatCompletions] {
        let error = lingua::transform_request(google_request_with_labels(), target, None)
            .expect_err("provider billing labels have no cross-provider mapping");

        let TransformError::ToUniversalFailed(reason) = &error else {
            panic!("expected a to-universal conversion failure for {target:?}, got {error:?}");
        };
        assert!(
            reason.contains("Unsupported mapping"),
            "rejection must use the unsupported-mapping category: {reason}"
        );
        assert!(
            reason.contains("labels"),
            "rejection must name the field it refuses: {reason}"
        );
    }
}

#[test]
fn google_request_labels_survive_native_passthrough() {
    let request = google_request_with_labels();

    let result = lingua::transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request must still pass through");

    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Google requests must not be re-serialized");
    };
    assert_eq!(actual, request);
}
