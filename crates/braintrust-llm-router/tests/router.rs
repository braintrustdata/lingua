use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use braintrust_llm_router::{
    serde_json::{json, Value},
    AuthConfig, ClientHeaders, Error, ModelCatalog, ModelFlavor, ModelSpec, Provider,
    ProviderFormat, RawResponseStream, RetryPolicy, RouterBuilder,
};
use bytes::Bytes;

/// Helper to create request body bytes from a Value
fn to_body(payload: Value) -> Bytes {
    Bytes::from(braintrust_llm_router::serde_json::to_vec(&payload).unwrap())
}

#[derive(Clone)]
struct StubProvider;

#[async_trait]
impl Provider for StubProvider {
    fn id(&self) -> &'static str {
        "stub"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    async fn complete(
        &self,
        payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<Bytes> {
        // Parse the incoming payload to extract model name
        let value: Value =
            braintrust_llm_router::serde_json::from_slice(&payload).unwrap_or_default();
        let model = value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        // Return raw JSON in OpenAI format as bytes
        let response = json!({
            "id": "stub-response",
            "object": "chat.completion",
            "model": model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": format!("Echo: {}", model)
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2
            }
        });
        let bytes = braintrust_llm_router::serde_json::to_vec(&response)
            .map_err(|e| Error::InvalidRequest(e.to_string()))?;
        Ok(Bytes::from(bytes))
    }

    async fn complete_stream(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Ok(Box::pin(tokio_stream::empty()))
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn router_routes_to_stub_provider() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "stub-model".into(),
        ModelSpec {
            model: "stub-model".into(),
            format: ProviderFormat::OpenAI,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .with_retry_policy(RetryPolicy::default())
        .add_provider("stub", StubProvider)
        .add_auth(
            "stub",
            AuthConfig::ApiKey {
                key: "test".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
        )
        .build()
        .expect("router builds");

    let model = "stub-model";
    let body = to_body(json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "Ping"}
        ]
    }));

    let bytes = router
        .complete(
            body,
            model,
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect("complete");
    // Parse bytes to Value using braintrust_llm_router's serde_json
    let response: Value =
        braintrust_llm_router::serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        response.get("model").and_then(Value::as_str),
        Some("stub-model")
    );
    assert!(response.get("choices").is_some());
}

