use lingua::{
    serde_json, serde_json::json, Bytes, ProviderFormat, TransformError, TransformResult,
};

fn responses_request_with_namespaced_tool_output() -> Bytes {
    let request = json!({
        "model": "gpt-5.1",
        "input": [
            {"role": "user", "content": "Which databases exist?"},
            {
                "type": "function_call",
                "call_id": "call_list_databases",
                "name": "list_databases",
                "arguments": "{}"
            },
            {
                "type": "function_call_output",
                "call_id": "call_list_databases",
                "name": "list_databases",
                "namespace": "mongodb",
                "output": "{\"databases\":[\"admin\"]}"
            }
        ]
    });

    Bytes::from(serde_json::to_vec(&request).expect("request serializes"))
}

#[test]
fn openai_function_call_output_namespace_cross_provider_transform_is_unsupported() {
    for target in [ProviderFormat::Anthropic, ProviderFormat::Google] {
        let error = lingua::transform_request(
            responses_request_with_namespaced_tool_output(),
            target,
            None,
        )
        .expect_err("a provider-scoped tool namespace has no cross-provider mapping");

        let TransformError::ToUniversalFailed(reason) = &error else {
            panic!("expected a to-universal conversion failure for {target:?}, got {error:?}");
        };
        assert!(
            reason.contains("Unsupported mapping"),
            "rejection must use the unsupported-mapping category: {reason}"
        );
        assert!(
            reason.contains("namespace") && reason.contains("function_call_output"),
            "rejection must name the field and item kind it refuses: {reason}"
        );
    }
}

#[test]
fn openai_function_call_output_namespace_survives_native_passthrough() {
    let request = responses_request_with_namespaced_tool_output();

    let result = lingua::transform_request(request.clone(), ProviderFormat::Responses, None)
        .expect("a native Responses request must still pass through");

    let TransformResult::PassThrough(actual) = result.result else {
        panic!("native Responses requests must not be re-serialized");
    };
    assert_eq!(actual, request);
}
