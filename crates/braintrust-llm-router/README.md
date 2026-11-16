# Braintrust LLM Router

> Production-ready Rust library for routing requests to various LLM providers

[![Crates.io](https://img.shields.io/crates/v/braintrust-llm-router.svg)](https://crates.io/crates/braintrust-llm-router)
[![Documentation](https://docs.rs/braintrust-llm-router/badge.svg)](https://docs.rs/braintrust-llm-router)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)

## Overview

`braintrust-llm-router` is a low-level routing layer for LLM providers. It accepts JSON request bytes, routes to the appropriate provider, and returns pre-serialized response bytes. This design enables zero-copy request forwarding in proxy/gateway architectures.

## Key features

- **Multiple providers** - OpenAI, Anthropic, Google, Bedrock, Azure, Mistral, and OpenAI-compatible endpoints
- **Format translation** - Automatic conversion between provider formats via [lingua](https://github.com/braintrustdata/lingua)
- **Streaming support** - First-class SSE streaming with pre-serialized chunks
- **Model catalog** - 250+ pre-configured models with pricing and capability metadata
- **Flexible auth** - API keys, OAuth, AWS SigV4, Azure Entra
- **Retry logic** - Configurable exponential backoff with jitter

## Installation

```toml
[dependencies]
braintrust-llm-router = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick start

```rust
use braintrust_llm_router::{
    OpenAIProvider, OpenAIConfig, ProviderFormat, Router, RouterResponse,
};
use bytes::Bytes;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Build router with provider and auth
    let router = Router::builder()
        .load_models("model_list.json")?
        .add_provider("openai", OpenAIProvider::new(OpenAIConfig::default())?)
        .add_api_key("openai", std::env::var("OPENAI_API_KEY")?)
        .build()?;

    // Request as JSON bytes
    let body = Bytes::from(serde_json::to_vec(&json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hello!"}]
    }))?);

    // Route request - auto-extracts model, returns pre-serialized response
    let result = router.handle(body, ProviderFormat::ChatCompletions, None).await?;

    match result.response {
        RouterResponse::Sync(bytes) => {
            println!("Provider: {}", result.provider);
            println!("Response: {}", String::from_utf8_lossy(&bytes));
        }
        RouterResponse::Stream(mut stream) => {
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                let bytes = chunk?;
                // Each chunk is pre-serialized JSON, ready for SSE
                print!("{}", String::from_utf8_lossy(&bytes));
            }
        }
    }
    Ok(())
}
```

## Supported providers

| Provider | Auth | Streaming | Notes |
|----------|------|-----------|-------|
| OpenAI | API Key (Bearer) | Yes | |
| Anthropic | API Key (x-api-key) | Yes | |
| Google | API Key | Yes | Gemini |
| Vertex AI | OAuth / Service Account | Yes | |
| AWS Bedrock | AWS SigV4 | Yes | |
| Azure OpenAI | API Key / Entra | Yes | |
| Mistral | API Key | Yes | |
| Nace | API Key (Bearer) | No | Drex decision models; OpenAI Decisions and System One |
| OpenAI-compatible | API Key | Yes | Together, Groq, Perplexity, etc. |

### Nace Drex decisions

Run the [Nace example](examples/nace.rs) with `NACE_API_KEY` set:

```bash
cargo run -p braintrust-llm-router --example nace
```

`create_provider("nace", ...)` defaults to `https://console.nace.ai`. Custom endpoints
may include `/v1`. Register Nace credentials under `nace` and catalog models with
`format: "openai"`, `flavor: "decisions"`, `supports_streaming: false` and
`available_providers: ["nace"]`. The pinned models are `drex-max-v1.1`,
`drex-max-finance` and `drex-v2.0`.

Use the router's decision entry points (`create_decision_request` and
`complete_decision`). OpenAI Decisions requests use `/v1/decisions`; native TypeSafe
requests use `/v1/systemone`. Chat, Responses and streaming are unsupported.
The provider forwards `X-Nace-Client: braintrust/<version>` for attribution and
requests object-shaped score legends on the System One endpoint.

