//! Call Drex through the Nace provider with a NACE_API_KEY.
//! Run: cargo run -p braintrust-llm-router --example nace

use std::sync::Arc;

use braintrust_llm_router::{
    api_key_auth, serde_json, ClientHeaders, ModelCatalog, NaceConfig, NaceProvider,
    ProviderFormat, Router,
};
use bytes::Bytes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("NACE_API_KEY")?;
    let model = "drex-latest";
    // A local catalog makes this example usable before the shared catalog is updated.
    let catalog = ModelCatalog::from_json_str(
        r#"{"drex-latest": {
            "format": "openai", "flavor": "decisions",
            "available_providers": ["nace"], "supports_streaming": false
        }}"#,
    )?;
    let router = Router::builder()
        .with_catalog(Arc::new(catalog))
        .add_provider(
            "nace",
            NaceProvider::new(NaceConfig::default())?,
            api_key_auth(&api_key),
            vec![],
        )
        .build()?;
    let body = Bytes::from(serde_json::to_vec(&serde_json::json!({
        "model": model,
        "input": "I was charged twice. Please refund the duplicate payment.",
        "questions": [
            {"type": "predicate", "name": "refund", "instructions": "Is a refund requested?"},
            {"type": "choice", "name": "topic", "instructions": "Which topic applies?",
             "choices": [{"value": "billing"}, {"value": "shipping"}]},
            {"type": "score", "name": "urgency", "instructions": "How urgent is the request?",
             "levels": [{"label": "low"}, {"label": "high"}]}
        ]
    }))?);
    let routes = router.resolve_provider_routes(model, ProviderFormat::Decisions, &[])?;
    let route = routes
        .first()
        .ok_or(braintrust_llm_router::Error::NoProvider(
            ProviderFormat::Decisions,
        ))?;
    let (request, _) =
        router.create_decision_request(body, route, false, ProviderFormat::Decisions)?;
    let result = router
        .complete_decision(request, &ClientHeaders::default())
        .await?;
    println!("{}", std::str::from_utf8(&result.response)?);
    Ok(())
}
