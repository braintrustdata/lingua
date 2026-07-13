use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use braintrust_llm_router::{
    serde_json::{json, Value},
    AuthConfig, ClientHeaders, Error, ModelCatalog, ModelFlavor, ModelSpec, Provider,
    ProviderFormat, RawResponseStream, RetryPolicy, Router, RouterBuilder,
};
use bytes::Bytes;

/// Helper to create request body bytes from a Value
fn to_body(payload: Value) -> Bytes {
    Bytes::from(braintrust_llm_router::serde_json::to_vec(&payload).unwrap())
}

async fn create_request(
    router: &Router,
    body: Bytes,
    model: &str,
    output_format: ProviderFormat,
) -> braintrust_llm_router::Result<(
    braintrust_llm_router::PreparedRequest,
    braintrust_llm_router::RouterMetadata,
)> {
    let routes = router.resolve_provider_routes(model, output_format, &[])?;
    let route = routes
        .first()
        .ok_or_else(|| Error::NoProvider(output_format))?;
    router
        .create_request(body, output_format, route, false)
        .await
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

    let (request, _metadata) =
        create_request(&router, body, model, ProviderFormat::ChatCompletions)
            .await
            .expect("create request");
    let bytes: Bytes = router
        .complete(request, &ClientHeaders::default())
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
async fn router_resolves_provider_alias_in_metadata() {
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

    let body = to_body(json!({
        "model": "stub-model",
        "messages": [{"role": "user", "content": "Ping"}]
    }));
    let (_request, metadata) =
        create_request(&router, body, "stub-model", ProviderFormat::ChatCompletions)
            .await
            .expect("create request");

    assert_eq!(metadata.provider_alias, "stub");
}

#[test]
fn fallback_alias_resolution_skips_aliases_not_eligible_for_model() {
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
            available_providers: vec!["eligible".to_string()],
        },
    );
    let catalog = Arc::new(catalog);

    let router = RouterBuilder::new()
        .with_catalog(Arc::clone(&catalog))
        .with_retry_policy(RetryPolicy::default())
        .add_provider(
            "eligible",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
            vec![ProviderFormat::ChatCompletions],
        )
        .add_provider(
            "ineligible",
            StubProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
            vec![],
        )
        .build()
        .expect("router builds");

    let routes = router
        .resolve_provider_routes(
            "stub-model",
            ProviderFormat::ChatCompletions,
            &["ineligible".to_string(), "eligible".to_string()],
        )
        .expect("fallback aliases resolve");

    let aliases: Vec<&str> = routes.iter().map(|route| route.provider_alias()).collect();
    assert_eq!(aliases, vec!["eligible"]);
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

    let (request, _metadata) =
        create_request(&router, body, model, ProviderFormat::ChatCompletions)
            .await
            .expect("create request");
    let bytes: Bytes = router
        .complete(request, &ClientHeaders::default())
        .await
        .expect("complete succeeds when auth is configured");
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

    let err = match create_request(&router, body, model, ProviderFormat::ChatCompletions).await {
        Ok(_) => panic!("missing provider"),
        Err(err) => err,
    };
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
    let err: braintrust_llm_router::Result<_> =
        create_request(&router, body, "", ProviderFormat::ChatCompletions).await;
    let err = match err {
        Ok(_) => panic!("validation"),
        Err(err) => err,
    };
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

#[derive(Clone)]
struct HttpFailingProvider;

