use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "tracing")]
use tracing::Instrument;

use bytes::Bytes;

use crate::auth::AuthConfig;
use crate::catalog::{
    default_catalog, load_catalog_from_disk, ModelCatalog, ModelResolver, ModelSpec,
};
use crate::error::{Error, Result};
use crate::providers::Provider;
use crate::retry::{RetryPolicy, RetryStrategy};
use crate::streaming::{transform_stream, ResponseStream};
use lingua::serde_json::Value;
use lingua::ProviderFormat;
use lingua::{TransformError, TransformResult};

// Re-export for convenience in dependent crates
pub use lingua::{extract_request_hints, RequestHints};
use reqwest::Url;

use crate::providers::{
    is_openai_compatible, AnthropicProvider, AzureProvider, BedrockProvider, GoogleProvider,
    MistralProvider, OpenAIProvider, VertexProvider,
};

/// Create a provider instance from configuration parameters.
///
/// This is the factory function for creating providers based on `kind`.
/// Use this with `RouterBuilder` to construct a `Router`.
///
/// # Arguments
///
/// * `kind` - Provider type: "openai", "anthropic", "azure", "google", "vertex", "bedrock", "mistral", or OpenAI-compatible
/// * `endpoint` - Custom endpoint URL (optional)
/// * `endpoint_template` - Endpoint template with `<model>` placeholder (optional, OpenAI only)
/// * `timeout` - Request timeout (optional)
/// * `metadata` - Provider-specific options (organization_id, project, api_version, etc.)
///
/// # Example
///
/// ```ignore
/// use braintrust_llm_router::{create_provider, Router, AuthConfig};
/// use std::collections::HashMap;
///
/// let metadata = HashMap::new();
/// let provider = create_provider("openai", None, None, None, &metadata)?;
/// let router = Router::builder()
///     .with_catalog(catalog)
///     .add_provider("openai", provider)
///     .add_auth("openai", auth)
///     .build()?;
/// ```
pub fn create_provider(
    kind: &str,
    endpoint: Option<&Url>,
    endpoint_template: Option<&str>,
    timeout: Option<Duration>,
    metadata: &HashMap<String, Value>,
) -> Result<Arc<dyn Provider>> {
    match kind {
        "openai" => Ok(Arc::new(OpenAIProvider::from_config(
            endpoint,
            endpoint_template,
            timeout,
            metadata,
        )?)),
        "anthropic" => Ok(Arc::new(AnthropicProvider::from_config(
            endpoint, timeout, metadata,
        )?)),
        "azure" => Ok(Arc::new(AzureProvider::from_config(
            endpoint, timeout, metadata,
        )?)),
        "google" => Ok(Arc::new(GoogleProvider::from_config(endpoint, timeout)?)),
        "vertex" => Ok(Arc::new(VertexProvider::from_config(
            endpoint, timeout, metadata,
        )?)),
        "bedrock" => Ok(Arc::new(BedrockProvider::from_config(
            endpoint, timeout, metadata,
        )?)),
        "mistral" => Ok(Arc::new(MistralProvider::from_config(endpoint, timeout)?)),
        kind if is_openai_compatible(kind) => Ok(Arc::new(OpenAIProvider::from_config(
            endpoint,
            endpoint_template,
            timeout,
            metadata,
        )?)),
        other => Err(Error::InvalidRequest(format!(
            "unsupported provider kind: {other}"
        ))),
    }
}

/// Resolved route information from model resolution.
type ResolvedRoute<'a> = (
    Arc<dyn Provider>,
    &'a AuthConfig,
    Arc<ModelSpec>,
    RetryStrategy,
);

pub struct Router {
    catalog: Arc<ModelCatalog>,
    resolver: ModelResolver,
    providers: HashMap<String, Arc<dyn Provider>>, // alias -> provider
    formats: HashMap<ProviderFormat, String>,      // format -> alias
    auth_configs: HashMap<String, AuthConfig>,     // alias -> auth
    retry_policy: RetryPolicy,
}

impl Router {
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    pub fn catalog(&self) -> Arc<ModelCatalog> {
        Arc::clone(&self.catalog)
    }

