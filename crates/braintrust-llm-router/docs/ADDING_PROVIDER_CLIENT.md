# Adding a provider client

This guide walks through adding HTTP client support for a new LLM provider to the router.

## Overview

The `braintrust-llm-router` crate handles HTTP communication with LLM providers. Each provider client implements the `Provider` trait which defines how to send requests and receive responses.

**Key responsibilities:**
- HTTP client configuration (endpoints, timeouts)
- Authentication (API keys, OAuth, AWS SigV4, etc.)
- Request execution (sync and streaming)
- Error handling with retry hints
- Health checks

## Separation from lingua

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| Format | `lingua` | Message transformation (JSON shapes) |
| Client | `braintrust-llm-router` | HTTP I/O, auth, routing, retry |

The router receives **pre-transformed payloads** from lingua. Providers are pure HTTP clients - they forward bytes and return bytes without parsing message content.

```
Request bytes → lingua transforms → Provider sends HTTP → Raw response bytes
```

---

## Provider trait reference

The `Provider` trait defines 5 required methods:

| Method | Signature | Purpose |
|--------|-----------|---------|
| `id()` | `fn id(&self) -> &'static str` | Provider identifier (e.g., `"mistral"`) |
| `format()` | `fn format(&self) -> ProviderFormat` | Which format this provider uses |
| `complete()` | `async fn complete(&self, payload, auth, spec) -> Result<Bytes>` | Non-streaming request |
| `complete_stream()` | `async fn complete_stream(&self, payload, auth, spec) -> Result<RawResponseStream>` | Streaming request |
| `health_check()` | `async fn health_check(&self, auth) -> Result<()>` | Liveness verification |

---

## Step-by-step guide

### Step 1: Create provider module

**Location**: `src/providers/myprovider.rs`

```rust
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;
```

### Step 2: Define config struct

```rust
#[derive(Debug, Clone)]
pub struct MyProviderConfig {
    pub endpoint: Url,
    pub timeout: Option<Duration>,
    // Add provider-specific options here
}

impl Default for MyProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.myprovider.com/v1").expect("valid default URL"),
            timeout: None,
        }
    }
}
```

### Step 3: Define provider struct with constructors

```rust
#[derive(Debug, Clone)]
pub struct MyProviderProvider {
    client: Client,
    config: MyProviderConfig,
}

impl MyProviderProvider {
    pub fn new(config: MyProviderConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }

        // Use shared client when possible, create new one for custom timeouts
        let client = if config.timeout.is_some() {
            crate::client::build_client(&settings)?
        } else {
            default_client().or_else(|_| crate::client::build_client(&settings))?
        };

        Ok(Self { client, config })
    }

    /// Factory method for `create_provider()`.
    pub fn from_config(endpoint: Option<&Url>, timeout: Option<Duration>) -> Result<Self> {
        let mut config = MyProviderConfig::default();
        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }
        Self::new(config)
    }

    fn chat_url(&self) -> Result<Url> {
        let mut url = self.config.endpoint.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("chat");
            segments.push("completions");
        }
        Ok(url)
    }
}
```

### Step 4: Implement Provider trait

```rust
fn extract_retry_after(status: StatusCode) -> Option<Duration> {
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Some(Duration::from_secs(2))
    } else {
        None
    }
}

#[async_trait]
impl crate::providers::Provider for MyProviderProvider {
    fn id(&self) -> &'static str {
        "myprovider"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::MyProvider  // Must exist in lingua
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
    ) -> Result<Bytes> {
        let url = self.chat_url()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "myprovider".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
                http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
            });
        }

        Ok(response.bytes().await?)
    }

    async fn complete_stream(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
    ) -> Result<RawResponseStream> {
        // Fall back to fake streaming if not supported
        if !spec.supports_streaming {
            let response = self.complete(payload, auth, spec).await?;
            return Ok(single_bytes_stream(response));
        }

        let url = self.chat_url()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "myprovider".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
                http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
            });
        }

        // Use sse_stream for standard Server-Sent Events format
        Ok(sse_stream(response))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self.chat_url()?;
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "myprovider".to_string(),
                source: anyhow::anyhow!("status {}", response.status()),
                retry_after: None,
                http: None,
            })
        }
    }
}
```

### Step 5: Register in providers/mod.rs

```rust
mod myprovider;

pub use myprovider::{MyProviderConfig, MyProviderProvider};
```

### Step 6: Add to create_provider() factory

**Location**: `src/router.rs`

```rust
pub fn create_provider(
    kind: &str,
    endpoint: Option<&Url>,
    endpoint_template: Option<&str>,
    timeout: Option<Duration>,
    metadata: &HashMap<String, Value>,
) -> Result<Arc<dyn Provider>> {
    match kind {
        // ... existing providers ...
        "myprovider" => Ok(Arc::new(MyProviderProvider::from_config(endpoint, timeout)?)),
        other => Err(Error::InvalidRequest(format!(
            "unsupported provider kind: {other}"
        ))),
    }
}
```

---

## Authentication reference

The `AuthConfig` enum supports multiple authentication methods:

