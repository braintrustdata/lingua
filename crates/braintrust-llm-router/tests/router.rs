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

/// Test that a provider can be registered for multiple formats via add_provider_for_format().
/// This enables composite providers like Bedrock that handle both Converse and Anthropic formats.
#[tokio::test]
async fn router_supports_multi_format_provider() {
    let mut catalog = ModelCatalog::empty();
    // Model A uses OpenAI format (primary format of the provider)
    catalog.insert(
        "model-a".into(),
        ModelSpec {
            model: "model-a".into(),
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
    // Model B uses Anthropic format (secondary format via add_provider_for_format)
    catalog.insert(
        "model-b".into(),
        ModelSpec {
            model: "model-b".into(),
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
    let catalog = Arc::new(catalog);

    // StubProvider returns OpenAI format from format()
    // We register it for both OpenAI (via add_provider) and Anthropic (via add_provider_for_format)
    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .add_provider("multi", StubProvider)
        .add_provider_for_format("multi", ProviderFormat::Anthropic)
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

    // Model A (OpenAI format) should route to "multi" provider
    let body = to_body(json!({
        "model": "model-a",
        "messages": [{"role": "user", "content": "Ping"}]
    }));
    let bytes = router
        .complete(
            body,
            "model-a",
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect("model-a should route to multi provider");
    let response: Value =
        braintrust_llm_router::serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        response.get("model").and_then(Value::as_str),
        Some("model-a")
    );

    // Model B (Anthropic format) should ALSO route to "multi" provider
    // This verifies add_provider_for_format() registered the provider for Anthropic format
    let body = to_body(json!({
        "model": "model-b",
        "messages": [{"role": "user", "content": "Ping"}]
    }));
    let bytes = router
        .complete(
            body,
            "model-b",
            ProviderFormat::OpenAI,
            &ClientHeaders::default(),
        )
        .await
        .expect("model-b should also route to multi provider via add_provider_for_format");
    let response: Value =
        braintrust_llm_router::serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        response.get("model").and_then(Value::as_str),
        Some("model-b")
    );
}
