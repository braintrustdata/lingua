#![cfg(feature = "bedrock")]

use lingua::processing::import::{
    import_messages_from_values, import_span_with_options, ImportOptions,
};
use lingua::serde_json::{self, json, Value};

fn import(span: Value) -> Value {
    let imported = import_span_with_options(
        serde_json::from_value(span.clone()).unwrap(),
        ImportOptions::default(),
    )
    .unwrap();
    assert!(imported.errors.is_empty(), "{:?}", imported.errors);
    let messages =
        import_messages_from_values(&span["input"], &span["output"], Some(&span["metadata"]));
    let mut structured = imported.input;
    structured.extend(imported.output);
    let messages = serde_json::to_value(messages).unwrap();
    assert_eq!(serde_json::to_value(structured).unwrap(), messages);
    messages
}

#[test]
fn strands_native_text_and_reply() {
    assert_eq!(
        import(json!({
            "input": {"messages": [{"role": "user", "content": [{"text": "Hello"}]}]},
            "output": {"message": {"role": "assistant", "content": [{"text": "Hi"}]}}
        })),
        json!([
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": [{"type": "text", "text": "Hi"}]}
        ])
    );
}

#[test]
fn strands_native_media() {
    for (document, image) in [
        (json!("aGVsbG8="), json!("aGVsbG8=")),
        (
            json!({"type": "braintrust_attachment", "key": "pdf-key", "filename": "test.pdf", "content_type": "application/pdf"}),
            json!({"type": "braintrust_attachment", "key": "image-key", "filename": "test.png", "content_type": "image/png"}),
        ),
    ] {
        let message = json!({"role": "user", "content": [
            {"text": "Read this"},
            {"document": {"format": "pdf", "name": "test", "source": {"bytes": document}}},
            {"image": {"format": "png", "source": {"bytes": image}}}
        ]});
        for input in [json!([message.clone()]), json!({"messages": [message]})] {
            assert_eq!(
                import(json!({"input": input})),
                json!([
                    {"role": "user", "content": [
                        {"type": "text", "text": "Read this"},
                        {"type": "file", "data": document, "filename": "test", "media_type": "application/pdf"},
                        {"type": "image", "image": image, "media_type": "image/png"}
                    ]}
                ])
            );
        }
    }
}

#[test]
fn strands_native_tools() {
    let messages = import(json!({"output": {
        "message": {"role": "assistant", "content": [{"toolUse": {"toolUseId": "call-1", "name": "lookup", "input": {"q": "hello"}}}]},
        "tool_result_message": {"role": "user", "content": [{"toolResult": {"toolUseId": "call-1", "content": [{"text": "Found"}]}}]}
    }}));
    assert_eq!(messages.as_array().unwrap().len(), 2);
    assert_eq!(messages[0]["content"][0]["type"], "tool_call");
    assert_eq!(
        messages[0]["content"][0]["arguments"],
        json!({"type": "valid", "value": {"q": "hello"}})
    );
    assert_eq!(messages[1]["content"][0]["output"], "Found");
}

#[test]
fn strands_system_metadata() {
    for metadata in [
        json!({"system_prompt": "Be brief"}),
        json!({"system_prompt": "Be brief", "system_prompt_content": [{"text": "Be brief"}]}),
        json!({"system_prompt_content": [{"text": "Be brief"}]}),
    ] {
        for system in [None, Some("Use input instruction")] {
            let mut input = vec![json!({"role": "user", "content": [{"text": "Hello"}]})];
            if let Some(system) = system {
                input.insert(0, json!({"role": "system", "content": system}));
            }
            let messages = import(json!({"input": {"messages": input}, "metadata": metadata}));
            assert_eq!(
                messages,
                json!([
                    {"role": "system", "content": system.unwrap_or("Be brief")},
                    {"role": "user", "content": "Hello"}
                ])
            );
        }
    }
}

#[test]
fn strands_unknown_content_is_preserved_as_opaque() {
    let input = json!({"messages": [{"role": "user", "content": [
        {"text": "Keep the whole message"},
        {"unsupported": {"data": "opaque"}}
    ]}]});
    let imported = import_span_with_options(
        serde_json::from_value(json!({"input": input, "span_attributes": {"type": "llm"}}))
            .unwrap(),
        ImportOptions {
            preserve_unsupported: true,
        },
    )
    .unwrap();
    assert!(imported.input.is_empty());
    assert!(!imported.opaque_input.is_empty());
}
