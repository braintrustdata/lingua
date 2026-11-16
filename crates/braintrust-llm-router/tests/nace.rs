use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use braintrust_llm_router::{
    create_provider,
    serde_json::{self, json, Value},
    AuthConfig, ClientHeaders, ClientSettings, Error, ModelCatalog, ModelSpec, NaceConfig,
    NaceProvider, OpenAIConfig, OpenAIProvider, Provider, ProviderFormat, RetryPolicy, Router,
};
use bytes::Bytes;
use reqwest::Url;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wiremock::matchers::{body_bytes, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL: &str = "drex-max-v1.1";
const CLIENT: &str = concat!("braintrust/", env!("CARGO_PKG_VERSION"));
const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl6h+QAAAAASUVORK5CYII=";

fn spec() -> ModelSpec {
    serde_json::from_value(json!({
        "model": MODEL,
        "format": "openai",
        "flavor": "decisions",
        "supports_streaming": false,
        "available_providers": ["nace"]
    }))
    .unwrap()
}

fn auth() -> AuthConfig {
    AuthConfig::ApiKey {
        key: "nace-test-key".into(),
        header: Some("authorization".into()),
        prefix: Some("Bearer".into()),
    }
}

fn router(server: &MockServer) -> Router {
    router_at(&server.uri())
}

fn router_at(base_url: &str) -> Router {
    router_with_retries(base_url, 0)
}

fn router_with_retries(base_url: &str, max_attempts: usize) -> Router {
    let mut catalog = ModelCatalog::empty();
    catalog.insert(MODEL.into(), spec());
    let endpoint = Url::parse(&format!("{base_url}/v1/")).unwrap();
    let provider = create_provider(
        "nace",
        Some(&endpoint),
        None,
        None,
        &HashMap::new(),
        Some(ClientSettings {
            pool_max_idle_per_host: 0,
            ..ClientSettings::default()
        }),
    )
    .unwrap();
    assert_eq!(provider.id(), "nace");
    assert_eq!(
        provider.provider_formats(),
        [ProviderFormat::Decisions, ProviderFormat::TypeSafe]
    );
    Router::builder()
        .with_catalog(Arc::new(catalog))
        .with_retry_policy(RetryPolicy {
            max_attempts,
            max_delay: Duration::ZERO,
            ..RetryPolicy::default()
        })
        .add_provider_arc("nace", provider, auth(), vec![])
        // A configured OpenAI provider must never receive a catalogued Drex call.
        .add_provider(
            "openai",
            OpenAIProvider::new(OpenAIConfig {
                endpoint: Url::parse(&format!("{base_url}/wrong-provider")).unwrap(),
                ..OpenAIConfig::default()
            })
            .unwrap(),
            auth(),
            vec![ProviderFormat::ChatCompletions, ProviderFormat::Decisions],
        )
        .build()
        .unwrap()
}

fn body(value: &Value) -> Bytes {
    Bytes::from(serde_json::to_vec(value).unwrap())
}

fn decision_request() -> Value {
    json!({
        "model": MODEL,
        "input": [{"role": "user", "content": [
            {"type": "input_text", "text": "I was charged twice. Please help."},
            {"type": "input_image", "image_url": format!("data:image/png;base64,{PNG}")}
        ]}],
        "questions": [
            {"type": "predicate", "name": "refund", "instructions": "Is a refund requested?"},
            {"type": "choice", "name": "department", "instructions": "Pick a team.",
             "choices": [{"value": "billing"}, {"value": "support"}]},
            {"type": "score", "name": "urgency", "instructions": "Rate urgency.",
             "levels": [{"label": "routine"}, {"label": "urgent"}]}
        ]
    })
}

fn decision_response() -> Value {
    json!({
        "model": MODEL,
        "answers": [
            {"type": "predicate", "name": "refund", "probability": 0.9},
            {"type": "choice", "name": "department", "choice": "billing", "confidence": 0.8,
             "probabilities": [{"value": "billing", "probability": 0.9}, {"value": "support", "probability": 0.1}]},
            {"type": "score", "name": "urgency", "score": 0.75, "confidence": 0.5,
             "probabilities": [{"value": 0, "label": "routine", "probability": 0.25},
                               {"value": 1, "label": "urgent", "probability": 0.75}]}
        ],
        "usage": {
            "input_tokens": 123,
            "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
            "output_tokens": 7,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 130
        }
    })
}

async fn request(
    router: &Router,
    payload: Bytes,
    format: ProviderFormat,
    headers: &ClientHeaders,
) -> braintrust_llm_router::Result<Bytes> {
    let routes = router.resolve_provider_routes(MODEL, format, &[])?;
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].provider_alias(), "nace");
    assert_eq!(routes[0].format(), format);
    let (prepared, metadata) =
        router.create_decision_request(payload, &routes[0], false, format)?;
    assert!(metadata.lingua_passthrough);
    let result = router.complete_decision(prepared, headers).await?;
    assert_eq!(result.response, result.raw_response);
    Ok(result.response)
}

