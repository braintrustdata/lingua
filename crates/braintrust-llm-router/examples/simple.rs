//! Simple example showing basic usage of the Braintrust LLM Router
//!
//! Run with:
//! ```bash
//! OPENAI_API_KEY=your_key cargo run --example simple
//! ```

use anyhow::Result;
use braintrust_llm_router::{
    serde_json::json, ClientHeaders, ModelCatalog, OpenAIConfig, OpenAIProvider, ProviderFormat,
    Router,
};
use bytes::Bytes;
use serde_json::Value;
use std::env;
use std::sync::Arc;

const MODEL_CATALOG_URL: &str = "https://raw.githubusercontent.com/braintrustdata/braintrust-proxy/main/packages/proxy/schema/model_list.json";

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "OPENAI_API_KEY environment variable not set. Skipping API call.\n\
                 Provide a key to run the online example."
            );
            return Ok(());
        }
    };

    let catalog_json = reqwest::get(MODEL_CATALOG_URL).await?.text().await?;
    let catalog = Arc::new(ModelCatalog::from_json_str(&catalog_json)?);

    let openai_provider = OpenAIProvider::new(OpenAIConfig::default())?;

    let router = Router::builder()
        .with_catalog(catalog)
        .add_provider("openai", openai_provider)
        .add_api_key("openai", api_key)
        .build()?;

    // Simple chat completion
    println!("🤖 Sending request to GPT-4...\n");

    let model = "gpt-4";
    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "What are the main benefits of using Rust for systems programming?"}
        ],
        "temperature": 0.7,
        "max_tokens": 500
    });

    // Convert payload to bytes and send request
    let body = Bytes::from(serde_json::to_vec(&payload)?);
    let bytes = router
        .complete(
            body,
            model,
            ProviderFormat::ChatCompletions,
            &ClientHeaders::default(),
        )
        .await?;
    let response: Value = serde_json::from_slice(&bytes)?;

    println!("📝 Response:\n");
    if let Some(content) = extract_assistant_text(&response) {
        println!("{content}");
    } else {
        println!("<no content returned>");
    }

    // Print usage statistics
    if let Some(usage) = response.get("usage") {
        println!("\n📊 Token Usage:");
        if let Some(input) = usage.get("prompt_tokens").and_then(Value::as_u64) {
            println!("  Input tokens: {input}");
        }
        if let Some(output) = usage.get("completion_tokens").and_then(Value::as_u64) {
            println!("  Output tokens: {output}");
        }
        if let Some(total) = usage.get("total_tokens").and_then(Value::as_u64) {
            println!("  Total tokens: {total}");
        }
    }

    // Print model information
    println!("\n🔍 Model Information:");
    if let Some(spec) = router.catalog().get("gpt-4") {
        let spec = spec.as_ref();
        println!(
            "  Display Name: {}",
            spec.display_name.as_deref().unwrap_or("GPT-4")
        );
        println!("  Format: {:?}", spec.format);
        println!("  Max Input Tokens: {:?}", spec.max_input_tokens);
        println!("  Max Output Tokens: {:?}", spec.max_output_tokens);
        if let Some(cost) = spec.input_cost_per_mil_tokens {
            println!("  Input Cost: ${cost:.2}/1M tokens");
        }
        if let Some(cost) = spec.output_cost_per_mil_tokens {
            println!("  Output Cost: ${cost:.2}/1M tokens");
        }
    }

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
