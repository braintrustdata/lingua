//! Example demonstrating routing requests to multiple LLM providers
//!
//! This example shows how to configure the router with multiple providers
//! and route requests based on the model name.
//!
//! Run with:
//! ```bash
//! OPENAI_API_KEY=your_key ANTHROPIC_API_KEY=your_key cargo run --example multi_provider
//! ```

use anyhow::Result;
use braintrust_llm_router::{
    serde_json::json, AnthropicConfig, AnthropicProvider, AuthConfig, ClientHeaders, ModelCatalog,
    OpenAIConfig, OpenAIProvider, ProviderFormat, Router,
};
use bytes::Bytes;
use serde_json::Value;
use std::env;
use std::sync::Arc;

const MODEL_CATALOG_URL: &str = "https://raw.githubusercontent.com/braintrustdata/braintrust-proxy/main/packages/proxy/schema/model_list.json";

#[tokio::main]
async fn main() -> Result<()> {
    let openai_key = env::var("OPENAI_API_KEY").ok();
    let anthropic_key = env::var("ANTHROPIC_API_KEY").ok();

    if openai_key.is_none() && anthropic_key.is_none() {
        eprintln!(
            "No API keys found. Set OPENAI_API_KEY and/or ANTHROPIC_API_KEY to run this example."
        );
        return Ok(());
    }

    let catalog_json = reqwest::get(MODEL_CATALOG_URL).await?.text().await?;
    let catalog = Arc::new(ModelCatalog::from_json_str(&catalog_json)?);

    let mut builder = Router::builder().with_catalog(catalog);

    // Add OpenAI provider if key is available
    if let Some(key) = &openai_key {
        let openai_provider = OpenAIProvider::new(OpenAIConfig::default())?;
        builder = builder
            .add_provider("openai", openai_provider)
            .add_api_key("openai", key.clone());
        println!("✅ OpenAI provider configured");
    }

    // Add Anthropic provider if key is available
    if let Some(key) = &anthropic_key {
        let anthropic_provider = AnthropicProvider::new(AnthropicConfig::default())?;
        builder = builder
            .add_provider("anthropic", anthropic_provider)
            .add_auth(
                "anthropic",
                AuthConfig::ApiKey {
                    key: key.clone(),
                    header: Some("x-api-key".into()),
                    prefix: None,
                },
            );
        println!("✅ Anthropic provider configured");
    }

    let router = builder.build()?;

    println!("\n🔄 Testing multi-provider routing...\n");

    let prompt = "What is 2 + 2? Answer in one word.";

    // Test OpenAI if available
    if openai_key.is_some() {
        println!("📍 Sending request to GPT-4...");
        let model = "gpt-4";
        let payload = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 50
        });

        let body = Bytes::from(serde_json::to_vec(&payload)?);
        match router
            .complete(
                body,
                model,
                ProviderFormat::ChatCompletions,
                &ClientHeaders::default(),
            )
            .await
        {
            Ok(bytes) => {
                if let Ok(response) = serde_json::from_slice::<Value>(&bytes) {
                    if let Some(text) = extract_assistant_text(&response) {
                        println!("   Response: {}\n", text.trim());
                    }
                }
            }
            Err(e) => println!("   Error: {e}\n"),
        }
    }

    // Test Anthropic if available
    if anthropic_key.is_some() {
        println!("📍 Sending request to Claude...");
        let model = "claude-3-5-haiku-20241022";
        let payload = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 50
        });

        let body = Bytes::from(serde_json::to_vec(&payload)?);
        match router
            .complete(
                body,
                model,
                ProviderFormat::ChatCompletions,
                &ClientHeaders::default(),
            )
            .await
        {
            Ok(bytes) => {
                if let Ok(response) = serde_json::from_slice::<Value>(&bytes) {
                    if let Some(text) = extract_assistant_text(&response) {
                        println!("   Response: {}\n", text.trim());
                    }
                }
            }
            Err(e) => println!("   Error: {e}\n"),
        }
    }

    // Show how routing works based on model names
    println!("📊 Model routing demonstration:");
    println!("   - 'gpt-4' routes to OpenAI");
    println!("   - 'claude-3-*' routes to Anthropic");
    println!("   - The router automatically selects the correct provider based on model format");

    Ok(())
}

/// Extract text content from assistant messages in OpenAI format response
fn extract_assistant_text(response: &Value) -> Option<String> {
    response
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
        .map(String::from)
}