#[tokio::test]
async fn routes_decisions_with_images_and_preserves_answers_order_and_usage() {
    let server = MockServer::start().await;
    let payload = body(&decision_request());
    let response = body(&decision_response());
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .and(header("authorization", "Bearer nace-test-key"))
        .and(header("x-nace-client", CLIENT))
        .and(header("x-request-id", "test-request"))
        .and(body_bytes(payload.to_vec()))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(response.to_vec(), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut headers = ClientHeaders::new();
    headers.insert_if_allowed("authorization", "Bearer incoming-client-key");
    headers.insert_if_allowed("x-nace-client", "spoofed-client");
    headers.insert_if_allowed("x-request-id", "test-request");
    let actual = request(
        &router(&server),
        payload,
        ProviderFormat::Decisions,
        &headers,
    )
    .await
    .unwrap();
    assert_eq!(actual, response);
}

#[tokio::test]
async fn routes_typesafe_without_translation_and_requests_object_legends() {
    let server = MockServer::start().await;
    let payload = body(&json!({
        "model": MODEL,
        "state": "I was charged twice. Please inspect the receipt image.",
        "images": [format!("data:image/png;base64,{PNG}")],
        "questions": {
            "refund": {"type": "noul", "instructions": "Is a refund requested?"},
            "department": {"type": "choice", "instructions": "Pick a team.",
                           "criteria": {"billing": "Payments", "support": "Technical"}},
            "urgency": {"type": "score", "instructions": "Rate urgency.",
                        "criteria": ["routine", "urgent"]}
        }
    }));
    let response = body(&json!({
        "model": MODEL,
        "answers": {
            "refund": {"type": "noul", "noul": 0.9},
            "department": {"type": "choice", "choice": "billing", "confidence": 0.8,
                           "probabilities": {"billing": 0.9, "support": 0.1}},
            "urgency": {"type": "score", "score": 0.75, "confidence": 0.5,
                        "legend": {"0": "routine", "1": "urgent"},
                        "probabilities": {"0": 0.25, "1": 0.75}}
        },
        "usage": {"input_tokens": 123, "output_tokens": 7}
    }));
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .and(header("authorization", "Bearer nace-test-key"))
        .and(header("x-nace-client", CLIENT))
        .and(header("x-nace-legend", "object"))
        .and(body_bytes(payload.to_vec()))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(response.to_vec(), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut headers = ClientHeaders::new();
    headers.insert_if_allowed("x-nace-legend", "array");
    let actual = request(
        &router(&server),
        payload,
        ProviderFormat::TypeSafe,
        &headers,
    )
    .await
    .unwrap();
    assert_eq!(actual, response);
}

#[tokio::test]
async fn preserves_validation_and_rate_limit_errors() {
    for status in [422, 429, 503] {
        let server = MockServer::start().await;
        let response = r#"{"error":{"type":"test_error","message":"Upstream error"}}"#;
        Mock::given(method("POST"))
            .and(path("/v1/decisions"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("retry-after", "7")
                    .insert_header("x-request-id", "upstream-request")
                    .set_body_raw(response, "application/json"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = request(
            &router(&server),
            body(&decision_request()),
            ProviderFormat::Decisions,
            &ClientHeaders::default(),
        )
        .await
        .unwrap_err();
        let expected_delay = (status != 422).then(|| Duration::from_secs(7));
        assert_eq!(error.retry_after(), expected_delay);
        assert_eq!(error.is_retryable(), expected_delay.is_some());
        let Error::Provider { provider, http, .. } = error else {
            panic!("expected upstream provider error, got {error:?}");
        };
        assert_eq!(provider, "nace");
        let http = http.unwrap();
        assert_eq!(http.status, status);
        assert_eq!(http.body, response);
        assert!(http.headers.contains(&("retry-after".into(), "7".into())));
        assert!(http
            .headers
            .contains(&("x-request-id".into(), "upstream-request".into())));
    }
}

#[tokio::test]
async fn preserves_nonstandard_529_capacity_errors() {
    // wiremock's HTTP implementation rejects nonstandard status codes.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            let received = socket.read(&mut buffer).await.unwrap();
            assert!(received > 0, "client closed before sending a request");
            socket.write_all(b"HTTP/1.1 529 Overloaded\r\ncontent-type: application/json\r\ncontent-length: 22\r\nretry-after: 7\r\nconnection: close\r\n\r\n{\"error\":\"overloaded\"}").await.unwrap();
        }
    });
    let error = request(
        &router_with_retries(&format!("http://{address}"), 2),
        body(&decision_request()),
        ProviderFormat::Decisions,
        &ClientHeaders::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
    assert!(error.is_retryable());
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .expect("expected initial request and two retries")
        .unwrap();
    let Error::Provider { provider, http, .. } = error else {
        panic!("expected upstream provider error, got {error:?}");
    };
    assert_eq!(provider, "nace");
    let http = http.unwrap();
    assert_eq!(http.status, 529);
    assert_eq!(http.body, r#"{"error":"overloaded"}"#);
    assert!(http.headers.contains(&("retry-after".into(), "7".into())));
}

#[tokio::test]
async fn retries_overload_responses_then_returns_success() {
    for status in [429, 503] {
        let server = MockServer::start().await;
        let payload = body(&decision_request());
        let response = body(&decision_response());
        Mock::given(method("POST"))
            .and(path("/v1/decisions"))
            .and(body_bytes(payload.to_vec()))
            .respond_with(ResponseTemplate::new(status).insert_header("retry-after", "7"))
            .up_to_n_times(1)
            .with_priority(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/decisions"))
            .and(body_bytes(payload.to_vec()))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(response.to_vec(), "application/json"),
            )
            .with_priority(2)
            .expect(1)
            .mount(&server)
            .await;
        let actual = request(
            &router_with_retries(&server.uri(), 1),
            payload,
            ProviderFormat::Decisions,
            &ClientHeaders::default(),
        )
        .await
        .unwrap();
        assert_eq!(actual, response);
    }
}

#[tokio::test]
async fn preserves_overload_error_when_retry_budget_is_exhausted() {
    let server = MockServer::start().await;
    let response = r#"{"error":"overloaded"}"#;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "7")
                .set_body_raw(response, "application/json"),
        )
        .expect(3)
        .mount(&server)
        .await;
    let error = request(
        &router_with_retries(&server.uri(), 2),
        body(&decision_request()),
        ProviderFormat::Decisions,
        &ClientHeaders::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
    let Error::Provider {
        http: Some(http), ..
    } = error
    else {
        panic!("expected upstream provider error, got {error:?}");
    };
    assert_eq!(http.status, 429);
    assert_eq!(http.body, response);
    assert!(http.headers.contains(&("retry-after".into(), "7".into())));
}

#[tokio::test]
async fn does_not_retry_validation_errors_or_missing_or_invalid_retry_after() {
    for (status, retry_after) in [
        (422, Some("7")),
        (401, Some("7")),
        (429, None),
        (429, Some("invalid")),
        (503, Some("-1")),
    ] {
        let server = MockServer::start().await;
        let mut response = ResponseTemplate::new(status);
        if let Some(value) = retry_after {
            response = response.insert_header("retry-after", value);
        }
        Mock::given(method("POST"))
            .and(path("/v1/decisions"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let error = request(
            &router_with_retries(&server.uri(), 2),
            body(&decision_request()),
            ProviderFormat::Decisions,
            &ClientHeaders::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.retry_after(), None);
        assert!(!error.is_retryable());
    }
}

#[tokio::test]
async fn rejects_chat_and_streaming_without_contacting_upstream() {
    let server = MockServer::start().await;
    let provider = NaceProvider::new(NaceConfig {
        endpoint: Url::parse(&server.uri()).unwrap(),
        ..NaceConfig::default()
    })
    .unwrap();
    let headers = ClientHeaders::default();
    assert!(matches!(
        provider
            .complete(
                Bytes::new(),
                &auth(),
                &spec(),
                ProviderFormat::ChatCompletions,
                &headers
            )
            .await,
        Err(Error::NoProvider(ProviderFormat::ChatCompletions))
    ));
    assert!(matches!(
        provider
            .complete_stream(
                Bytes::new(),
                &auth(),
                &spec(),
                ProviderFormat::Decisions,
                &headers
            )
            .await,
        Err(Error::InvalidRequest(_))
    ));
    let router = router(&server);
    let routes = router
        .resolve_provider_routes(MODEL, ProviderFormat::ChatCompletions, &[])
        .unwrap();
    assert!(router
        .create_request(
            body(&json!({"model": MODEL, "messages": [{"role": "user", "content": "hi"}]})),
            ProviderFormat::ChatCompletions,
            &routes[0],
            false,
        )
        .await
        .is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn health_check_uses_model_listing_without_inference() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer nace-test-key"))
        .and(header("x-nace-client", CLIENT))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .expect(1)
        .mount(&server)
        .await;
    let provider = NaceProvider::new(NaceConfig {
        endpoint: Url::parse(&server.uri()).unwrap(),
        ..NaceConfig::default()
    })
    .unwrap();
    provider.health_check(&auth()).await.unwrap();
}
