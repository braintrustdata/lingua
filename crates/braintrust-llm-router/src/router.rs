use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "tracing")]
use tracing::Instrument;

use bytes::Bytes;

use crate::auth::AuthConfig;
use crate::catalog::{load_catalog_from_disk, ModelCatalog, ModelResolver, ModelSpec};
use crate::error::{Error, Result};
use crate::providers::{
    enable_streaming_payload, prepare_bedrock_request, requires_bedrock_request_preparation,
    ClientHeaders, Provider,
};
use crate::retry::{RetryPolicy, RetryStrategy};
use crate::streaming::{transform_stream, ResponseStream};
use lingua::serde_json::Value;
use lingua::ProviderFormat;
use lingua::{TransformError, TransformResult};

// Re-export for convenience in dependent crates
pub use lingua::{extract_request_hints, RequestHints};
use reqwest::Url;

use crate::providers::{
    is_openai_compatible, AnthropicProvider, AzureProvider, BedrockProvider, DatabricksProvider,
    GoogleProvider, MistralProvider, OpenAIProvider, VertexProvider,
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
        "databricks" => Ok(Arc::new(DatabricksProvider::from_config(
            endpoint, timeout,
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
    String, // alias
    Arc<dyn Provider>,
    &'a AuthConfig,
    Arc<ModelSpec>,
    ProviderFormat,
    RetryStrategy,
);

/// Metadata about how an incoming request was interpreted, mainly to be used
/// for observability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterMetadata {
    /// The detected format of the incoming request payload.
    ///
    /// When the request was already in the target provider's format, this reflects
    /// that format. When the payload was transformed, this is the source format
    /// that was detected.
    pub detected_input_format: ProviderFormat,

    /// The alias of the provider that was used to execute the request.
    pub provider_alias: String,

    /// The output format of the provider response, which may not always match
    /// the requested output format.
    pub provider_format: ProviderFormat,
}

/// Request prepared by the router and ready for execution.
pub struct PreparedRequest<'a> {
    inner: PreparedRequestInner<'a>,
}

/// Streaming request prepared by the router and ready for execution.
pub struct PreparedStreamRequest<'a> {
    inner: PreparedRequestInner<'a>,
}

struct PreparedRequestInner<'a> {
    provider: Arc<dyn Provider>,
    auth: &'a AuthConfig,
    spec: Arc<ModelSpec>,
    format: ProviderFormat,
    payload: Bytes,
    output_format: ProviderFormat,
    strategy: RetryStrategy,
}

async fn prepare_provider_request(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    stream: bool,
) -> Result<(Bytes, Option<ProviderFormat>)> {
    if requires_bedrock_request_preparation(format) {
        let bytes = prepare_bedrock_request(body, spec, format).await?;
        return Ok((bytes, Some(format)));
    }

    let (transformed, detected_format) =
        match lingua::transform_request(body.clone(), format, Some(&spec.model)) {
            Ok(TransformResult::PassThrough(bytes)) => (bytes, None),
            Ok(TransformResult::Transformed {
                bytes,
                source_format,
            }) => (bytes, Some(source_format)),
            Err(TransformError::UnsupportedTargetFormat(_)) => (body, None),
            Err(err) => return Err(err.into()),
        };

    if stream {
        // TODO: Fold streaming intent into `lingua::transform_request` once we
        // are ready to update its Rust/WASM/Python/TS call sites together.
        Ok((
            enable_streaming_payload(transformed, format),
            detected_format,
        ))
    } else {
        Ok((transformed, detected_format))
    }
}

pub struct Router {
    catalog: Arc<ModelCatalog>,
    resolver: ModelResolver,
    providers: HashMap<String, Arc<dyn Provider>>, // alias -> provider
    auth_configs: HashMap<String, AuthConfig>,     // alias -> auth
    formats: HashMap<ProviderFormat, String>,      // format -> default alias
    retry_policy: RetryPolicy,
}

impl Router {
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    pub fn catalog(&self) -> Arc<ModelCatalog> {
        Arc::clone(&self.catalog)
    }

