/*!
Unit tests for OpenAI request transformations.
*/

use std::{collections::HashMap, fs, path::PathBuf};

use assert_json_diff::assert_json_eq;

use super::transformations::{OpenAIRequestTransformer, TransformError};
use crate::{
    providers::openai::generated::{
        ChatCompletionRequestMessageContent, ChatCompletionRequestMessageRole,
        ChatCompletionToolChoiceOption, CreateChatCompletionRequestClass, FunctionToolChoiceType,
        PurpleType,
    },
    serde_json::{json, Value},
};

fn build_request(payload: Value) -> CreateChatCompletionRequestClass {
    crate::serde_json::from_value(payload).expect("valid request")
}

fn snapshot_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("payloads")
        .join("snapshots")
        .join(relative)
}

fn load_snapshot_value(relative: &str) -> Value {
    let path = snapshot_path(relative);
    let data = fs::read_to_string(path).expect("snapshot readable");
    crate::serde_json::from_str(&data).expect("valid snapshot json")
}

fn load_request_snapshot(relative: &str) -> CreateChatCompletionRequestClass {
    crate::serde_json::from_value(load_snapshot_value(relative)).expect("valid snapshot request")
}

#[test]
fn normalizes_base64_pdf_content_to_file_part() {
    let mut request = build_request(json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": "data:application/pdf;base64,QUJD"
                        }
                    }
                ]
            }
        ]
    }));

    OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect("transform succeeds");

    let parts = match &request.messages[0].content {
        Some(
            ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                parts,
            ),
        ) => parts,
        _ => panic!("expected array content"),
    };

    assert_eq!(
        parts[0].chat_completion_request_message_content_part_type,
        PurpleType::File
    );
    let file = parts[0].file.as_ref().expect("file part");
    assert_eq!(
        file.file_data.as_deref(),
        Some("data:application/pdf;base64,QUJD")
    );
    assert_eq!(file.filename.as_deref(), Some("file_from_base64.pdf"));
}

#[test]
fn reasoning_models_rewrite_limits_and_roles() {
    let mut request = build_request(json!({
        "model": "o1-preview",
        "max_tokens": 128,
        "temperature": 0.5,
        "parallel_tool_calls": true,
        "messages": [
            {
                "role": "system",
                "content": "behave"
            },
            {
                "role": "user",
                "content": "hello"
            }
        ]
    }));

    OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect("transform succeeds");

    assert_eq!(request.max_tokens, None);
    assert_eq!(request.max_completion_tokens, Some(128));
    assert!(request.temperature.is_none());
    assert!(request.parallel_tool_calls.is_none());
    assert!(matches!(
        request.messages[0].role,
        ChatCompletionRequestMessageRole::User
    ));
}

#[test]
fn json_schema_retained_for_models_with_native_support() {
    let mut request = build_request(json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "output",
                "schema": { "type": "object" },
                "strict": true
            }
        }
    }));

    let mut transformer = OpenAIRequestTransformer::new(&mut request);
    transformer.transform().expect("transform succeeds");

    assert!(!transformer.managed_structured_output());
    assert!(request.response_format.is_some());
    assert!(request.tools.is_none());
    assert!(request.tool_choice.is_none());
}

#[test]
fn json_schema_transforms_to_managed_tool_for_other_models() {
    let mut request = build_request(json!({
        "model": "mistral-large-latest",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "output",
                "schema": { "type": "object" },
                "strict": true
            }
        }
    }));

    let mut transformer = OpenAIRequestTransformer::new(&mut request);
    transformer.transform().expect("transform succeeds");

    assert!(transformer.managed_structured_output());
    assert!(request.response_format.is_none());

    let tools = request.tools.as_ref().expect("tools populated");
    assert_eq!(tools.len(), 1);
    let function = tools[0].function.as_ref().expect("function tool");
    assert_eq!(function.name, "json");
    assert_eq!(
        function.description.as_deref(),
        Some("Output the result in JSON format")
    );
    assert_eq!(function.strict, Some(true));
    let parameters = function.parameters.as_ref().expect("parameters");
    let mut expected = HashMap::new();
    expected.insert("type".to_string(), Some(Value::String("object".into())));
    assert_eq!(parameters, &expected);

    match request.tool_choice.as_ref().expect("tool choice set") {
        ChatCompletionToolChoiceOption::FunctionToolChoiceClass(choice) => {
            assert!(choice.allowed_tools.is_none());
            assert_eq!(choice.allowed_tools_type, FunctionToolChoiceType::Function);
            assert_eq!(choice.function.as_ref().unwrap().name, "json");
        }
        other => panic!("unexpected tool choice variant: {:?}", other),
    }
}

#[test]
fn text_response_format_is_removed() {
    let mut request = build_request(json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "response_format": {
            "type": "text"
        }
    }));

    OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect("transform succeeds");

    assert!(request.response_format.is_none());
}

