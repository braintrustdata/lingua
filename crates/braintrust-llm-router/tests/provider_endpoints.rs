use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use braintrust_llm_router::{
    api_key_auth, clear_override_client, create_provider, set_override_client, ClientHeaders,
    ModelSpec, ProviderFormat,
};
use bytes::Bytes;
use futures::StreamExt;
use reqwest::{Request, Response, Url};
use reqwest_middleware::{ClientBuilder, Middleware, Next};

struct CaptureRequests(Arc<Mutex<Vec<Request>>>);

#[async_trait]
impl Middleware for CaptureRequests {
    async fn handle(
        &self,
        request: Request,
        _extensions: &mut http::Extensions,
        _next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        self.0.lock().unwrap().push(request);
        Ok(http::Response::builder()
            .status(200)
            .header("content-type", "text/event-stream")
            .body("data: [DONE]\n\n")
            .unwrap()
            .into())
    }
}

struct OverrideGuard;

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        clear_override_client();
    }
}

// The middleware captures real outgoing requests without opening a network connection.
async fn assert_endpoint(
    kind: &str,
    endpoint: Option<&str>,
    template: Option<&str>,
    expected_url: &str,
) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(CaptureRequests(Arc::clone(&requests)))
        .build();
    set_override_client(client);
    let _guard = OverrideGuard;
    let endpoint = endpoint.map(|url| Url::parse(url).unwrap());
    let provider = create_provider(
        kind,
        endpoint.as_ref(),
        template,
        None,
        &HashMap::new(),
        None,
    )
    .unwrap();
    assert!(provider.matches_provider_alias(&kind.to_ascii_lowercase()));
    let spec: ModelSpec = serde_json::from_str(
        r#"{"model":"qwen3.6-35b-a3b","format":"chat_completions","flavor":"chat"}"#,
    )
    .unwrap();
    let auth = api_key_auth("test-key");
    let headers = ClientHeaders::default();
    let body = Bytes::from_static(br#"{"model":"qwen3.6-35b-a3b","messages":[]}"#);
    provider
        .complete(
            body.clone(),
            &auth,
            &spec,
            ProviderFormat::ChatCompletions,
            &headers,
        )
        .await
        .unwrap();
    let mut stream = provider
        .complete_stream(
            body,
            &auth,
            &spec,
            ProviderFormat::ChatCompletions,
            &headers,
        )
        .await
        .unwrap();
    while let Some(chunk) = stream.next().await {
        chunk.unwrap();
    }

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2, "unary and streaming requests");
    for request in requests.iter() {
        assert_eq!(request.url().as_str(), expected_url, "{kind}");
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(request.headers()["authorization"], "Bearer test-key");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn saladcloud_factory_uses_registered_endpoint() {
    assert_endpoint(
        "saladcloud",
        None,
        None,
        "https://ai.salad.cloud/v1/chat/completions",
    )
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn saladcloud_factory_preserves_explicit_endpoint() {
    assert_endpoint(
        "saladcloud",
        Some("https://custom.example/v1/"),
        None,
        "https://custom.example/v1/chat/completions",
    )
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn saladcloud_factory_preserves_template_precedence() {
    for endpoint in [None, Some("https://custom.example/v1")] {
        assert_endpoint(
            "saladcloud",
            endpoint,
            Some("https://<model>.example/v1/"),
            "https://qwen3.6-35b-a3b.example/v1/chat/completions",
        )
        .await;
    }
}

#[tokio::test]
#[serial_test::serial]
async fn factory_respects_existing_provider_defaults() {
    for (kind, expected_url) in [
        ("openai", "https://api.openai.com/v1/chat/completions"),
        ("groq", "https://api.groq.com/openai/v1/chat/completions"),
        (
            "lepton",
            "https://qwen3.6-35b-a3b.lepton.run/api/v1/chat/completions",
        ),
    ] {
        assert_endpoint(kind, None, None, expected_url).await;
    }
}