    // Internal method to create a prepared request, handles streaming and non-streaming requests.
    async fn create_prepared_request_internal(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
        stream: bool,
    ) -> Result<(PreparedRequestInner<'_>, RouterMetadata)> {
        let routes = self.resolve_providers(model, output_format)?;
        let route = routes
            .first()
            .ok_or_else(|| Error::NoProvider(output_format))?;
        let (provider_alias, provider, auth, spec, format, strategy) = route;
        let (payload, detected_format) =
            prepare_provider_request(body, spec.as_ref(), *format, stream).await?;
        Ok((
            PreparedRequestInner {
                provider: provider.clone(),
                auth,
                spec: spec.clone(),
                format: *format,
                payload,
                output_format,
                strategy: strategy.clone(),
            },
            RouterMetadata {
                detected_input_format: detected_format.unwrap_or(*format),
                provider_alias: provider_alias.clone(),
                provider_format: *format,
            },
        ))
    }

    /// Create a prepared request from raw body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// The body will be automatically transformed to the selected provider format if needed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.create_request",
            skip(self, body),
            fields(llm.model = %model)
        )
    )]
    pub async fn create_request(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<(PreparedRequest<'_>, RouterMetadata)> {
        let (inner, metadata) = self
            .create_prepared_request_internal(body, model, output_format, false)
            .await?;
        Ok((PreparedRequest { inner }, metadata))
    }

    /// Execute a prepared request and return transformed response bytes.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete",
            skip(self, request, client_headers),
            fields(llm.model = %request.inner.spec.model)
        )
    )]
    pub async fn complete(
        &self,
        request: PreparedRequest<'_>,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let PreparedRequestInner {
            provider,
            auth,
            spec,
            format,
            payload,
            output_format,
            strategy,
        } = request.inner;
        let response_bytes = self
            .execute_with_retry(
                provider,
                auth,
                spec,
                format,
                payload,
                strategy,
                client_headers,
            )
            .await?;
        let result = lingua::transform_response(response_bytes.clone(), output_format)?;
        let response = match result {
            TransformResult::PassThrough(bytes) => bytes,
            TransformResult::Transformed { bytes, .. } => bytes,
        };
        Ok(response)
    }

    /// Create a prepared streaming request from raw body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// The body will be automatically transformed to the selected provider format if needed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.create_stream_request",
            skip(self, body),
            fields(llm.model = %model)
        )
    )]
    pub async fn create_stream_request(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<(PreparedStreamRequest<'_>, RouterMetadata)> {
        let (inner, metadata) = self
            .create_prepared_request_internal(body, model, output_format, true)
            .await?;
        Ok((PreparedStreamRequest { inner }, metadata))
    }

    /// Execute a prepared streaming request.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete_stream",
            skip(self, request, client_headers),
            fields(llm.model = %request.inner.spec.model)
        )
    )]
    pub async fn complete_stream(
        &self,
        request: PreparedStreamRequest<'_>,
        client_headers: &ClientHeaders,
    ) -> Result<ResponseStream> {
        let PreparedRequestInner {
            provider,
            auth,
            spec,
            format,
            payload,
            output_format,
            strategy: _,
        } = request.inner;
        let raw_stream = provider
            .clone()
            .complete_stream(payload, auth, spec.as_ref(), format, client_headers)
            .await?;
        Ok(transform_stream(raw_stream, output_format))
    }

    /// Resolve all providers for a given model and output format.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// # Returns
    /// A vector of resolved routes, one for each provider. Returns routes in
    /// priority order.
    fn resolve_providers(
        &self,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Vec<ResolvedRoute<'_>>> {
        let (spec, catalog_format, aliases) = self.resolver.resolve(model)?;
        let routes: Vec<Result<ResolvedRoute<'_>>> = aliases
            .iter()
            .map(|alias| {
                self.resolve_provider(
                    output_format,
                    spec.clone(),
                    catalog_format,
                    alias.to_string(),
                )
            })
            .collect();
        let mut first_error = None;
        let successes: Vec<ResolvedRoute<'_>> = routes
            .into_iter()
            .filter_map(|r| match r {
                Ok(s) => Some(s),
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                    None
                }
            })
            .collect();
        #[cfg(feature = "tracing")]
        if let Some(error) = first_error.as_ref() {
            if successes.is_empty() {
                tracing::warn!(
                    model = %model,
                    output_format = ?output_format,
                    all_aliases = ?aliases,
                    spec = ?spec,
                    catalog_format = ?catalog_format,
                    error = %error,
                    "failed to resolve any provider aliases",
                );
            }
        }
        if successes.is_empty() {
            if let Some(fallback_alias) = self.formats.get(&catalog_format).cloned() {
                match self.resolve_provider(
                    output_format,
                    spec,
                    catalog_format,
                    fallback_alias.clone(),
                ) {
                    Ok(route) => return Ok(vec![route]),
                    Err(fallback_error) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            model,
                            aliases = ?aliases,
                            fallback_alias = %fallback_alias,
                            error = %fallback_error,
                            "format fallback failed",
                        );
                        return Err(fallback_error);
                    }
                }
            }
            #[cfg(feature = "tracing")]
            tracing::warn!(
                model,
                aliases = ?aliases,
                "no providers found for model",
            );
            return Err(first_error.unwrap_or_else(|| Error::NoProvider(catalog_format)));
        }
        Ok(successes)
    }

    fn resolve_provider(
        &self,
        output_format: ProviderFormat,
        spec: Arc<ModelSpec>,
        catalog_format: ProviderFormat,
        alias: String,
    ) -> Result<ResolvedRoute<'_>> {
        #[cfg(feature = "tracing")]
        let registered: Vec<&str> = self.providers.keys().map(String::as_str).collect();
        if !self.providers.contains_key(alias.as_str()) {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                resolver_alias = %alias,
                format = ?catalog_format,
                registered = ?registered,
                "resolver alias not found in providers"
            );
            return Err(Error::NoProvider(catalog_format));
        }
        let provider = self.providers.get(&alias).cloned().ok_or_else(|| {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                alias = %alias,
                format = ?catalog_format,
                registered = ?registered,
                "no provider found for resolved alias"
            );
            Error::NoProvider(catalog_format)
        })?;
        let provider_formats = provider.provider_formats();
        let format = if output_format == ProviderFormat::ChatCompletions
            && provider_formats.contains(&ProviderFormat::Responses)
            && spec.requires_responses_api()
        {
            ProviderFormat::Responses
        } else if provider.id() == "azure" && catalog_format == ProviderFormat::Anthropic {
            // Anthropic on Azure only supports the messages format and isn’t
            // interchangeable with other APIs.
            ProviderFormat::Anthropic
        } else if output_format != catalog_format && provider_formats.contains(&output_format) {
            output_format
        } else {
            catalog_format
        };
        let auth = self
            .auth_configs
            .get(&alias)
            .ok_or_else(|| Error::NoAuth(alias.clone()))?;
        let strategy = self.retry_policy.strategy();
        Ok((alias, provider, auth, spec, format, strategy))
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_with_retry(
        &self,
        provider: Arc<dyn Provider>,
        auth: &AuthConfig,
        spec: Arc<ModelSpec>,
        format: ProviderFormat,
        payload: Bytes,
        mut strategy: RetryStrategy,
        client_headers: &ClientHeaders,
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
                    http.url = tracing::field::Empty,
                    http.status_code = tracing::field::Empty,
                );
                async {
                    provider
                        .complete(payload.clone(), auth, &spec, format, client_headers)
                        .await
                }
                .instrument(span)
                .await
            };

            #[cfg(not(feature = "tracing"))]
            let result = provider
                .complete(payload.clone(), auth, &spec, format, client_headers)
                .await;

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
                        return Err(match err {
                            Error::Http(source) => Error::UpstreamUnavailable {
                                provider: provider.id().to_string(),
                                source: source.into(),
                            },
                            Error::Middleware(source) => Error::UpstreamUnavailable {
                                provider: provider.id().to_string(),
                                source: source.into(),
                            },
                            other => other,
                        });
                    }
                }
            }
        }
    }
}

