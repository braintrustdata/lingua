mod anthropic;
mod azure;
mod bedrock;
mod google;
mod mistral;
mod openai;
mod openai_responses;
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
pub use openai_responses::OpenAIResponsesProvider;
pub use vertex::{VertexConfig, VertexProvider};

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::error::Result;
use crate::streaming::RawResponseStream;
use lingua::ProviderFormat;

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

    /// The format this provider expects/produces.
    fn format(&self) -> ProviderFormat;

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
    async fn complete(&self, payload: Bytes, auth: &AuthConfig, spec: &ModelSpec) -> Result<Bytes>;

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
    async fn complete_stream(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
    ) -> Result<RawResponseStream>;

    /// Check if the provider is reachable.
    async fn health_check(&self, auth: &AuthConfig) -> Result<()>;
}

impl dyn Provider {
    pub fn arc(self: Arc<Self>) -> Arc<dyn Provider> {
        self
    }
}
