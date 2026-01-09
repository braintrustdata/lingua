//! Example demonstrating streaming responses from LLM providers
//!
//! Run with:
//! ```bash
//! ANTHROPIC_API_KEY=your_key cargo run --example streaming
//! ```

use anyhow::Result;
use braintrust_llm_router::{
    serde_json::json, AnthropicConfig, AnthropicProvider, AuthConfig, ProviderFormat,
    ResponseStream, Router,
};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::Value;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize router
    let anthropic_provider = AnthropicProvider::new(AnthropicConfig::default())?;

    let anthropic_api_key = env::var("ANTHROPIC_API_KEY")?;

    let router = Router::builder()
        .add_provider("anthropic", anthropic_provider)
        .add_auth(
            "anthropic",
            AuthConfig::ApiKey {
                key: anthropic_api_key,
                header: Some("x-api-key".into()),
                prefix: None,
            },
        )
        .build()?;

    println!("🎭 Streaming a story from Claude...\n");
    println!("{}", "=".repeat(50));
    println!();

    // Create a streaming request
    let model = "claude-3-7-sonnet-20250219";
    let payload = json!({
        "model": model,
        "system": "You are a creative storyteller.",
        "messages": [
            {"role": "user", "content": "Write a short story about a robot learning to paint. Make it touching and beautiful."}
        ],
        "temperature": 0.9,
        "max_tokens": 1000,
        "stream": true
    });

    let body = Bytes::from(serde_json::to_vec(&payload)?);
    let mut stream = router
        .complete_stream(body, model, ProviderFormat::OpenAI)
        .await?;

    // Process the stream
    let mut full_response = String::new();
    let mut token_count = 0;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(bytes) => {
                // Parse bytes to Value
                let chunk: Value = match serde_json::from_slice(&bytes) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Skip keep-alive markers
                if chunk.get("_keep_alive").is_some() {
                    continue;
                }

                if let Some(text) = extract_chunk_text(&chunk) {
                    // Print to stdout immediately for real-time effect
                    print!("{text}");
                    io::stdout().flush()?;

                    full_response.push_str(&text);
                    token_count += 1;
                }

                // Check if stream is finished
                if let Some(finish_reason) = chunk_finish_reason(&chunk) {
                    println!("\n\n");
                    println!("{}", "=".repeat(50));
                    println!("✅ Stream finished: {finish_reason:?}");
                    break;
                }
            }
            Err(e) => {
                eprintln!("\n❌ Error in stream: {e}");
                break;
            }
        }
    }

    // Print statistics
    println!("\n📊 Streaming Statistics:");
    println!("  Total chunks received: {token_count}");
    println!("  Total characters: {}", full_response.len());
    let average_chunk = if token_count > 0 {
        full_response.len() as f64 / token_count as f64
    } else {
        0.0
    };
    println!("  Average chunk size: {average_chunk:.1} chars");

    // Demonstrate streaming with multiple models in parallel
    println!("\n🔄 Comparing responses from multiple models...\n");

    let models = vec!["claude-3-5-haiku-20241022", "claude-3-7-sonnet-20250219"];
    let prompt = "Explain quantum computing in one sentence.";

    // Create streams for multiple models
    let mut streams: Vec<(String, ResponseStream)> = Vec::new();
    for &model in &models {
        let payload = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.3,
            "max_tokens": 100,
            "stream": true
        });
        let body = Bytes::from(serde_json::to_vec(&payload)?);
        let stream = router
            .complete_stream(body, model, ProviderFormat::OpenAI)
            .await?;
        streams.push((model.to_string(), stream));
    }

    // Collect responses
    let mut responses = Vec::new();
    for (model, mut stream) in streams {
        let mut response = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    if let Ok(chunk) = serde_json::from_slice::<Value>(&bytes) {
                        if let Some(text) = extract_chunk_text(&chunk) {
                            response.push_str(&text);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Stream error for {model}: {e}");
                    break;
                }
            }
        }
        responses.push((model, response));
    }

    // Display comparative results
    for (model, response) in responses {
        println!("📍 {model}:");
        println!("   {}\n", response.trim());
    }

    Ok(())
}

/// Extract text content from a streaming chunk (Value).
fn extract_chunk_text(chunk: &Value) -> Option<String> {
    // Skip keep-alive markers
    if chunk.get("_keep_alive").is_some() {
        return None;
    }

    let choices = chunk.get("choices")?.as_array()?;
    choices.iter().find_map(|choice| {
        let delta = choice.get("delta")?;
        match delta {
            Value::Object(map) => {
                if let Some(content) = map.get("content") {
                    match content {
                        Value::Array(parts) => {
                            let mut text = String::new();
                            for part in parts {
                                if let Value::Object(part_map) = part {
                                    if let Some(Value::String(fragment)) = part_map.get("text") {
                                        text.push_str(fragment);
                                    }
                                } else if let Some(fragment) = part.as_str() {
                                    text.push_str(fragment);
                                }
                            }
                            if text.is_empty() {
                                None
                            } else {
                                Some(text)
                            }
                        }
                        Value::String(text) => Some(text.clone()),
                        _ => None,
                    }
                } else if let Some(Value::String(text)) = map.get("text") {
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.clone())
                    }
                } else {
                    None
                }
            }
            Value::String(text) => Some(text.clone()),
            _ => None,
        }
    })
}

/// Extract finish_reason from a streaming chunk (Value).
fn chunk_finish_reason(chunk: &Value) -> Option<&str> {
    let choices = chunk.get("choices")?.as_array()?;
    choices
        .iter()
        .find_map(|choice| choice.get("finish_reason")?.as_str())
}
