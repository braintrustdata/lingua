use crate::capabilities::ProviderFormat;
use crate::processing::adapters::ProviderAdapter;
use crate::processing::transform::TransformError;
use crate::providers::anthropic::AnthropicAdapter;
use crate::serde_json::{self, Value};
use crate::universal::{
    UniversalRequest, UniversalResponse, UniversalStreamChoice, UniversalStreamChunk,
    UniversalUsage,
};

const BEDROCK_ANTHROPIC_VERSION: &str = "bedrock-2023-05-31";

/// Adapter for Bedrock's Anthropic invoke body format.
///
/// For Bedrock runtime HTTP invoke endpoints, the model is selected by URL path
/// and the request body contains only Anthropic fields.
pub struct BedrockAnthropicAdapter {
    inner: AnthropicAdapter,
}

impl BedrockAnthropicAdapter {
    pub fn new() -> Self {
        Self {
            inner: AnthropicAdapter,
        }
    }

    fn is_raw_invoke_body(payload: &Value) -> bool {
        let Some(obj) = payload.as_object() else {
            return false;
        };
        if obj.contains_key("modelId") || obj.contains_key("body") {
            return false;
        }
        if obj.contains_key("model") {
            return false;
        }
        obj.contains_key("messages")
            && (obj.contains_key("anthropic_version")
                || obj.contains_key("max_tokens")
                || obj.contains_key("system")
                || obj.contains_key("tools"))
    }

    fn convert_to_invoke_body(mut payload: Value) -> Value {
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("model");
            obj.remove("stream");
            obj.remove("stream_options");
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
        if !Self::is_raw_invoke_body(payload) {
            return false;
        }
        self.inner.request_to_universal(payload.clone()).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        if !Self::is_raw_invoke_body(&payload) {
            return Err(TransformError::ToUniversalFailed(
                "Invalid Bedrock Anthropic request format".to_string(),
            ));
        }
        self.inner.request_to_universal(payload)
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let mut request = req.clone();
        request.params.stream = None;
        let anthropic_payload = self.inner.request_from_universal(&request)?;
        Ok(Self::convert_to_invoke_body(anthropic_payload))
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
        // Intercept message_stop with Bedrock invocation metrics
        if payload.get("type").and_then(Value::as_str) == Some("message_stop") {
            if let Some(usage) = usage_from_bedrock_invocation_metrics(&payload) {
                return Ok(Some(UniversalStreamChunk::new(
                    None,
                    None,
                    vec![UniversalStreamChoice {
                        index: 0,
                        delta: Some(serde_json::json!({})),
                        finish_reason: Some("stop".to_string()),
                    }],
                    None,
                    Some(usage),
                )));
            }
        }

        self.inner.stream_to_universal(payload)
    }

    fn stream_from_universal(&self, chunk: &UniversalStreamChunk) -> Result<Value, TransformError> {
        self.inner.stream_from_universal(chunk)
    }
}

fn usage_from_bedrock_invocation_metrics(payload: &Value) -> Option<UniversalUsage> {
    let metrics = payload.get("amazon-bedrock-invocationMetrics")?;
    Some(UniversalUsage {
        prompt_tokens: metrics.get("inputTokenCount").and_then(Value::as_i64),
        completion_tokens: metrics.get("outputTokenCount").and_then(Value::as_i64),
        prompt_cached_tokens: None,
        prompt_cache_creation_tokens: None,
        completion_reasoning_tokens: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::adapters::ProviderAdapter;
    use crate::serde_json::{json, Value};

    #[test]
    fn test_message_stop_with_bedrock_metrics_emits_usage_chunk() {
        let adapter = BedrockAnthropicAdapter::new();
        let payload = json!({
            "type": "message_stop",
            "amazon-bedrock-invocationMetrics": {
                "inputTokenCount": 16,
                "outputTokenCount": 4
            }
        });

        let chunk = adapter
            .stream_to_universal(payload)
            .expect("stream conversion should succeed")
            .expect("message_stop with metrics should emit chunk");

        assert_eq!(
            chunk.usage.as_ref().and_then(|usage| usage.prompt_tokens),
            Some(16)
        );
        assert_eq!(
            chunk
                .usage
                .as_ref()
                .and_then(|usage| usage.completion_tokens),
            Some(4)
        );
        assert_eq!(
            chunk
                .choices
                .first()
                .and_then(|choice| choice.finish_reason.as_deref()),
            Some("stop")
        );
    }

    #[test]
    fn detect_request_accepts_raw_invoke_body() {
        let adapter = BedrockAnthropicAdapter::new();

        let valid = json!({
            "anthropic_version": "bedrock-2023-05-31",
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

        let envelope = json!({"modelId": "m", "body": {}});
        assert!(!adapter.detect_request(&envelope));
    }

    #[test]
    fn request_to_universal_parses_flat_body() {
        let adapter = BedrockAnthropicAdapter::new();
        let raw = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });
        let universal = adapter.request_to_universal(raw).unwrap();
        assert_eq!(universal.model, None);
    }

    #[test]
    fn request_roundtrip_emits_flat_invoke_body() {
        let adapter = BedrockAnthropicAdapter::new();
        let input = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}]
        });

        let mut universal = adapter.request_to_universal(input).unwrap();
        // Model is injected externally (by the router/runner) since it's in the URL path
        universal.model = Some("us.anthropic.claude-haiku-4-5-20251001-v1:0".to_string());
        let transformed = adapter.request_from_universal(&universal).unwrap();

        assert!(transformed.get("model").is_none());
        assert!(transformed.get("stream").is_none());
        assert_eq!(
            transformed
                .get("anthropic_version")
                .and_then(Value::as_str)
                .unwrap(),
            "bedrock-2023-05-31"
        );
        assert!(transformed.get("messages").is_some());
        assert!(transformed.get("max_tokens").is_some());
    }
}
