use crate::capabilities::ProviderFormat;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::AnthropicAdapter;
use crate::serde_json::{self, Value};
use crate::universal::{UniversalRequest, UniversalResponse, UniversalStreamChunk};

const BEDROCK_ANTHROPIC_VERSION: &str = "bedrock-2023-05-31";

/// Adapter for Bedrock's Anthropic invoke envelope (`modelId` + `body`).
///
/// Internally delegates all conversion logic to `AnthropicAdapter` after
/// unwrapping the Bedrock envelope.
pub struct BedrockAnthropicAdapter {
    inner: AnthropicAdapter,
}

impl BedrockAnthropicAdapter {
    pub fn new() -> Self {
        Self {
            inner: AnthropicAdapter,
        }
    }

    fn unwrap_bedrock_invoke_payload(payload: &Value) -> Option<Value> {
        let obj = payload.as_object()?;
        let model_id = obj.get("modelId")?.as_str()?;
        let body = obj.get("body")?;

        let mut inner_payload = match body {
            Value::Object(_) => body.clone(),
            Value::String(s) => serde_json::from_str::<Value>(s).ok()?,
            _ => return None,
        };

        if let Some(inner_obj) = inner_payload.as_object_mut() {
            if !inner_obj.contains_key("model") {
                inner_obj.insert("model".into(), Value::String(model_id.to_string()));
            }
        }

        Some(inner_payload)
    }

    fn convert_to_invoke_body(mut payload: Value) -> Value {
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("model");
            obj.insert(
                "anthropic_version".into(),
                Value::String(BEDROCK_ANTHROPIC_VERSION.into()),
            );
        }
        payload
    }
}

impl Default for BedrockAnthropicAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for BedrockAnthropicAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::BedrockAnthropic
    }

    fn directory_name(&self) -> &'static str {
        "bedrock-anthropic"
    }

    fn display_name(&self) -> &'static str {
        "Bedrock Anthropic"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        Self::unwrap_bedrock_invoke_payload(payload)
            .as_ref()
            .is_some_and(|unwrapped| self.inner.detect_request(unwrapped))
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        let unwrapped = Self::unwrap_bedrock_invoke_payload(&payload).ok_or_else(|| {
            TransformError::ToUniversalFailed(
                "Invalid Bedrock Anthropic request envelope".to_string(),
            )
        })?;
        self.inner.request_to_universal(unwrapped)
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let anthropic_payload = self.inner.request_from_universal(req)?;

        let model_id = anthropic_payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let body = Self::convert_to_invoke_body(anthropic_payload);

        Ok(serde_json::json!({
            "modelId": model_id,
            "body": body
        }))
    }

    fn apply_defaults(&self, req: &mut UniversalRequest) {
        self.inner.apply_defaults(req);
    }

    fn detect_response(&self, payload: &Value) -> bool {
        self.inner.detect_response(payload)
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        self.inner.response_to_universal(payload)
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        self.inner.response_from_universal(resp)
    }

    fn detect_stream_response(&self, payload: &Value) -> bool {
        self.inner.detect_stream_response(payload)
    }

    fn stream_to_universal(
        &self,
        payload: Value,
    ) -> Result<Option<UniversalStreamChunk>, TransformError> {
        self.inner.stream_to_universal(payload)
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        self.inner.stream_from_universal(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::adapters::ProviderAdapter;
    use crate::serde_json::{json, Value};

    #[test]
    fn detect_request_only_matches_bedrock_anthropic_envelope() {
        let adapter = BedrockAnthropicAdapter::new();

        let valid = json!({
            "modelId": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "body": {
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "Hello"}]
            }
        });
        assert!(adapter.detect_request(&valid));

        let plain_anthropic = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!adapter.detect_request(&plain_anthropic));
    }

    #[test]
    fn request_from_universal_outputs_invoke_ready_payload() {
        let adapter = BedrockAnthropicAdapter::new();
        let wrapped = json!({
            "modelId": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "body": {
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "Hello"}]
            }
        });

        let universal = adapter.request_to_universal(wrapped).unwrap();
        let transformed = adapter.request_from_universal(&universal).unwrap();

        assert_eq!(
            transformed.get("modelId").and_then(Value::as_str).unwrap(),
            "us.anthropic.claude-haiku-4-5-20251001-v1:0"
        );
        let body = transformed.get("body").unwrap();
        assert!(body.get("model").is_none());
        assert_eq!(
            body.get("anthropic_version")
                .and_then(Value::as_str)
                .unwrap(),
            "bedrock-2023-05-31"
        );
        assert!(body.get("messages").is_some());
        assert!(body.get("max_tokens").is_some());
    }

    #[test]
    fn convert_to_invoke_body_removes_model_and_sets_version() {
        let payload = json!({
            "model": "us.anthropic.claude-haiku-4-5-20251001-v1:0",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let converted = BedrockAnthropicAdapter::convert_to_invoke_body(payload);
        assert!(converted.get("model").is_none());
        assert_eq!(
            converted
                .get("anthropic_version")
                .and_then(Value::as_str)
                .unwrap(),
            "bedrock-2023-05-31"
        );
    }
}