#[tokio::test]
async fn router_requires_auth_for_provider() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "stub-model".into(),
        ModelSpec {
            model: "stub-model".into(),
            format: ProviderFormat::OpenAI,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .add_provider("stub", StubProvider)
        .build()
        .expect("router builds");

    let model = "stub-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let err = router
        .complete(
            body,
            model,
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect_err("missing auth");
    assert!(matches!(err, Error::NoAuth(alias) if alias == "stub"));
}

#[tokio::test]
async fn router_reports_missing_provider() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "lonely-model".into(),
        ModelSpec {
            model: "lonely-model".into(),
            format: ProviderFormat::OpenAI,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );

    let router = RouterBuilder::new()
        .with_catalog(Arc::new(catalog))
        .build()
        .expect("router builds");

    let model = "lonely-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let err = router
        .complete(
            body,
            model,
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect_err("missing provider");
    assert!(matches!(err, Error::NoProvider(ProviderFormat::OpenAI)));
}

#[tokio::test]
async fn router_propagates_validation_errors() {
    let router = RouterBuilder::new()
        .add_provider("stub", StubProvider)
        .add_auth(
            "stub",
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
        )
        .build()
        .expect("router builds");

    // Empty model should fail validation
    let body = to_body(json!({
        "model": "",
        "messages": []
    }));
    let err = router
        .complete(body, "", ProviderFormat::OpenAI, &ClientHeaders::default())
        .await
        .expect_err("validation");
    // Empty model is treated as unknown model, not invalid request
    assert!(matches!(err, Error::UnknownModel(_)));
}

#[derive(Clone)]
struct FailingProvider {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl Provider for FailingProvider {
    fn id(&self) -> &'static str {
        "failing"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    async fn complete(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<Bytes> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(Error::Timeout)
    }

    async fn complete_stream(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Err(Error::Timeout)
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn router_retries_and_propagates_terminal_error() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "retry-model".into(),
        ModelSpec {
            model: "retry-model".into(),
            format: ProviderFormat::OpenAI,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let attempts = Arc::new(AtomicUsize::new(0));
    let retry_policy = RetryPolicy {
        max_attempts: 2,
        initial_delay: Duration::from_millis(0),
        max_delay: Duration::from_millis(0),
        exponential_base: 2.0,
        jitter: false,
    };

    let router = RouterBuilder::new()
        .with_retry_policy(retry_policy)
        .with_catalog(Arc::clone(&catalog))
        .add_provider(
            "failing",
            FailingProvider {
                attempts: Arc::clone(&attempts),
            },
        )
        .add_auth(
            "failing",
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
        )
        .build()
        .expect("router builds");

    let model = "retry-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let err = router
        .complete(
            body,
            model,
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect_err("terminal error");
    assert!(matches!(err, Error::Timeout));
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[derive(Clone)]
struct PayloadCapturingProvider {
    received: Arc<std::sync::Mutex<Option<Bytes>>>,
}

#[async_trait]
impl Provider for PayloadCapturingProvider {
    fn id(&self) -> &'static str {
        "capturing"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::OpenAI
    }

    async fn complete(
        &self,
        payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<Bytes> {
        *self.received.lock().unwrap() = Some(payload);

        let response = json!({
            "id": "test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "hello"}],
            "model": "test",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });
        Ok(Bytes::from(
            braintrust_llm_router::serde_json::to_vec(&response).unwrap(),
        ))
    }

    async fn complete_stream(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Ok(Box::pin(tokio_stream::empty()))
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn router_supports_multi_format_provider() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "openai-model".into(),
        ModelSpec {
            model: "openai-model".into(),
            format: ProviderFormat::OpenAI,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    catalog.insert(
        "anthropic-model".into(),
        ModelSpec {
            model: "anthropic-model".into(),
            format: ProviderFormat::Anthropic,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    catalog.insert(
        "converse-model".into(),
        ModelSpec {
            model: "converse-model".into(),
            format: ProviderFormat::Converse,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let received = Arc::new(std::sync::Mutex::new(None));
    let provider = PayloadCapturingProvider {
        received: Arc::clone(&received),
    };

    let router = RouterBuilder::new()
        .with_catalog(catalog)
        .add_provider("multi", provider)
        .add_provider_for_format("multi", ProviderFormat::Anthropic)
        .add_provider_for_format("multi", ProviderFormat::Converse)
        .add_auth(
            "multi",
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
        )
        .build()
        .expect("router builds");

    // Send OpenAI-format request targeting an Anthropic-format model.
    // The router should transform the request to Anthropic format using spec.format.
    let body = to_body(json!({
        "model": "anthropic-model",
        "messages": [{"role": "user", "content": "hello"}]
    }));

    let _ = router
        .complete(
            body,
            "anthropic-model",
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await;

    let payload_bytes = received
        .lock()
        .unwrap()
        .take()
        .expect("provider received payload");
    let payload: Value = braintrust_llm_router::serde_json::from_slice(&payload_bytes).unwrap();

    // The payload should have been transformed to Anthropic format
    assert!(
        payload.get("max_tokens").is_some(),
        "payload should be in Anthropic format (has max_tokens)"
    );

    // Now test OpenAI-format model through the same provider
    *received.lock().unwrap() = None;
    let body = to_body(json!({
        "model": "openai-model",
        "messages": [{"role": "user", "content": "hello"}]
    }));

    let _ = router
        .complete(
            body,
            "openai-model",
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await;

    let payload_bytes = received
        .lock()
        .unwrap()
        .take()
        .expect("provider received payload");
    let payload: Value = braintrust_llm_router::serde_json::from_slice(&payload_bytes).unwrap();

    // The payload should pass through as OpenAI format (no max_tokens added by Anthropic transform)
    assert!(
        payload.get("model").is_some(),
        "payload should still be in OpenAI format"
    );

    // Now test Converse-format model through the same provider
    *received.lock().unwrap() = None;
    let body = to_body(json!({
        "model": "converse-model",
        "messages": [{"role": "user", "content": "hello"}]
    }));

    let _ = router
        .complete(
            body,
            "converse-model",
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await;

    let payload_bytes = received
        .lock()
        .unwrap()
        .take()
        .expect("provider received payload");
    let payload: Value = braintrust_llm_router::serde_json::from_slice(&payload_bytes).unwrap();

    // The payload should have been transformed to Converse format
    assert!(
        payload.get("modelId").is_some(),
        "payload should be in Converse format (has modelId)"
    );

    // Now test Converse → Converse passthrough
    *received.lock().unwrap() = None;
    let body = to_body(json!({
        "modelId": "converse-model",
        "messages": [{"role": "user", "content": [{"text": "hello"}]}]
    }));

    let _ = router
        .complete(
            body,
            "converse-model",
            ProviderFormat::Converse,
            &ClientHeaders::default(),
        )
        .await;

    let payload_bytes = received
        .lock()
        .unwrap()
        .take()
        .expect("provider received payload");
    let payload: Value = braintrust_llm_router::serde_json::from_slice(&payload_bytes).unwrap();

    // The payload should pass through as Converse format
    assert!(
        payload.get("modelId").is_some(),
        "Converse passthrough should preserve modelId"
    );
}