#[async_trait]
impl Provider for HttpFailingProvider {
    fn id(&self) -> &'static str {
        "http-failing"
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
        let err = reqwest::Client::new()
            .get("http://127.0.0.1:1")
            .send()
            .await
            .expect_err("connection failure");
        Err(err.into())
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

#[derive(Clone)]
struct MiddlewareFailingProvider;

#[async_trait]
impl Provider for MiddlewareFailingProvider {
    fn id(&self) -> &'static str {
        "middleware-failing"
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
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();
        let err = client
            .get("http://127.0.0.1:1")
            .send()
            .await
            .expect_err("connection failure");
        Err(err.into())
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

/// A stub that pretends to be the native Anthropic provider and records the format
/// it was called with, so tests can assert on resolve_provider's format selection.
#[derive(Clone)]
struct CapturingAnthropicStub {
    recorded_format: Arc<Mutex<Option<ProviderFormat>>>,
}

#[async_trait]
impl Provider for CapturingAnthropicStub {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Anthropic, ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<Bytes> {
        *self.recorded_format.lock().unwrap() = Some(format);
        let response = braintrust_llm_router::serde_json::json!({
            "id": "stub",
            "object": "chat.completion",
            "model": "claude-test",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
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
        _format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Ok(Box::pin(tokio_stream::empty()))
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

fn anthropic_router(catalog: Arc<ModelCatalog>) -> (Router, Arc<Mutex<Option<ProviderFormat>>>) {
    let recorded_format = Arc::new(Mutex::new(None));
    let stub = CapturingAnthropicStub {
        recorded_format: Arc::clone(&recorded_format),
    };
    let router = RouterBuilder::new()
        .with_catalog(catalog)
        .add_provider(
            "anthropic",
            stub,
            AuthConfig::ApiKey {
                key: "test-key".into(),
                header: Some("x-api-key".into()),
                prefix: None,
            },
            vec![ProviderFormat::Anthropic, ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");
    (router, recorded_format)
}

#[tokio::test]
async fn anthropic_openai_catalog_format_resolves_to_chat_completions() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "my-openai-anthropic-model".into(),
        ModelSpec {
            model: "claude-haiku-4-5".into(),
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
    let (router, recorded_format) = anthropic_router(Arc::new(catalog));

    let body = to_body(braintrust_llm_router::serde_json::json!({
        "model": "my-openai-anthropic-model",
        "messages": [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "What is 1+1?"}
        ]
    }));
    let (request, _metadata) = create_request(
        &router,
        body,
        "my-openai-anthropic-model",
        ProviderFormat::ChatCompletions,
    )
    .await
    .expect("create request");
    router
        .complete(request, &ClientHeaders::default())
        .await
        .expect("complete");

    assert_eq!(
        *recorded_format.lock().unwrap(),
        Some(ProviderFormat::ChatCompletions),
        "custom model with catalog_format=openai should use ChatCompletions to hit /chat/completions"
    );
}

#[tokio::test]
async fn anthropic_native_catalog_format_resolves_to_anthropic() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "claude-haiku-4-5".into(),
        ModelSpec {
            model: "claude-haiku-4-5".into(),
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
            available_providers: Default::default(),
        },
    );
    let (router, recorded_format) = anthropic_router(Arc::new(catalog));

    let body = to_body(braintrust_llm_router::serde_json::json!({
        "model": "claude-haiku-4-5",
        "messages": [{"role": "user", "content": "What is 1+1?"}]
    }));
    let (request, _metadata) = create_request(
        &router,
        body,
        "claude-haiku-4-5",
        ProviderFormat::ChatCompletions,
    )
    .await
    .expect("create request");
    router
        .complete(request, &ClientHeaders::default())
        .await
        .expect("complete");

    assert_eq!(
        *recorded_format.lock().unwrap(),
        Some(ProviderFormat::Anthropic),
        "standard Anthropic model should still use Anthropic format to hit /v1/messages"
    );
}

/// Stub provider that supports both ChatCompletions and Responses formats and
/// records the format it was called with.
#[derive(Clone)]
struct CapturingOpenAIStub {
    recorded_format: Arc<Mutex<Option<ProviderFormat>>>,
}

#[async_trait]
impl Provider for CapturingOpenAIStub {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses]
    }

