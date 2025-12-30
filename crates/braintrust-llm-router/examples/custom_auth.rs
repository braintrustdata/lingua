//! Example demonstrating different authentication methods
//!
//! This example shows how to configure various authentication types:
//! - API Key (standard)
//! - API Key with custom header (e.g., Anthropic's x-api-key)
//! - OAuth tokens
//! - AWS Signature V4 (for Bedrock)
//! - Azure Entra ID
//! - Custom headers
//!
//! Run with:
//! ```bash
//! OPENAI_API_KEY=your_key cargo run --example custom_auth
//! ```

use anyhow::Result;
use braintrust_llm_router::{
    serde_json::json, AuthConfig, OpenAIConfig, OpenAIProvider, ProviderFormat, Router,
};
use bytes::Bytes;
use serde_json::Value;
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔐 Authentication Configuration Examples\n");
    println!("{}", "=".repeat(50));

    // Example 1: Standard API Key (OpenAI style)
    println!("\n1️⃣  Standard API Key Authentication (OpenAI)");
    println!("   Header: Authorization: Bearer <key>");

    let standard_api_key = AuthConfig::ApiKey {
        key: "sk-your-openai-key".into(),
        header: Some("authorization".into()),
        prefix: Some("Bearer".into()),
    };
    println!("   Config: {:?}", standard_api_key.auth_type());

    // Example 2: Custom Header API Key (Anthropic style)
    println!("\n2️⃣  Custom Header API Key (Anthropic)");
    println!("   Header: x-api-key: <key>");

    let anthropic_api_key = AuthConfig::ApiKey {
        key: "sk-ant-your-anthropic-key".into(),
        header: Some("x-api-key".into()),
        prefix: None, // No prefix for Anthropic
    };
    println!("   Config: {:?}", anthropic_api_key.auth_type());

    // Example 3: OAuth Token
    println!("\n3️⃣  OAuth Token Authentication");
    println!("   Header: Authorization: Bearer <token>");

    let oauth_config = AuthConfig::OAuth {
        access_token: "your-oauth-access-token".into(),
        token_type: Some("Bearer".into()),
    };
    println!("   Config: {:?}", oauth_config.auth_type());

    // Example 4: AWS Signature V4 (for Bedrock)
    println!("\n4️⃣  AWS Signature V4 (Bedrock)");
    println!("   Uses AWS credentials to sign requests");

    let aws_config = AuthConfig::AwsSignatureV4 {
        access_key: "AKIAIOSFODNN7EXAMPLE".into(),
        secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        session_token: None, // Optional for temporary credentials
        region: "us-east-1".into(),
        service: "bedrock-runtime".into(),
    };
    println!("   Config: {:?}", aws_config.auth_type());

    // Example 5: Azure Entra ID
    println!("\n5️⃣  Azure Entra ID (formerly Azure AD)");
    println!("   Header: Authorization: Bearer <entra_token>");

    let azure_config = AuthConfig::AzureEntra {
        bearer_token: "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIs...".into(),
    };
    println!("   Config: {:?}", azure_config.auth_type());

    // Example 6: Custom Headers
    println!("\n6️⃣  Custom Headers");
    println!("   For providers with non-standard auth requirements");

    let mut custom_headers = HashMap::new();
    custom_headers.insert("X-Custom-Auth".into(), "custom-token-value".into());
    custom_headers.insert("X-Org-ID".into(), "org-12345".into());

    let custom_config = AuthConfig::Custom {
        headers: custom_headers,
    };
    println!("   Config: {:?}", custom_config.auth_type());

    // Live demonstration with OpenAI if key is available
    println!("\n{}", "=".repeat(50));
    println!("\n🚀 Live Demo with OpenAI\n");

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("   OPENAI_API_KEY not set - skipping live demo.");
            println!("   Set the environment variable to see the auth in action.\n");
            return Ok(());
        }
    };

    let model_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/catalog/model_list.json");

    let openai_provider = OpenAIProvider::new(OpenAIConfig::default())?;

    // Using the helper method for simple API key auth
    let router = Router::builder()
        .load_models(model_path)?
        .add_provider("openai", openai_provider)
        .add_api_key("openai", api_key) // Convenience method
        .build()?;

    let model = "gpt-4";
    let payload = json!({
        "model": model,
        "messages": [{"role": "user", "content": "Say 'Authentication successful!' in a creative way."}],
        "max_tokens": 50
    });

    println!("   Sending authenticated request to GPT-4...");
    let body = Bytes::from(serde_json::to_vec(&payload)?);
    let bytes = router.complete(body, model, ProviderFormat::OpenAI).await?;
    let response: Value = serde_json::from_slice(&bytes)?;

    if let Some(text) = extract_assistant_text(&response) {
        println!("   Response: {}\n", text.trim());
    }

    println!("✅ Authentication example completed!");

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
