use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
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
        _format: ProviderFormat,
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
            format: ProviderFormat::ChatCompletions,
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
            available_providers: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .with_retry_policy(RetryPolicy::default())
        .add_provider(
            "stub",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
            vec![ProviderFormat::ChatCompletions],
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

    let bytes: Bytes = router
        .complete(
            body,
            model,
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await
        .expect("complete")
        .output;
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
            format: ProviderFormat::ChatCompletions,
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
            available_providers: Default::default(),
        },
    );
    let catalog = Arc::new(catalog);

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .add_provider(
            "stub",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");

    let model = "stub-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let bytes: Bytes = router
        .complete(
            body,
            model,
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await
        .expect("complete succeeds when auth is configured")
        .output;
    let response: Value =
        braintrust_llm_router::serde_json::from_slice(&bytes).expect("valid json");
    assert_eq!(
        response.get("model").and_then(Value::as_str),
        Some("stub-model")
    );
}

#[tokio::test]
async fn router_reports_missing_provider() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "lonely-model".into(),
        ModelSpec {
            model: "lonely-model".into(),
            format: ProviderFormat::ChatCompletions,
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
            available_providers: Default::default(),
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
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await
        .expect_err("missing provider");
    let (err, _attempted_aliases) = err.into_parts();
    assert!(matches!(
        err,
        Error::NoProvider(ProviderFormat::ChatCompletions)
    ));
}

#[tokio::test]
async fn router_propagates_validation_errors() {
    let router = RouterBuilder::new()
        .with_catalog(Arc::new(ModelCatalog::empty()))
        .add_provider(
            "stub",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");

    // Empty model should fail validation
    let body = to_body(json!({
        "model": "",
        "messages": []
    }));
    let err = router
        .complete(
            body,
            "",
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await;
    let err = err.expect_err("validation");
    let (err, _attempted_aliases) = err.into_parts();
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

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
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
        _format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Err(Error::Timeout)
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn router_falls_back_to_second_provider_and_reports_aliases() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "fallback-model".into(),
        ModelSpec {
            model: "fallback-model".into(),
            format: ProviderFormat::ChatCompletions,
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
            available_providers: vec!["failing".into(), "stub".into()],
        },
    );
    let catalog = Arc::new(catalog);

    let attempts = Arc::new(AtomicUsize::new(0));

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .add_provider(
            "failing",
            FailingProvider {
                attempts: Arc::clone(&attempts),
            },
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
            vec![],
        )
        .add_provider(
            "stub",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
            vec![],
        )
        .build()
        .expect("router builds");

    let model = "fallback-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let routed_response = router
        .complete(
            body,
            model,
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await;
    let routed_response = routed_response.expect("fallback succeeds");
    let response: Value =
        braintrust_llm_router::serde_json::from_slice(&routed_response.output).expect("valid json");

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(routed_response.provider_alias, "stub");
    assert_eq!(routed_response.attempted_aliases, vec!["failing", "stub"]);
    assert_eq!(response.get("model").and_then(Value::as_str), Some(model));
}