    async fn complete(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<Bytes> {
        *self.recorded_format.lock().unwrap() = Some(format);
        let response = braintrust_llm_router::serde_json::json!({
            "id": "stub",
            "object": "chat.completion",
            "model": "gpt-5.4-mini",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
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
        _format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> braintrust_llm_router::Result<RawResponseStream> {
        Ok(Box::pin(tokio_stream::empty()))
    }

    async fn health_check(&self, _auth: &AuthConfig) -> braintrust_llm_router::Result<()> {
        Ok(())
    }
}

fn openai_router(catalog: Arc<ModelCatalog>) -> (Router, Arc<Mutex<Option<ProviderFormat>>>) {
    let recorded_format = Arc::new(Mutex::new(None));
    let stub = CapturingOpenAIStub {
        recorded_format: Arc::clone(&recorded_format),
    };
    let router = RouterBuilder::new()
        .with_catalog(catalog)
        .add_provider(
            "openai",
            stub,
            AuthConfig::ApiKey {
                key: "test-key".into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
            vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
        )
        .build()
        .expect("router builds");
    (router, recorded_format)
}

#[tokio::test]
async fn reasoning_effort_with_tools_upgrades_format_to_responses() {
    // Use gpt-5.2-mini (minor version 2 < 3) so model_requires_responses_api() returns
    // false. Only the body-level detection (reasoning_effort + tools) should trigger the
    // upgrade from ChatCompletions to Responses.
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "gpt-5.2-mini".into(),
        ModelSpec {
            model: "gpt-5.2-mini".into(),
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
    let (router, recorded_format) = openai_router(Arc::new(catalog));

    let body = to_body(json!({
        "model": "gpt-5.2-mini",
        "messages": [{"role": "user", "content": "Tokyo weather?"}],
        "reasoning_effort": "medium",
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {
                    "type": "object",
                    "properties": {"location": {"type": "string"}},
                    "required": ["location"]
                }
            }
        }]
    }));

    let (request, metadata) = create_request(
        &router,
        body,
        "gpt-5.2-mini",
        ProviderFormat::ChatCompletions,
    )
    .await
    .expect("create request");

    assert_eq!(
        metadata.provider_format,
        ProviderFormat::Responses,
        "provider_format in metadata should reflect the upgrade to Responses"
    );

    router
        .complete(request, &ClientHeaders::default())
        .await
        .expect("complete");

    assert_eq!(
        *recorded_format.lock().unwrap(),
        Some(ProviderFormat::Responses),
        "provider.complete() must be called with Responses so the request hits /v1/responses"
    );
}

#[tokio::test]
async fn responses_required_model_uses_responses_for_anthropic_messages_output() {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "gpt-5.5".into(),
        ModelSpec {
            model: "gpt-5.5".into(),
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
    let (router, _recorded_format) = openai_router(Arc::new(catalog));

    let body = to_body(json!({
        "model": "gpt-5.5",
        "max_tokens": 1024,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let routes = router
        .resolve_provider_routes("gpt-5.5", ProviderFormat::Anthropic, &[])
        .expect("resolve routes");
    let route = routes.first().expect("route");
    let (_request, metadata) = router
        .create_request(body, ProviderFormat::Anthropic, route, false)
        .await
        .expect("create request");

    assert_eq!(
        metadata.detected_input_format,
        ProviderFormat::Anthropic,
        "request should be detected as Anthropic messages input"
    );
    assert_eq!(
        metadata.provider_format,
        ProviderFormat::Responses,
        "gpt-5.5 should use Responses transport even when the caller uses /v1/messages"
    );
}

fn retry_model_catalog() -> Arc<ModelCatalog> {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(
        "retry-model".into(),
        ModelSpec {
            model: "retry-model".into(),
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
    Arc::new(catalog)
}

#[tokio::test]
async fn router_does_not_retry_timeout_errors() {
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
        .with_catalog(retry_model_catalog())
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
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");

    let model = "retry-model";
    let body = to_body(json!({
        "model": model,
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let (request, _metadata) =
        create_request(&router, body, model, ProviderFormat::ChatCompletions)
            .await
            .expect("create request");
    let err: braintrust_llm_router::Result<Bytes> =
        router.complete(request, &ClientHeaders::default()).await;
    let err = err.expect_err("terminal error");
    assert!(matches!(err, Error::Timeout));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn router_maps_terminal_http_errors_to_upstream_unavailable() {
    let retry_policy = RetryPolicy {
        max_attempts: 0,
        initial_delay: Duration::from_millis(0),
        max_delay: Duration::from_millis(0),
        exponential_base: 2.0,
        jitter: false,
    };

    let router = RouterBuilder::new()
        .with_retry_policy(retry_policy)
        .with_catalog(retry_model_catalog())
        .add_provider(
            "http-failing",
            HttpFailingProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");

    let body = to_body(json!({
        "model": "retry-model",
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let (request, _metadata) = create_request(
        &router,
        body,
        "retry-model",
        ProviderFormat::ChatCompletions,
    )
    .await
    .expect("create request");
    let err = router
        .complete(request, &ClientHeaders::default())
        .await
        .expect_err("terminal error");

    match err {
        Error::UpstreamUnavailable { provider, .. } => {
            assert_eq!(provider, "http-failing");
        }
        other => panic!("expected UpstreamUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn router_maps_terminal_middleware_errors_to_upstream_unavailable() {
    let retry_policy = RetryPolicy {
        max_attempts: 0,
        initial_delay: Duration::from_millis(0),
        max_delay: Duration::from_millis(0),
        exponential_base: 2.0,
        jitter: false,
    };

    let router = RouterBuilder::new()
        .with_retry_policy(retry_policy)
        .with_catalog(retry_model_catalog())
        .add_provider(
            "middleware-failing",
            MiddlewareFailingProvider,
            AuthConfig::ApiKey {
                key: "test".into(),
                header: None,
                prefix: None,
            },
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .expect("router builds");

    let body = to_body(json!({
        "model": "retry-model",
        "messages": [{"role": "user", "content": "Ping"}]
    }));

    let (request, _metadata) = create_request(
        &router,
        body,
        "retry-model",
        ProviderFormat::ChatCompletions,
    )
    .await
    .expect("create request");
    let err = router
        .complete(request, &ClientHeaders::default())
        .await
        .expect_err("terminal error");

    match err {
        Error::UpstreamUnavailable { provider, .. } => {
            assert_eq!(provider, "middleware-failing");
        }
        other => panic!("expected UpstreamUnavailable, got {other:?}"),
    }
}
