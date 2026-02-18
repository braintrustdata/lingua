mod anthropic;
mod azure;
mod bedrock;
mod google;
mod mistral;
mod openai;
mod vertex;

pub use anthropic::{AnthropicConfig, AnthropicProvider};
pub use azure::{AzureConfig, AzureProvider};
pub use bedrock::{BedrockConfig, BedrockProvider};
pub use google::{GoogleConfig, GoogleProvider};
pub use mistral::{MistralConfig, MistralProvider};
pub use openai::{
    is_openai_compatible, openai_compatible_endpoint, OpenAICompatibleEndpoint, OpenAIConfig,
    OpenAIProvider,
};
pub use vertex::{VertexConfig, VertexProvider};

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::error::Result;
use crate::streaming::RawResponseStream;
use lingua::ProviderFormat;

/// Header prefixes blocked from forwarding to upstream LLM providers.
pub const BLOCKED_HEADER_PREFIXES: &[&str] = &["x-amzn", "x-bt", "sec-", "cf-"];

/// Exact header names blocked from forwarding to upstream LLM providers
/// from https://github.com/braintrustdata/braintrust-proxy/blob/e992f51734c71e689ea0090f9e0a6759c9a593a4/packages/proxy/src/proxy.ts#L247
pub const BLOCKED_HEADERS: &[&str] = &[
    "authorization",
    "api-key",
    "x-api-key",
    "x-auth-token",
    "content-length",
    "origin",
    "priority",
    "referer",
    "user-agent",
    "cache-control",
    // Block accept-encoding so reqwest handles compression internally.
    // If client's Accept-Encoding is forwarded, reqwest skips auto-decompression.
    "accept-encoding",
    // Block proxy/forwarding headers to avoid conflicts with upstream CDNs (e.g., Cloudflare Error 1000)
    "x-forwarded-for",
    "x-forwarded-proto",
    "x-forwarded-host",
    "x-real-ip",
];

#[derive(Debug, Clone, Default)]
pub struct ClientHeaders {
    inner: HashMap<String, String>,
}

impl ClientHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_blocked(name: &str) -> bool {
        let name = name.to_lowercase();
        BLOCKED_HEADER_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
            || BLOCKED_HEADERS.iter().any(|&blocked| name == blocked)
    }

    pub fn insert_if_allowed(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        if !Self::is_blocked(&name) {
            self.inner.insert(name.to_lowercase(), value.into());
        }
    }

    pub fn apply(&self, headers: &mut HeaderMap) {
        for (name, value) in &self.inner {
            if name == "host" {
                // Don't forward client Host; reqwest sets it from the upstream URL.
                continue;
            }
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                headers.insert(header_name, header_value);
            }
        }
    }

    pub(crate) fn to_json_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        self.apply(&mut headers);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

// NOTE: This FromIterator impl exists to support collecting forwarded client
// headers from `(String, String)` pairs at crate boundaries. We use pairs instead
// of `http::HeaderMap` because different workspace crates depend on different
// major versions of the `http` crate, making `HeaderMap` incompatible.
impl FromIterator<(String, String)> for ClientHeaders {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        let mut client_headers = ClientHeaders::new();
        for (name, value) in iter {
            client_headers.insert_if_allowed(name, value);
        }
        client_headers
    }
}

/// Provider trait for LLM API backends.
///
/// Implementations should be `Send + Sync` to allow concurrent access.
/// Providers are stored as `Arc<dyn Provider>` in the Router.
///
/// Providers are pure HTTP clients - they receive pre-transformed payloads
/// as bytes, forward them to the upstream API, and return raw bytes responses.
/// All format transformations happen in the Router layer via lingua.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (e.g., "openai", "anthropic").
    fn id(&self) -> &'static str;

    /// All formats this provider can handle.
    fn provider_formats(&self) -> Vec<ProviderFormat>;

    /// Execute a completion request.
    ///
    /// Returns raw bytes response from the provider. The Router handles
    /// converting this to the requested output format via lingua.
    ///
    /// # Arguments
    ///
    /// * `payload` - Pre-transformed bytes payload ready to send to the provider
    /// * `auth` - Authentication configuration
    /// * `spec` - Model specification
    /// * `client_headers` - Client headers to forward to the upstream provider
    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes>;

    /// Execute a streaming completion request.
    ///
    /// Returns a stream of raw bytes chunks. The Router handles transforming
    /// these to the requested output format via `transform_stream()`.
    ///
    /// # Arguments
    ///
    /// * `payload` - Pre-transformed bytes payload ready to send to the provider
    /// * `auth` - Authentication configuration
    /// * `spec` - Model specification
    /// * `client_headers` - Client headers to forward to the upstream provider
    async fn complete_stream(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<RawResponseStream>;

    /// Check if the provider is reachable.
    async fn health_check(&self, auth: &AuthConfig) -> Result<()>;

    /// Build HTTP headers for a request.
    ///
    /// Default implementation returns JSON content-type headers.
    /// Override for provider-specific headers (e.g., OpenAI-Organization).
    fn build_headers(&self, client_headers: &ClientHeaders) -> HeaderMap {
        client_headers.to_json_headers()
    }
}

impl dyn Provider {
    pub fn arc(self: Arc<Self>) -> Arc<dyn Provider> {
        self
    }
}