    /// Execute a completion request with the given body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// The body will be automatically transformed to the target provider's format if needed.
    /// The response will be converted to the requested output format.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete",
            skip(self, body),
            fields(llm.model = %model)
        )
    )]
    pub async fn complete(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Bytes> {
        let (provider, auth, spec, strategy) = self.resolve_provider(model)?;
        let payload = match lingua::transform_request(body.clone(), provider.format(), Some(model))
        {
            Ok(TransformResult::PassThrough(bytes)) => bytes,
            Ok(TransformResult::Transformed { bytes, .. }) => bytes,
            Err(TransformError::UnsupportedTargetFormat(_)) => body.clone(),
            Err(e) => return Err(Error::Lingua(e.to_string())),
        };

        let response_bytes = self
            .execute_with_retry(provider.clone(), auth, spec, payload, strategy)
            .await?;

        let result = lingua::transform_response(response_bytes.clone(), output_format)
            .map_err(|e| Error::Lingua(e.to_string()))?;

        let response = match result {
            TransformResult::PassThrough(bytes) => bytes,
            TransformResult::Transformed { bytes, .. } => bytes,
        };

        Ok(response)
    }

    /// Execute a streaming completion request with the given body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// The body will be automatically transformed to the target provider's format if needed.
    /// Stream chunks will be transformed to the requested output format.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete_stream",
            skip(self, body),
            fields(llm.model = %model)
        )
    )]
    pub async fn complete_stream(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<ResponseStream> {
        let (provider, auth, spec, _) = self.resolve_provider(model)?;
        let payload = match lingua::transform_request(body.clone(), provider.format(), Some(model))
        {
            Ok(TransformResult::PassThrough(bytes)) => bytes,
            Ok(TransformResult::Transformed { bytes, .. }) => bytes,
            Err(TransformError::UnsupportedTargetFormat(_)) => body.clone(),
            Err(e) => return Err(Error::Lingua(e.to_string())),
        };

        let raw_stream = provider.complete_stream(payload, auth, &spec).await?;

        Ok(transform_stream(raw_stream, output_format))
    }

    pub fn provider_alias(&self, model: &str) -> Result<String> {
        let (_, format, alias) = self.resolver.resolve(model)?;
        Ok(self.formats.get(&format).cloned().unwrap_or(alias))
    }

    fn resolve_provider(&self, model: &str) -> Result<ResolvedRoute<'_>> {
        let (spec, format, alias) = self.resolver.resolve(model)?;
        let alias = self.formats.get(&format).cloned().unwrap_or(alias);
        let provider = self
            .providers
            .get(&alias)
            .cloned()
            .ok_or_else(|| Error::NoProvider(format))?;
        let auth = self
            .auth_configs
            .get(&alias)
            .ok_or_else(|| Error::NoAuth(alias.clone()))?;
        let strategy = self.retry_policy.strategy();
        Ok((provider, auth, spec, strategy))
    }

    async fn execute_with_retry(
        &self,
        provider: Arc<dyn Provider>,
        auth: &AuthConfig,
        spec: Arc<ModelSpec>,
        payload: Bytes,
        mut strategy: RetryStrategy,
    ) -> Result<Bytes> {
        #[cfg(feature = "tracing")]
        let mut attempt = 0u32;

        loop {
            #[cfg(feature = "tracing")]
            {
                attempt += 1;
            }

            #[cfg(feature = "tracing")]
            let result = {
                let span = tracing::info_span!(
                    "bt.router.provider.attempt",
                    llm.provider = %provider.id(),
                    attempt = attempt,
                );
                async { provider.complete(payload.clone(), auth, &spec).await }
                    .instrument(span)
                    .await
            };

            #[cfg(not(feature = "tracing"))]
            let result = provider.complete(payload.clone(), auth, &spec).await;

            match result {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if let Some(delay) = strategy.next_delay(&err) {
                        #[cfg(feature = "tracing")]
                        tracing::info!(
                            llm.provider = %provider.id(),
                            attempt = attempt,
                            delay_ms = delay.as_millis() as u64,
                            error = %err,
                            "retrying after delay"
                        );
                        sleep(delay).await;
                        continue;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
}

pub struct RouterBuilder {
    catalog: Option<Arc<ModelCatalog>>,
    providers: HashMap<String, Arc<dyn Provider>>,
    formats: HashMap<ProviderFormat, String>,
    auth_configs: HashMap<String, AuthConfig>,
    retry_policy: RetryPolicy,
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RouterBuilder {
    pub fn new() -> Self {
        Self {
            catalog: Some(default_catalog()),
            providers: HashMap::new(),
            formats: HashMap::new(),
            auth_configs: HashMap::new(),
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn load_models(mut self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        let catalog = load_catalog_from_disk(path)?;
        self.catalog = Some(catalog);
        Ok(self)
    }

    pub fn with_catalog(mut self, catalog: Arc<ModelCatalog>) -> Self {
        self.catalog = Some(catalog);
        self
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn add_provider<P>(mut self, alias: impl Into<String>, provider: P) -> Self
    where
        P: Provider + 'static,
    {
        let alias = alias.into();
        let format = provider.format();
        self.formats.insert(format, alias.clone());
        self.providers.insert(alias, Arc::new(provider));
        self
    }

    /// Add a pre-wrapped provider (for use with `create_provider()`).
    pub fn add_provider_arc(
        mut self,
        alias: impl Into<String>,
        provider: Arc<dyn Provider>,
    ) -> Self {
        let alias = alias.into();
        let format = provider.format();
        self.formats.insert(format, alias.clone());
        self.providers.insert(alias, provider);
        self
    }

    pub fn add_auth(mut self, alias: impl Into<String>, auth: AuthConfig) -> Self {
        self.auth_configs.insert(alias.into(), auth);
        self
    }

    pub fn add_api_key(mut self, alias: impl Into<String>, key: impl Into<String>) -> Self {
        self.auth_configs.insert(
            alias.into(),
            AuthConfig::ApiKey {
                key: key.into(),
                header: Some("authorization".into()),
                prefix: Some("Bearer".into()),
            },
        );
        self
    }

    pub fn build(self) -> Result<Router> {
        let catalog = self
            .catalog
            .ok_or_else(|| Error::InvalidRequest("model catalog not configured".into()))?;
        let resolver = ModelResolver::new(Arc::clone(&catalog));

        Ok(Router {
            catalog,
            resolver,
            providers: self.providers,
            formats: self.formats,
            auth_configs: self.auth_configs,
            retry_policy: self.retry_policy,
        })
    }
}