Drex Max v1.1 and Finance accept inline PNG, JPEG and WebP images; Drex 2.0 is text
only. For JSON System One calls, put data URLs in the top-level `images` array;
TypeSafe JSON `assets` are not supported. Drex's [Decisions API contract](https://console.nace.ai/docs/drex/guides/openai-compatible-decisions)
limits supported input roles, question types and image inputs; unsupported inputs
return the upstream validation error. Responses retain probability values, question
order and usage. Nace [bills input tokens only](https://console.nace.ai/docs/drex/reference/pricing),
including image tokens; reported output tokens are not billed.

This configures the routing library. Braintrust's hosted gateway also needs its
Nace credential/provider registration and deployment; a local example does not
enable a hosted listing.

## API reference

### Router construction

```rust
use braintrust_llm_router::{Router, RetryPolicy, OpenAIProvider, OpenAIConfig};
use std::time::Duration;

let router = Router::builder()
    .load_models("model_list.json")?           // Load model catalog
    .add_provider("openai", OpenAIProvider::new(OpenAIConfig::default())?)
    .add_api_key("openai", "sk-...")           // Convenience for Bearer auth
    .with_retry_policy(RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(10),
        exponential_base: 2.0,
        jitter: true,
    })
    .build()?;
```

### Making requests

Two entry points:

```rust
// handle() - auto-extracts model from body, detects streaming
let result = router.handle(body, ProviderFormat::ChatCompletions, None).await?;

// complete() / complete_stream() - explicit model parameter
let bytes = router.complete(body, "gpt-4", ProviderFormat::ChatCompletions, None).await?;
let stream = router.complete_stream(body, "gpt-4", ProviderFormat::ChatCompletions, None).await?;
```

### Authentication

```rust
use braintrust_llm_router::{api_key_auth, AuthConfig};

// OpenAI-style (Authorization: Bearer <key>)
api_key_auth("sk-...")
// Or equivalently:
AuthConfig::ApiKey {
    key: "sk-...".into(),
    header: Some("authorization".into()),
    prefix: Some("Bearer".into()),
}

// Anthropic-style (x-api-key: <key>)
AuthConfig::ApiKey {
    key: "sk-ant-...".into(),
    header: Some("x-api-key".into()),
    prefix: None,
}

// AWS Bedrock
AuthConfig::AwsSignatureV4 {
    access_key: "AKIA...".into(),
    secret_key: "...".into(),
    session_token: None,
    region: "us-east-1".into(),
    service: "bedrock-runtime".into(),
}

// Azure Entra ID
AuthConfig::AzureEntra {
    bearer_token: "eyJ...".into(),
}
```

### Model catalog

```rust
let catalog = router.catalog();

// Get model metadata
if let Some(spec) = catalog.get("gpt-4") {
    println!("Format: {:?}", spec.format);
    println!("Max input tokens: {:?}", spec.max_input_tokens);
    println!("Cost: ${}/1M tokens", spec.input_cost_per_mil_tokens.unwrap_or(0.0));
}

// Iterate all models
for (name, spec) in catalog.iter() {
    println!("{}: {:?}", name, spec.format);
}
```

## Examples

See the [examples/](examples/) directory:

- [simple.rs](examples/simple.rs) - Basic usage
- [streaming.rs](examples/streaming.rs) - Streaming responses
- [multi_provider.rs](examples/multi_provider.rs) - Multiple providers
- [custom_auth.rs](examples/custom_auth.rs) - Authentication methods

## Testing

```bash
cargo test --all-features
```

## License

Licensed under MIT OR Apache-2.0. See [LICENSE](LICENSE).

## Credits

Built by [Braintrust](https://www.braintrust.dev). Based on the routing logic from [Braintrust Proxy](https://github.com/braintrustdata/braintrust-proxy).
