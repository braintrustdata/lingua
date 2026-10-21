//! `Part.toolCall` is Google's server-side tool invocation echo protocol, and
//! `toolName` identifies the provider-owned tool. It survives same-format
//! passthrough and is refused by every transforming path, while caller-defined
//! `functionCall` parts stay portable.

use lingua::{
    transform_request, transform_response, Bytes, ProviderFormat, TransformError, TransformResult,
};

const TOOL_CALL_RESPONSE: &[u8] = br#"{"candidates":[{"index":0,"content":{"role":"model","parts":[{"toolCall":{"id":"tc_1","toolName":"google_search_web","toolType":"GOOGLE_SEARCH_WEB","args":{"query":"weather"}}}]},"finishReason":"STOP"}],"modelVersion":"gemini-3.5-flash"}"#;

const TOOL_CALL_ECHO_REQUEST: &[u8] = br#"{"contents":[{"role":"user","parts":[{"text":"What is the weather?"}]},{"role":"model","parts":[{"toolCall":{"id":"tc_1","toolName":"google_search_web","toolType":"GOOGLE_SEARCH_WEB","args":{"query":"weather"}}}]}]}"#;

const FUNCTION_CALL_RESPONSE: &[u8] = br#"{"candidates":[{"index":0,"content":{"role":"model","parts":[{"functionCall":{"name":"get_weather","args":{"city":"SF"}}}]},"finishReason":"STOP"}],"modelVersion":"gemini-3.5-flash"}"#;

const CROSS_PROVIDER_TARGETS: [ProviderFormat; 3] = [
    ProviderFormat::ChatCompletions,
    ProviderFormat::Anthropic,
    ProviderFormat::Responses,
];

fn assert_unsupported_tool_call(error: TransformError) {
    assert!(
        matches!(
            error,
            TransformError::ToUniversalFailed(_) | TransformError::FromUniversalFailed(_)
        ),
        "expected a conversion failure category, got {error:?}"
    );
    let message = error.to_string();
    assert!(message.contains("Unsupported mapping"), "{message}");
    assert!(message.contains("toolCall"), "{message}");
}

#[test]
fn google_server_tool_call_response_passes_through_unchanged_for_google_target() {
    let response = Bytes::from_static(TOOL_CALL_RESPONSE);
    let result = transform_response(response.clone(), ProviderFormat::Google)
        .expect("a native Google response passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google response must pass through");
    };
    assert_eq!(actual, response);

    let request = Bytes::from_static(TOOL_CALL_ECHO_REQUEST);
    let result = transform_request(request.clone(), ProviderFormat::Google, None)
        .expect("a native Google request passes through");
    let TransformResult::PassThrough(actual) = result.result else {
        panic!("a native Google request must pass through");
    };
    assert_eq!(actual, request);
}

#[test]
fn google_server_tool_call_cross_provider_transform_is_explicitly_unsupported() {
    for target in CROSS_PROVIDER_TARGETS {
        let error = transform_response(Bytes::from_static(TOOL_CALL_RESPONSE), target)
            .expect_err("a provider-executed tool invocation has no portable mapping");
        assert_unsupported_tool_call(error);

        let error = transform_request(Bytes::from_static(TOOL_CALL_ECHO_REQUEST), target, None)
            .expect_err("an echoed provider-executed invocation has no portable mapping");
        assert_unsupported_tool_call(error);
    }
}

#[test]
fn google_caller_defined_function_call_still_transforms_cross_provider() {
    for target in CROSS_PROVIDER_TARGETS {
        let result = transform_response(Bytes::from_static(FUNCTION_CALL_RESPONSE), target)
            .expect("caller-defined function tools remain portable");
        assert!(matches!(result.result, TransformResult::Transformed { .. }));
    }
}
