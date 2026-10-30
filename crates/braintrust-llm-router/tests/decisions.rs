use std::sync::Arc;

use braintrust_llm_router::{
    serde_json::{self, json, Value},
    AuthConfig, ClientHeaders, Error, ModelCatalog, ModelFlavor, ModelSpec, OpenAIConfig,
    OpenAIProvider, ProviderFormat, RetryPolicy, Router, RouterBuilder, TypeSafeConfig,
    TypeSafeProvider,
};
use bytes::Bytes;
use reqwest::Url;
use wiremock::matchers::{body_json, body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn spec(model: &str, format: ProviderFormat, flavor: ModelFlavor, provider: &str) -> ModelSpec {
    ModelSpec {
        model: model.into(),
        format,
        flavor,
        display_name: None,
        parent: None,
        input_cost_per_mil_tokens: None,
        output_cost_per_mil_tokens: None,
        input_cache_read_cost_per_mil_tokens: None,
        multimodal: None,
        reasoning: None,
        max_input_tokens: None,
        max_output_tokens: None,
        supports_streaming: false,
        extra: Default::default(),
        available_providers: vec![provider.into()],
    }
}

fn auth(key: &str) -> AuthConfig {
    AuthConfig::ApiKey {
        key: key.into(),
        header: Some("authorization".into()),
        prefix: Some("Bearer".into()),
    }
}

fn router(server: &MockServer) -> Router {
    let mut catalog = ModelCatalog::empty();
    for model in ["jev-latest", "braintrust/jev-latest"] {
        catalog.insert(
            model.into(),
            spec(
                model,
                ProviderFormat::TypeSafe,
                ModelFlavor::Evaluation,
                "typesafe",
            ),
        );
    }
    catalog.insert(
        "native-jev-as-chat".into(),
        spec(
            "native-jev-as-chat",
            ProviderFormat::ChatCompletions,
            ModelFlavor::Chat,
            "typesafe",
        ),
    );
    catalog.insert(
        "gpt-6-luna".into(),
        spec(
            "gpt-6-luna",
            ProviderFormat::ChatCompletions,
            ModelFlavor::Chat,
            "openai",
        ),
    );
    let base = Url::parse(&server.uri()).unwrap();
    RouterBuilder::new()
        .with_catalog(Arc::new(catalog))
        .with_retry_policy(RetryPolicy {
            max_attempts: 0,
            ..RetryPolicy::default()
        })
        .add_provider(
            "typesafe",
            TypeSafeProvider::new(TypeSafeConfig {
                endpoint: base.clone(),
                timeout: None,
            })
            .unwrap(),
            auth("typesafe-key"),
            vec![ProviderFormat::TypeSafe],
        )
        .add_provider(
            "openai",
            OpenAIProvider::new(OpenAIConfig {
                endpoint: base.join("v1").unwrap(),
                ..OpenAIConfig::default()
            })
            .unwrap(),
            auth("openai-key"),
            vec![ProviderFormat::ChatCompletions],
        )
        .build()
        .unwrap()
}

fn body(value: Value) -> Bytes {
    Bytes::from(serde_json::to_vec(&value).unwrap())
}

async fn decide(
    router: &Router,
    model: &str,
    request: Value,
) -> braintrust_llm_router::Result<Value> {
    let routes = router.resolve_provider_routes(model, ProviderFormat::Decisions, &[])?;
    let (prepared, metadata) = router.create_decision_request(body(request), &routes[0], false)?;
    assert_eq!(metadata.detected_input_format, ProviderFormat::Decisions);
    let response = router
        .complete_decision(prepared, &ClientHeaders::default())
        .await?;
    Ok(serde_json::from_slice(&response.response).unwrap())
}

fn scorer_request(model: &str) -> Value {
    json!({
        "model": model,
        "input": [
            {"role": "system", "content": "Grade the answer."},
            {"role": "user", "content": "2 + 2 = 4"}
        ],
        "questions": [{
            "type": "choice",
            "name": "score",
            "instructions": "Pick the grade.",
            "choices": [{"value": "A"}, {"value": "B"}]
        }]
    })
}

#[tokio::test]
async fn routes_typesafe_models_through_system_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .and(header("authorization", "Bearer typesafe-key"))
        .and(body_json(json!({
            "model": "jev-latest",
            "state": {"messages": [
                {"role": "system", "content": "Grade the answer."},
                {"role": "user", "content": "2 + 2 = 4"}
            ]},
            "questions": {"score": {
                "type": "choice",
                "instructions": "Pick the grade.",
                "criteria": {"A": null, "B": null}
            }}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "jev-1.14.0",
            "answers": {"score": {
                "type": "choice",
                "choice": "A",
                "confidence": 0.75,
                "probabilities": {"A": 0.875, "B": 0.125}
            }},
            "usage": {"input_tokens": 30, "output_tokens": 1}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let response = decide(&router(&server), "jev-latest", scorer_request("jev-latest"))
        .await
        .unwrap();
    assert_eq!(
        response,
        json!({
            "model": "jev-1.14.0",
            "answers": [{
                "type": "choice",
                "name": "score",
                "choice": "A",
                "confidence": 0.75,
                "probabilities": [
                    {"value": "A", "probability": 0.875},
                    {"value": "B", "probability": 0.125}
                ]
            }],
            "usage": {
                "input_tokens": 30,
                "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
                "output_tokens": 1,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": 31
            }
        })
    );
}

#[tokio::test]
async fn typesafe_providers_serve_decisions_for_chat_described_models() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "jev-1.14.0",
            "answers": {"score": {"type": "choice", "choice": "B", "confidence": 0.5,
                "probabilities": {"A": 0.25, "B": 0.75}}},
            "usage": {"input_tokens": 30, "output_tokens": 1}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let router = router(&server);
    let routes = router
        .resolve_provider_routes("native-jev-as-chat", ProviderFormat::Decisions, &[])
        .unwrap();
    assert_eq!(routes[0].provider_alias(), "typesafe");
    assert_eq!(routes[0].format(), ProviderFormat::TypeSafe);
    let response = decide(
        &router,
        "native-jev-as-chat",
        scorer_request("native-jev-as-chat"),
    )
    .await
    .unwrap();
    assert_eq!(response["answers"][0]["choice"], json!("B"));
}

#[tokio::test]
async fn passes_openai_decisions_through_unchanged() {
    let server = MockServer::start().await;
    let request = json!({
        "model": "gpt-6-luna",
        "input": "I was charged twice.",
        "questions": [{"type": "predicate", "name": "billing", "instructions": "Is this about billing?"}],
        "prompt_cache_key": "support"
    });
    let decision = json!({
        "model": "gpt-6-luna",
        "answers": [{"type": "predicate", "name": "billing", "probability": 0.97}],
        "usage": {
            "input_tokens": 40,
            "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
            "output_tokens": 0,
            "output_tokens_details": {"reasoning_tokens": 0},
            "total_tokens": 40
        }
    });
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .and(header("authorization", "Bearer openai-key"))
        .and(body_json(request.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(decision.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let response = decide(&router(&server), "gpt-6-luna", request)
        .await
        .unwrap();
    assert_eq!(response, decision);
}

#[tokio::test]
async fn surfaces_upstream_decision_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "detail": "max_tokens_exceeded"
        })))
        .mount(&server)
        .await;

    let error = decide(&router(&server), "jev-latest", scorer_request("jev-latest"))
        .await
        .unwrap_err();
    let Error::Provider { provider, http, .. } = error else {
        panic!("expected a provider error, got {error:?}");
    };
    assert_eq!(provider, "typesafe");
    let http = http.expect("upstream HTTP error");
    assert_eq!(http.status, 400);
    assert!(http.body.contains("max_tokens_exceeded"));
}

#[tokio::test]
async fn rejects_chat_requests_for_decision_models() {
    let server = MockServer::start().await;
    let router = router(&server);
    let routes = router
        .resolve_provider_routes("jev-latest", ProviderFormat::ChatCompletions, &[])
        .unwrap();
    let error = router
        .create_request(
            body(json!({"model": "jev-latest", "messages": [{"role": "user", "content": "hi"}]})),
            ProviderFormat::ChatCompletions,
            &routes[0],
            false,
        )
        .await
        .err()
        .expect("chat request is rejected");
    assert!(
        error
            .to_string()
            .contains("only supports decision requests"),
        "{error}"
    );
}

#[tokio::test]
async fn rejects_decision_requests_the_route_cannot_serve() {
    let server = MockServer::start().await;
    let router = router(&server);
    let error = decide(
        &router,
        "jev-latest",
        json!({
            "model": "jev-latest",
            "input": [{"role": "user", "content": [
                {"type": "input_image", "image_url": "https://example.com/a.png"}
            ]}],
            "questions": [{"type": "predicate", "instructions": "Damaged?"}]
        }),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("image content"), "{error}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn rewrites_decision_body_model_unless_preserved() {
    for (preserve_body_model, upstream_model) in
        [(false, "native-jev-as-chat"), (true, "jev-latest")]
    {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/systemone"))
            .and(body_partial_json(json!({"model": upstream_model})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "jev-1.14.0",
                "answers": {"score": {"type": "choice", "choice": "A", "confidence": 1.0,
                    "probabilities": {"A": 1.0, "B": 0.0}}},
                "usage": {"input_tokens": 30, "output_tokens": 1}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let router = router(&server);
        let routes = router
            .resolve_provider_routes("native-jev-as-chat", ProviderFormat::Decisions, &[])
            .unwrap();
        let (prepared, _) = router
            .create_decision_request(
                body(scorer_request("jev-latest")),
                &routes[0],
                preserve_body_model,
            )
            .unwrap();
        router
            .complete_decision(prepared, &ClientHeaders::default())
            .await
            .unwrap();
    }
}
