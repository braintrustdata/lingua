#[cfg(test)]
mod tests {
    use lingua::processing::{import_messages_from_spans, Span};
    use lingua::universal::{Message, UserContent};
    use serde_json::{json, Value};

    #[test]
    fn values_move_between_json_backends_without_conversion() {
        let value = json!({
            "text": "café 日本語 🚀",
            "numbers": [i64::MIN, u64::MAX, 1.5, -0.0],
            "nested": [{"null": null, "bool": true}]
        });
        let original = value.to_string();
        let allocation = value
            .as_object()
            .unwrap()
            .get("text")
            .unwrap()
            .as_str()
            .unwrap()
            .as_ptr();
        let lingua_value: lingua::serde_json::Value = value;
        let returned_value: Value = lingua_value;
        assert_eq!(returned_value.to_string(), original);
        assert_eq!(
            returned_value
                .as_object()
                .unwrap()
                .get("text")
                .unwrap()
                .as_str()
                .unwrap()
                .as_ptr(),
            allocation
        );
    }

    #[test]
    fn imports_messages_with_the_standard_json_backend() {
        let messages = import_messages_from_spans(vec![Span {
            input: Some(json!([{"role": "user", "content": "café 日本語 🚀"}])),
            output: None,
            other: serde_json::Map::new(),
        }]);
        assert_eq!(messages.len(), 1);
        let Message::User {
            content: UserContent::String(content),
        } = &messages[0]
        else {
            panic!("expected user text");
        };
        assert_eq!(content, "café 日本語 🚀");
    }
}
