use crate::capabilities::ProviderFormat;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::AnthropicAdapter;
use crate::serde_json::Value;
use crate::universal::{UniversalRequest, UniversalResponse, UniversalStreamChunk};
use serde::Deserialize;

const VERTEX_ANTHROPIC_VERSION: &str = "vertex-2023-10-16";

#[derive(Deserialize)]
struct VertexDetectHint {
    #[serde(default)]
    anthropic_version: Option<String>,
    #[serde(default)]
    model: Option<Value>,
    #[serde(default)]
    messages: Option<Value>,
}

/// Adapter for Vertex AI's Anthropic rawPredict body format.
///
/// For Vertex AI rawPredict endpoints, the model is selected by URL path
/// and the request body contains Anthropic fields with `anthropic_version`
/// set to the Vertex-specific version string.
pub struct VertexAnthropicAdapter {
    inner: AnthropicAdapter,
}

impl VertexAnthropicAdapter {
    pub fn new() -> Self {
        Self {
            inner: AnthropicAdapter,
        }
    }

    fn is_raw_vertex_body(payload: &Value) -> bool {
        let Ok(hint) = crate::serde_json::from_value::<VertexDetectHint>(payload.clone()) else {
            return false;
        };
        if hint.model.is_some() {
            return false;
        }
        let has_vertex_version = hint
            .anthropic_version
            .as_deref()
            .is_some_and(|v| v.contains("vertex"));
        hint.messages.is_some() && has_vertex_version
    }

    fn convert_to_vertex_body(mut payload: Value) -> Value {
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("model");
            obj.remove("stream");
            obj.remove("stream_options");
            obj.insert(
                "anthropic_version".into(),
                Value::String(VERTEX_ANTHROPIC_VERSION.into()),
            );
        }
        payload
    }
}

impl Default for VertexAnthropicAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderAdapter for VertexAnthropicAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::VertexAnthropic
    }

    fn directory_name(&self) -> &'static str {
        "vertex-anthropic"
    }

    fn display_name(&self) -> &'static str {
        "Vertex Anthropic"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        if !Self::is_raw_vertex_body(payload) {
            return false;
        }
        self.inner.request_to_universal(payload.clone()).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        if !Self::is_raw_vertex_body(&payload) {
            return Err(TransformError::ToUniversalFailed(
                "Invalid Vertex Anthropic request format".to_string(),
            ));
        }
        self.inner.request_to_universal(payload)
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let mut request = req.clone();
        request.params.stream = None;
        let anthropic_payload = self.inner.request_from_universal(&request)?;
        Ok(Self::convert_to_vertex_body(anthropic_payload))
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
    use crate::serde_json::{json, Map};

    #[test]
    fn detect_request_accepts_raw_vertex_body() {
        let adapter = VertexAnthropicAdapter::new();

        let valid = json!({
            "anthropic_version": "vertex-2023-10-16",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&valid));

        let with_model = json!({
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!adapter.detect_request(&with_model));

        let bedrock_version = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(!adapter.detect_request(&bedrock_version));
    }

    #[test]
    fn request_to_universal_parses_flat_body() {
        let adapter = VertexAnthropicAdapter::new();
        let raw = json!({
            "anthropic_version": "vertex-2023-10-16",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let universal = adapter.request_to_universal(raw).unwrap();
        assert_eq!(universal.model, None);
    }

    #[test]
    fn request_roundtrip_emits_flat_vertex_body() {
        let adapter = VertexAnthropicAdapter::new();
        let input = json!({
            "anthropic_version": "vertex-2023-10-16",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let mut universal = adapter.request_to_universal(input).unwrap();
        universal.model = Some("publishers/anthropic/models/claude-haiku-4-5".to_string());
        let transformed = adapter.request_from_universal(&universal).unwrap();

        let obj: Map<String, Value> = crate::serde_json::from_value(transformed).unwrap();
        assert!(!obj.contains_key("model"));
        assert!(!obj.contains_key("stream"));
        assert_eq!(obj["anthropic_version"], "vertex-2023-10-16");
        assert!(obj.contains_key("messages"));
        assert!(obj.contains_key("max_tokens"));
    }
}
