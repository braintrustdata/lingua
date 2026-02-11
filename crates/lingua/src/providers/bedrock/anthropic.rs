/*!
Bedrock Anthropic invoke endpoint request preparation.

Converts Anthropic-format requests for Bedrock's InvokeModel endpoint,
which natively accepts the Anthropic Messages API format. This avoids
the lossy translation through the Converse API.

Key differences from direct Anthropic API:
- `model` field is removed (model is specified in the URL path)
- `anthropic_version` header is added as a body field
*/

use crate::serde_json::Value;

const BEDROCK_ANTHROPIC_VERSION: &str = "bedrock-2023-05-31";

/// Convert an Anthropic-format request for Bedrock's InvokeModel endpoint.
///
/// Removes the `model` field (model is in the URL path) and adds
/// `anthropic_version` required by the Bedrock invoke endpoint.
pub fn convert_to_anthropic(mut payload: Value) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("model");
        obj.insert(
            "anthropic_version".into(),
            Value::String(BEDROCK_ANTHROPIC_VERSION.into()),
        );
    }
    payload
}

/// Convert an Anthropic-format request for Bedrock's invoke streaming endpoint.
///
/// Same as `convert_to_anthropic` but also ensures `stream: true` is set.
pub fn convert_to_anthropic_stream(mut payload: Value) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("model");
        obj.insert(
            "anthropic_version".into(),
            Value::String(BEDROCK_ANTHROPIC_VERSION.into()),
        );
        obj.insert("stream".into(), Value::Bool(true));
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    #[test]
    fn test_convert_removes_model_adds_version() {
        let input = json!({
            "model": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = convert_to_anthropic(input);

        assert!(result.get("model").is_none());
        assert_eq!(
            result.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
        assert_eq!(result.get("max_tokens").unwrap().as_i64().unwrap(), 1024);
        assert!(result.get("messages").is_some());
    }

    #[test]
    fn test_convert_preserves_all_anthropic_fields() {
        let input = json!({
            "model": "anthropic.claude-sonnet-4-5-20250929-v1:0",
            "max_tokens": 2048,
            "messages": [{"role": "user", "content": "Hello"}],
            "system": "You are helpful.",
            "temperature": 0.7,
            "top_p": 0.9,
            "top_k": 40,
            "stop_sequences": ["STOP"],
            "tools": [{"name": "get_weather", "description": "Get weather", "input_schema": {"type": "object"}}],
            "thinking": {"type": "enabled", "budget_tokens": 5000}
        });

        let result = convert_to_anthropic(input);

        assert!(result.get("model").is_none());
        assert_eq!(
            result.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
        assert_eq!(result.get("max_tokens").unwrap().as_i64().unwrap(), 2048);
        assert!(result.get("messages").is_some());
        assert_eq!(
            result.get("system").unwrap().as_str().unwrap(),
            "You are helpful."
        );
        assert_eq!(
            result.get("temperature").unwrap().as_f64().unwrap(),
            0.7
        );
        assert_eq!(result.get("top_p").unwrap().as_f64().unwrap(), 0.9);
        assert_eq!(result.get("top_k").unwrap().as_i64().unwrap(), 40);
        assert!(result.get("stop_sequences").is_some());
        assert!(result.get("tools").is_some());
        assert!(result.get("thinking").is_some());
    }

    #[test]
    fn test_convert_without_model_field() {
        let input = json!({
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = convert_to_anthropic(input);

        assert!(result.get("model").is_none());
        assert_eq!(
            result.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
    }

    #[test]
    fn test_convert_stream_sets_stream_true() {
        let input = json!({
            "model": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let result = convert_to_anthropic_stream(input);

        assert!(result.get("model").is_none());
        assert_eq!(
            result.get("anthropic_version").unwrap().as_str().unwrap(),
            "bedrock-2023-05-31"
        );
        assert_eq!(result.get("stream").unwrap().as_bool().unwrap(), true);
        assert_eq!(result.get("max_tokens").unwrap().as_i64().unwrap(), 1024);
    }

    #[test]
    fn test_convert_stream_overwrites_existing_stream() {
        let input = json!({
            "model": "anthropic.claude-sonnet-4-5-20250929-v1:0",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": false
        });

        let result = convert_to_anthropic_stream(input);

        assert_eq!(result.get("stream").unwrap().as_bool().unwrap(), true);
    }
}
