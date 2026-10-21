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