/// One provider registration: alias, provider, auth, and default formats.
struct ProviderEntry {
    alias: String,
    provider: Arc<dyn Provider>,
    auth: AuthConfig,
    default_for_formats: Vec<ProviderFormat>,
}

pub struct RouterBuilder {
    catalog: Option<Arc<ModelCatalog>>,
    custom_catalog: Option<ModelCatalog>,
    provider_entries: Vec<ProviderEntry>,
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
            catalog: None,
            custom_catalog: None,
            provider_entries: Vec::new(),
            retry_policy: RetryPolicy::default(),
        }
    }

    pub fn load_models(mut self, path: impl AsRef<std::path::Path>) -> Result<Self> {
        let catalog = load_catalog_from_disk(path)?;
        self.catalog = Some(catalog);
        self.custom_catalog = None;
        Ok(self)
    }

    pub fn with_catalog(mut self, catalog: Arc<ModelCatalog>) -> Self {
        self.catalog = Some(catalog);
        self.custom_catalog = None;
        self
    }

    /// Configure the router with custom models overlaid on a shared base catalog.
    ///
    /// Custom entries shadow base entries for resolution, while `Router::catalog()`
    /// continues to expose the base catalog for compatibility.
    pub fn with_overlay_catalog(mut self, base: Arc<ModelCatalog>, custom: ModelCatalog) -> Self {
        self.catalog = Some(base);
        self.custom_catalog = Some(custom);
        self
    }

    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    pub fn add_provider<P>(
        mut self,
        alias: impl Into<String>,
        provider: P,
        auth: AuthConfig,
        default_for_formats: Vec<ProviderFormat>,
    ) -> Self
    where
        P: Provider + 'static,
    {
        self.provider_entries.push(ProviderEntry {
            alias: alias.into(),
            provider: Arc::new(provider),
            auth,
            default_for_formats,
        });
        self
    }

    /// Add a pre-wrapped provider (for use with `create_provider()`).
    pub fn add_provider_arc(
        mut self,
        alias: impl Into<String>,
        provider: Arc<dyn Provider>,
        auth: AuthConfig,
        default_for_formats: Vec<ProviderFormat>,
    ) -> Self {
        self.provider_entries.push(ProviderEntry {
            alias: alias.into(),
            provider,
            auth,
            default_for_formats,
        });
        self
    }

    pub fn build(self) -> Result<Router> {
        let catalog = self
            .catalog
            .ok_or_else(|| Error::InvalidRequest("model catalog not configured".into()))?;
        let resolver = match self.custom_catalog {
            Some(custom) => ModelResolver::with_overlay(Arc::clone(&catalog), custom),
            None => ModelResolver::new(Arc::clone(&catalog)),
        };

        let mut providers = HashMap::new();
        let mut auth_configs = HashMap::new();
        let mut formats = HashMap::new();
        let mut backup_formats = HashMap::new();
        for entry in self.provider_entries {
            if providers.contains_key(&entry.alias) {
                return Err(Error::InvalidRequest(format!(
                    "provider alias already exists: {}",
                    entry.alias
                )));
            }
            providers.insert(entry.alias.clone(), entry.provider.clone());
            auth_configs.insert(entry.alias.clone(), entry.auth);

            for format in entry.default_for_formats {
                if let Some(existing_alias) = formats.get(&format) {
                    return Err(Error::InvalidRequest(format!(
                        "format already has a default provider: {format}: {existing_alias}, {}",
                        entry.alias
                    )));
                }
                formats.insert(format, entry.alias.clone());
            }
            for format in entry.provider.provider_formats() {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    "adding backup format: {format} -> {alias}",
                    alias = entry.alias
                );
                backup_formats
                    .entry(format)
                    .or_insert_with(|| entry.alias.clone());
            }
        }
        // Do a second pass to find any formats with no default provider, and
        // add a default provider chosen at random.
        for (format, alias) in backup_formats {
            formats.entry(format).or_insert(alias);
        }

        Ok(Router {
            catalog,
            resolver,
            providers,
            formats,
            auth_configs,
            retry_policy: self.retry_policy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelCatalog, ModelFlavor, ModelSpec};
    use crate::error::Error;
    use crate::streaming::RawResponseStream;
    use async_trait::async_trait;
    use reqwest::header::HeaderMap;

    struct FakeProvider {
        name: &'static str,
        formats: Vec<ProviderFormat>,
    }

    #[async_trait]
    impl Provider for FakeProvider {
        fn id(&self) -> &'static str {
            self.name
        }

        fn provider_formats(&self) -> Vec<ProviderFormat> {
            self.formats.clone()
        }

        async fn complete(
            &self,
            _payload: Bytes,
            _auth: &AuthConfig,
            _spec: &ModelSpec,
            _format: ProviderFormat,
            _client_headers: &ClientHeaders,
        ) -> Result<Bytes> {
            Ok(Bytes::from("{}"))
        }

        async fn complete_stream(
            &self,
            _payload: Bytes,
            _auth: &AuthConfig,
            _spec: &ModelSpec,
            _format: ProviderFormat,
            _client_headers: &ClientHeaders,
        ) -> Result<RawResponseStream> {
            unimplemented!()
        }

        async fn health_check(&self, _auth: &AuthConfig) -> Result<()> {
            Ok(())
        }

        fn build_headers(&self, client_headers: &ClientHeaders) -> HeaderMap {
            client_headers.to_json_headers()
        }
    }

    fn google_spec(model: &str) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format: ProviderFormat::Google,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
            available_providers: Default::default(),
        }
    }

    fn openai_spec(model: &str, flavor: ModelFlavor) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format: ProviderFormat::ChatCompletions,
            flavor,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
            available_providers: Default::default(),
        }
    }

    fn openai_spec_with_available_providers(model: &str, flavor: ModelFlavor) -> ModelSpec {
        let mut spec = openai_spec(model, flavor);
        spec.available_providers = vec!["openai".into(), "azure".into(), "cerebras".into()];
        spec
    }

    fn resolved_aliases(
        router: &Router,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Vec<String>> {
        router
            .resolve_providers(model, output_format)
            .map(|routes| {
                routes
                    .into_iter()
                    .map(|(alias, _, _, _, _, _)| alias)
                    .collect()
            })
    }

    #[tokio::test]
    async fn prepare_provider_request_enables_stream_for_google_to_chat_completions() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-5-mini","contents":[{"role":"user","parts":[{"text":"hello"}]}]}"#,
        );
        let spec = openai_spec("gpt-5-mini", ModelFlavor::Chat);

        let (payload, _) =
            prepare_provider_request(body, &spec, ProviderFormat::ChatCompletions, true)
                .await
                .expect("request prepares");

        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");
        assert_eq!(parsed.get("stream"), Some(&Value::Bool(true)));
        assert_eq!(parsed.get("stream_options"), None);
    }

    #[tokio::test]
    async fn prepare_provider_request_leaves_non_streaming_google_to_chat_completions_without_stream_flag(
    ) {
        let body = Bytes::from_static(
            br#"{"model":"gpt-5-mini","contents":[{"role":"user","parts":[{"text":"hello"}]}]}"#,
        );
        let spec = openai_spec("gpt-5-mini", ModelFlavor::Chat);

        let (payload, _) =
            prepare_provider_request(body, &spec, ProviderFormat::ChatCompletions, false)
                .await
                .expect("request prepares");

        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");
        assert_eq!(parsed.get("stream"), None);
        assert_eq!(parsed.get("stream_options"), None);
    }

    fn dummy_auth() -> AuthConfig {
        AuthConfig::ApiKey {
            key: "test".into(),
            header: Some("authorization".into()),
            prefix: Some("Bearer".into()),
        }
    }

    #[test]
    fn vertex_model_routes_to_vertex_provider() {
        let vertex_model = "publishers/google/models/gemini-2.5-flash-preview-04-17";
        let google_model = "gemini-2.5-flash";

        let mut catalog = ModelCatalog::empty();
        catalog.insert(vertex_model.into(), google_spec(vertex_model));
        catalog.insert(google_model.into(), google_spec(google_model));

        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .add_provider(
                "vertex",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::Google],
                },
                dummy_auth(),
                vec![], // not default; vertex models resolved via catalog
            )
            .build()
            .expect("router builds");

        assert_eq!(
            resolved_aliases(&router, vertex_model, ProviderFormat::Google).unwrap(),
            vec!["vertex".to_string()]
        );
        assert_eq!(
            resolved_aliases(&router, google_model, ProviderFormat::Google).unwrap(),
            vec!["google".to_string()]
        );
    }

    #[test]
    fn vertex_model_falls_back_to_google_when_no_vertex_provider() {
        let vertex_model = "publishers/google/models/gemini-pro";

        let mut catalog = ModelCatalog::empty();
        catalog.insert(vertex_model.into(), google_spec(vertex_model));

        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");

        assert_eq!(
            resolved_aliases(&router, vertex_model, ProviderFormat::Google).unwrap(),
            vec!["google".to_string()]
        );
    }

    #[test]
    fn responses_required_model_forces_responses_format_for_chat_output() {
        let model = "gpt-5-pro";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format, _) = routes[0];
        assert_eq!(format, ProviderFormat::Responses);
    }

    #[test]
    fn codex_variant_forces_responses_format_for_chat_output() {
        let model = "gpt-5.1-codex";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format, _) = routes[0];
        assert_eq!(format, ProviderFormat::Responses);
    }

    #[test]
    fn non_responses_model_keeps_chat_completions_format() {
        let model = "gpt-5-mini";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format, _) = routes[0];
        assert_eq!(format, ProviderFormat::ChatCompletions);
    }

    #[test]
    fn responses_required_model_without_responses_support_stays_chat_completions() {
        let model = "gpt-5-pro";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format, _) = routes[0];
        assert_eq!(format, ProviderFormat::ChatCompletions);
    }

    #[test]
    fn responses_required_model_falls_back_to_azure_provider() {
        let model = "gpt-5-pro";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "azure",
                FakeProvider {
                    name: "azure",
                    formats: vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (alias, provider, _, _, format, _) = &routes[0];
        assert_eq!(alias, "azure");
        assert_eq!(provider.id(), "azure");
        assert_eq!(*format, ProviderFormat::Responses);
    }

    #[test]
    fn build_fails_when_provider_alias_duplicated() {
        let catalog = Arc::new(ModelCatalog::empty());
        let result = Router::builder()
            .with_catalog(Arc::clone(&catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai2",
                    formats: vec![],
                },
                dummy_auth(),
                vec![],
            )
            .build();
        let err = match result {
            Ok(_) => panic!("expected duplicate alias error"),
            Err(e) => e,
        };
        assert!(
            matches!(err, Error::InvalidRequest(ref msg) if msg.contains("provider alias already exists") && msg.contains("openai")),
            "expected InvalidRequest about duplicate alias, got: {err:?}"
        );
    }

    #[test]
    fn build_fails_when_format_has_two_default_providers() {
        let catalog = Arc::new(ModelCatalog::empty());
        let result = Router::builder()
            .with_catalog(Arc::clone(&catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .add_provider(
                "openai2",
                FakeProvider {
                    name: "openai2",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build();
        let err = match result {
            Ok(_) => panic!("expected duplicate default for format error"),
            Err(e) => e,
        };
        assert!(
            matches!(err, Error::InvalidRequest(ref msg) if msg.contains("format already has a default provider") && msg.contains("openai") && msg.contains("openai2")),
            "expected InvalidRequest about format already has default, got: {err:?}"
        );
    }

    #[test]
    fn build_succeeds_when_same_format_multiple_providers_only_one_default() {
        let catalog = Arc::new(ModelCatalog::empty());
        let router = Router::builder()
            .with_catalog(Arc::clone(&catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .add_provider(
                "openai2",
                FakeProvider {
                    name: "openai2",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![], // not default
            )
            .build()
            .expect("router builds");
        assert!(router.catalog().get("any").is_none());
    }

    #[test]
    fn overlay_catalog_resolves_custom_and_base_models() {
        let mut base = ModelCatalog::empty();
        base.insert(
            "base-model".into(),
            openai_spec("base-model", ModelFlavor::Chat),
        );
        let mut custom = ModelCatalog::empty();
        custom.insert(
            "custom-model".into(),
            openai_spec_with_available_providers("custom-model", ModelFlavor::Chat),
        );

        let router = Router::builder()
            .with_overlay_catalog(Arc::new(base), custom)
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build()
            .expect("router builds");

        assert!(router.catalog().get("base-model").is_some());
        assert!(router.catalog().get("custom-model").is_none());
        assert!(router
            .resolve_providers("base-model", ProviderFormat::ChatCompletions)
            .is_ok());
        assert!(router
            .resolve_providers("custom-model", ProviderFormat::ChatCompletions)
            .is_ok());
    }

    #[test]
    fn resolved_aliases_returns_only_registered_available_providers() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(
            model.into(),
            openai_spec_with_available_providers(model, ModelFlavor::Chat),
        );
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .add_provider(
                "azure",
                FakeProvider {
                    name: "azure",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let aliases = resolved_aliases(&router, model, ProviderFormat::ChatCompletions)
            .expect("resolved aliases");
        assert_eq!(aliases, vec!["openai".to_string(), "azure".to_string()]);
    }

    #[test]
    fn resolve_providers_falls_back_to_format_slot_when_alias_not_registered() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(
            model.into(),
            openai_spec_with_available_providers(model, ModelFlavor::Chat),
        );
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "other_gpt",
                FakeProvider {
                    name: "other_gpt",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert!(
            !routes.is_empty(),
            "at least one route (unregistered openai falls back to format slot azure)"
        );
        assert_eq!(
            resolved_aliases(&router, model, ProviderFormat::ChatCompletions).unwrap(),
            vec!["other_gpt".to_string()],
            "resolving providers returns only registered aliases from available_providers"
        );
    }
}