| Variant | Use case | Header result |
|---------|----------|---------------|
| `ApiKey { key, header, prefix }` | Standard API key | `Authorization: Bearer <key>` or custom |
| `OAuth { access_token, token_type }` | OAuth/Bearer token | `Authorization: Bearer <token>` |
| `AwsSignatureV4 { ... }` | AWS Bedrock | Provider handles signing |
| `AzureEntra { bearer_token }` | Azure OpenAI | `Authorization: Bearer <token>` |
| `Custom { headers }` | Arbitrary headers | Custom key-value pairs |

**Usage in provider:**

```rust
let mut headers = HeaderMap::new();
headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
auth.apply_headers(&mut headers)?;  // Adds auth headers

// For AWS SigV4, handle in provider (see bedrock.rs)
if let Some((access_key, secret_key, session_token, region, service)) = auth.aws_credentials() {
    // Sign request with AWS credentials
}
```

---

## Streaming reference

| Helper | Format | Use case |
|--------|--------|----------|
| `sse_stream(response)` | Server-Sent Events | OpenAI, Anthropic, Mistral, most providers |
| `bedrock_event_stream(response)` | AWS binary event stream | AWS Bedrock |
| `single_bytes_stream(bytes)` | Single chunk | Fake streaming fallback |

**SSE format** (most common):
```
data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" world"}}]}

data: [DONE]
```

**Fake streaming** for providers that don't support native streaming:
```rust
if !spec.supports_streaming {
    let response = self.complete(payload, auth, spec).await?;
    return Ok(single_bytes_stream(response));
}
```

---

## Error handling

Always use `Error::Provider` with full context:

```rust
Err(Error::Provider {
    provider: "myprovider".to_string(),
    source: anyhow::anyhow!("HTTP {status}: {text}"),
    retry_after: extract_retry_after(status),  // Enables router retry
    http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
})
```

**Retry hints:**
- Return `retry_after: Some(Duration)` for rate limits (429) and server errors (5xx)
- The router uses this to implement exponential backoff

---

## Model catalog

The catalog maps model names to providers. Edit `src/catalog/model_list.json`:

### ModelSpec fields

| Field | Type | Purpose |
|-------|------|---------|
| `format` | string | `ProviderFormat` - **must match your provider** |
| `flavor` | string | `chat`, `completion`, `embedding`, or `responses` |
| `displayName` | string | Human-readable name |
| `max_input_tokens` | number | Input token limit |
| `max_output_tokens` | number | Output token limit |
| `input_cost_per_mil_tokens` | number | Cost per 1M input tokens |
| `output_cost_per_mil_tokens` | number | Cost per 1M output tokens |
| `multimodal` | boolean | Supports images/files |
| `reasoning` | boolean | Extended thinking capability |
| `supports_streaming` | boolean | Streaming support (default: true) |
| `parent` | string | Parent model for versioned variants |

### Example entries

```json
{
  "myprovider-large": {
    "format": "myprovider",
    "flavor": "chat",
    "displayName": "MyProvider Large",
    "input_cost_per_mil_tokens": 2.0,
    "output_cost_per_mil_tokens": 6.0,
    "max_input_tokens": 128000,
    "max_output_tokens": 8192
  },
  "myprovider-large-2025-01": {
    "format": "myprovider",
    "flavor": "chat",
    "parent": "myprovider-large",
    "max_input_tokens": 128000,
    "max_output_tokens": 8192
  },
  "myprovider-small": {
    "format": "myprovider",
    "flavor": "chat",
    "displayName": "MyProvider Small",
    "input_cost_per_mil_tokens": 0.5,
    "output_cost_per_mil_tokens": 1.5,
    "max_input_tokens": 32000,
    "max_output_tokens": 4096
  }
}
```

### Loading catalogs

The bundled catalog is included at compile time:
```rust
pub const BUNDLED_CATALOG_JSON: &str = include_str!("model_list.json");
```

Load a custom catalog at runtime:
```rust
let router = Router::builder()
    .load_models("path/to/custom_models.json")?
    .add_provider("myprovider", MyProviderProvider::new(config)?)
    .add_api_key("myprovider", api_key)
    .build()?;
```

### Model resolution

1. **Exact match** - `"gpt-4o"` matches `"gpt-4o"`
2. **Prefix match** - `"gpt-4o-2024-08-06"` falls back to `"gpt-4o"`

---

## Checklist

- [ ] Create `src/providers/myprovider.rs`
- [ ] Define `MyProviderConfig` with endpoint and timeout
- [ ] Implement `new()` and `from_config()` constructors
- [ ] Implement all 5 `Provider` trait methods
- [ ] Handle errors with `retry_after` hints
- [ ] Export in `src/providers/mod.rs`
- [ ] Add case to `create_provider()` in `src/router.rs`
- [ ] Add model entries to `src/catalog/model_list.json`
- [ ] Ensure `format` field in catalog matches `ProviderFormat`

---

## Reference implementation

See `src/providers/mistral.rs` (~240 lines) for a clean, well-documented example that handles:
- Standard SSE streaming
- Error handling with retry hints
- Health checks
- Tracing integration (feature-gated)