#[test]
fn json_schema_with_existing_tools_returns_error() {
    let mut request = build_request(json!({
        "model": "custom-model",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "tools": [
            {
                "type": "function",
                "function": { "name": "foo" }
            }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "output",
                "schema": { "type": "object" }
            }
        }
    }));

    let err = OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect_err("conflicting tools should error");

    match err {
        TransformError::Unsupported { feature } => {
            assert_eq!(feature, "tools_with_structured_output");
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn reasoning_snapshot_matches_expected() {
    let mut request = load_request_snapshot("transformations/reasoning/request.json");
    let expected = load_snapshot_value("transformations/reasoning/expected.json");

    OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect("transform succeeds");

    let actual = crate::serde_json::to_value(&request).expect("serialize request");
    assert_json_eq!(expected, actual);
}

#[test]
fn structured_output_snapshot_matches_expected() {
    let mut request = load_request_snapshot("transformations/structured_output/request.json");
    let expected = load_snapshot_value("transformations/structured_output/expected.json");

    OpenAIRequestTransformer::new(&mut request)
        .transform()
        .expect("transform succeeds");

    let actual = crate::serde_json::to_value(&request).expect("serialize request");
    assert_json_eq!(expected, actual);
}

#[test]
fn removes_stream_options_for_mistral() {
    use crate::providers::openai::transformations::TargetProvider;

    let mut request = build_request(json!({
        "model": "mistral-large",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "stream_options": {
            "include_usage": true
        }
    }));

    OpenAIRequestTransformer::new(&mut request)
        .with_target_provider(TargetProvider::Mistral)
        .transform()
        .expect("transform succeeds");

    assert!(request.stream_options.is_none());
}

#[test]
fn removes_parallel_tool_calls_for_azure() {
    use crate::providers::openai::transformations::TargetProvider;

    let mut request = build_request(json!({
        "model": "gpt-4",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "parallel_tool_calls": true
    }));

    OpenAIRequestTransformer::new(&mut request)
        .with_target_provider(TargetProvider::Azure)
        .transform()
        .expect("transform succeeds");

    assert!(request.parallel_tool_calls.is_none());
}

#[test]
fn removes_seed_for_azure_with_api_version() {
    use crate::providers::openai::transformations::TargetProvider;
    use crate::serde_json::Map;

    let mut request = build_request(json!({
        "model": "gpt-4",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "seed": 42
    }));

    let mut metadata = Map::new();
    metadata.insert("api_version".to_string(), json!("2023-07-01-preview"));

    OpenAIRequestTransformer::new(&mut request)
        .with_target_provider(TargetProvider::Azure)
        .with_provider_metadata(Some(&metadata))
        .transform()
        .expect("transform succeeds");

    assert!(request.seed.is_none());
}

#[test]
fn keeps_seed_for_azure_without_api_version() {
    use crate::providers::openai::transformations::TargetProvider;

    let mut request = build_request(json!({
        "model": "gpt-4",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "seed": 42
    }));

    OpenAIRequestTransformer::new(&mut request)
        .with_target_provider(TargetProvider::Azure)
        .transform()
        .expect("transform succeeds");

    assert_eq!(request.seed, Some(42));
}

#[test]
fn normalizes_vertex_model_names() {
    use crate::providers::openai::transformations::TargetProvider;

    let test_cases = vec![
        ("publishers/meta/models/llama-2-7b", "meta/llama-2-7b"),
        ("publishers/google/models/gemini-pro", "gemini-pro"),
        ("gemini-pro", "gemini-pro"),
    ];

    for (input_model, expected_model) in test_cases {
        let mut request = build_request(json!({
            "model": input_model,
            "messages": [
                { "role": "user", "content": "hi" }
            ]
        }));

        OpenAIRequestTransformer::new(&mut request)
            .with_target_provider(TargetProvider::Vertex)
            .transform()
            .expect("transform succeeds");

        assert_eq!(
            request.model, expected_model,
            "Failed for input: {}",
            input_model
        );
    }
}

#[test]
fn detects_responses_api_for_pro_models() {
    let pro_models = vec![
        "o1-pro",
        "o1-pro-2024",
        "o3-pro",
        "gpt-5-pro",
        "gpt-5-codex",
    ];

    for model in pro_models {
        let mut request = build_request(json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "hi" }
            ]
        }));

        let mut transformer = OpenAIRequestTransformer::new(&mut request);
        transformer.transform().expect("transform succeeds");

        assert!(
            transformer.use_responses_api(),
            "Failed to detect Responses API for model: {}",
            model
        );
    }
}

#[test]
fn does_not_use_responses_api_for_regular_models() {
    let regular_models = vec!["gpt-4", "gpt-3.5-turbo", "o1-preview"];

    for model in regular_models {
        let mut request = build_request(json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "hi" }
            ]
        }));

        let mut transformer = OpenAIRequestTransformer::new(&mut request);
        transformer.transform().expect("transform succeeds");

        assert!(
            !transformer.use_responses_api(),
            "Should not use Responses API for model: {}",
            model
        );
    }
}

#[test]
fn combined_provider_transformations() {
    use crate::providers::openai::transformations::TargetProvider;

    let mut request = build_request(json!({
        "model": "gpt-4",
        "messages": [
            { "role": "user", "content": "hi" }
        ],
        "stream_options": { "include_usage": true },
        "parallel_tool_calls": true
    }));

    OpenAIRequestTransformer::new(&mut request)
        .with_target_provider(TargetProvider::Databricks)
        .transform()
        .expect("transform succeeds");

    // Databricks doesn't support stream_options or parallel_tool_calls
    assert!(request.stream_options.is_none());
    assert!(request.parallel_tool_calls.is_none());
}
