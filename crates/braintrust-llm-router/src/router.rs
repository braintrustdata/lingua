use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "tracing")]
use tracing::Instrument;

use bytes::Bytes;

use crate::auth::AuthConfig;
use crate::catalog::{load_catalog_from_disk, ModelCatalog, ModelResolver, ModelSpec};
use crate::error::{Error, Result};
use crate::providers::{ClientHeaders, Provider};
use crate::retry::{RetryPolicy, RetryStrategy};
use crate::streaming::{transform_stream, ResponseStream};
use lingua::processing::{adapter_for_format, adapters};
use lingua::serde_json::Value;
use lingua::universal::message::{Message, UserContent, UserContentPart};
use lingua::util::media::MediaBlock;
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
    String,
    Arc<dyn Provider>,
    &'a AuthConfig,
    Arc<ModelSpec>,
    ProviderFormat,
    RetryStrategy,
);

fn is_remote_image_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

async fn fetch_remote_image_as_base64(url: &str) -> Result<MediaBlock> {
    lingua::util::media::convert_media_to_base64(url, None, None)
        .await
        .map_err(|e| Error::InvalidRequest(format!("failed to fetch image URL {url}: {e}")))
}

type FetchMediaFuture<'a> = Pin<Box<dyn Future<Output = Result<MediaBlock>> + Send + 'a>>;

// Preserve the legacy proxy behavior that fetched remote image URLs before
// Bedrock request translation, since Converse and Bedrock Anthropic expect
// inline media rather than raw remote URLs.
fn should_inline_remote_image_urls(format: ProviderFormat) -> bool {
    matches!(
        format,
        ProviderFormat::BedrockAnthropic | ProviderFormat::Converse
    )
}

async fn inline_remote_image_urls_with_fetch<F>(
    request: &mut lingua::UniversalRequest,
    mut fetch: F,
) -> Result<()>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    for message in &mut request.messages {
        let content = match message {
            Message::System { content }
            | Message::Developer { content }
            | Message::User { content } => content,
            Message::Assistant { .. } | Message::Tool { .. } => continue,
        };

        let UserContent::Array(parts) = content else {
            continue;
        };

        for part in parts {
            let UserContentPart::Image {
                image, media_type, ..
            } = part
            else {
                continue;
            };

            let Some(url) = image.as_str() else {
                continue;
            };

            if !is_remote_image_url(url) {
                continue;
            }

            let media_block = fetch(url).await?;
            *image = lingua::serde_json::Value::String(media_block.data);
            *media_type = Some(media_block.media_type);
        }
    }

    Ok(())
}

async fn prepare_request_with_remote_image_inlining<F>(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    fetch: F,
) -> Result<Bytes>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    let payload: lingua::serde_json::Value = match lingua::serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(err) => {
            return Err(TransformError::DeserializationFailed(err.to_string()).into());
        }
    };

    let source_adapter = match adapters()
        .iter()
        .map(|adapter| adapter.as_ref())
        .find(|adapter| adapter.detect_request(&payload))
    {
        Some(adapter) => adapter,
        None => return Err(TransformError::UnableToDetectFormat.into()),
    };

    if source_adapter.format() == format {
        return Ok(body);
    }

    let mut request = match source_adapter.request_to_universal(payload) {
        Ok(request) => request,
        Err(err) => return Err(err.into()),
    };

    inline_remote_image_urls_with_fetch(&mut request, fetch).await?;

    if request.model.is_none() {
        request.model = Some(spec.model.clone());
    }

    let target_adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    target_adapter.apply_defaults(&mut request);
    let prepared = target_adapter.request_from_universal(&request)?;

    lingua::serde_json::to_vec(&prepared)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)
}

async fn prepare_provider_request(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
) -> Result<Bytes> {
    if should_inline_remote_image_urls(format) {
        return prepare_request_with_remote_image_inlining(body, spec, format, |url| {
            Box::pin(fetch_remote_image_as_base64(url))
        })
        .await;
    }

    match lingua::transform_request(body.clone(), format, Some(&spec.model)) {
        Ok(TransformResult::PassThrough(bytes)) => Ok(bytes),
        Ok(TransformResult::Transformed { bytes, .. }) => Ok(bytes),
        Err(TransformError::UnsupportedTargetFormat(_)) => Ok(body),
        Err(err) => Err(err.into()),
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

    /// Execute a completion request with the given body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    /// * `client_headers` - Client headers to forward to the upstream provider
    ///
    /// The body will be automatically transformed to the target provider's format if needed.
    /// The response will be converted to the requested output format.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete",
            skip(self, body, client_headers),
            fields(llm.model = %model)
        )
    )]
    pub async fn complete(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let routes = self.resolve_providers(model, output_format)?;
        // Choose the first provider
        let route = routes
            .first()
            .ok_or_else(|| Error::NoProvider(output_format))?;
        let (_, provider, auth, spec, format, strategy) = route;
        let payload = prepare_provider_request(body, spec.as_ref(), *format).await?;

        let response_bytes = self
            .execute_with_retry(
                provider.clone(),
                auth,
                spec.clone(),
                *format,
                payload,
                strategy.clone(),
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

    /// Execute a streaming completion request with the given body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    /// * `client_headers` - Client headers to forward to the upstream provider
    ///
    /// The body will be automatically transformed to the target provider's format if needed.
    /// Stream chunks will be transformed to the requested output format.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete_stream",
            skip(self, body, client_headers),
            fields(llm.model = %model, http.url = tracing::field::Empty, http.status_code = tracing::field::Empty)
        )
    )]
    pub async fn complete_stream(
        &self,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<ResponseStream> {
        let routes = self.resolve_providers(model, output_format)?;
        let route = routes
            .first()
            .ok_or_else(|| Error::NoProvider(output_format))?;
        let (_, provider, auth, spec, format, _) = route;
        let payload = prepare_provider_request(body, spec.as_ref(), *format).await?;

        let raw_stream = provider
            .clone()
            .complete_stream(payload, auth, spec.as_ref(), *format, client_headers)
            .await?;

        Ok(transform_stream(raw_stream, output_format))
    }

    /// Get the aliases of the providers that can handle the given model and output format.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name for routing (e.g., "gpt-4", "claude-3-opus")
    /// * `output_format` - The output format, or None to auto-detect from body
    ///
    /// # Returns
    /// A vector of provider aliases that can handle the given model and output
    /// format. The aliases are in priority order. Follows the same order as the
    /// complete and complete_stream methods.
    pub fn provider_aliases(
        &self,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Vec<String>> {
        self.resolve_providers(model, output_format)
            .map(|routes| routes.into_iter().map(|(alias, ..)| alias).collect())
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
                        return Err(err);
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
            provider_entries: Vec::new(),
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
        let resolver = ModelResolver::new(Arc::clone(&catalog));

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

    fn bedrock_spec(model: &str, format: ProviderFormat) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format,
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

    fn dummy_auth() -> AuthConfig {
        AuthConfig::ApiKey {
            key: "test".into(),
            header: Some("authorization".into()),
            prefix: Some("Bearer".into()),
        }
    }

    #[test]
    fn should_inline_remote_image_urls_matches_legacy_proxy_formats() {
        assert!(should_inline_remote_image_urls(
            ProviderFormat::BedrockAnthropic
        ));
        assert!(should_inline_remote_image_urls(ProviderFormat::Converse));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Anthropic));
        assert!(!should_inline_remote_image_urls(
            ProviderFormat::ChatCompletions
        ));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Responses));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Google));
    }

    #[tokio::test]
    async fn prepare_request_passes_through_same_format_converse_without_fetch() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "modelId": "anthropic.claude-3-haiku-20240307-v1:0",
                "messages": [{
                    "role": "user",
                    "content": [{"text": "Hello"}]
                }]
            }))
            .unwrap(),
        );

        let prepared = prepare_request_with_remote_image_inlining(
            body.clone(),
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::Converse,
            ),
            ProviderFormat::Converse,
            |_url| {
                Box::pin(async {
                    panic!("fetch should not be called for same-format converse requests");
                })
            },
        )
        .await
        .unwrap();

        assert_eq!(prepared, body);
    }

    #[tokio::test]
    async fn prepare_request_inlines_remote_chat_image_for_converse() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "What is this?"},
                        {"type": "image_url", "image_url": {"url": "https://example.com/image.jpg"}}
                    ]
                }]
            }))
            .unwrap(),
        );

        let prepared = prepare_request_with_remote_image_inlining(
            body,
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::Converse,
            ),
            ProviderFormat::Converse,
            |_url| {
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "image/jpeg".to_string(),
                        data: "abcd".to_string(),
                    })
                })
            },
        )
        .await
        .unwrap();
        let value: lingua::serde_json::Value = lingua::serde_json::from_slice(&prepared).unwrap();

        let bytes = value
            .pointer("/messages/0/content/1/image/source/bytes")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(
            value
                .pointer("/messages/0/content/1/image/format")
                .and_then(|v| v.as_str()),
            Some("jpeg")
        );
    }

    #[tokio::test]
    async fn prepare_request_inlines_remote_responses_image_for_bedrock_anthropic() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "model": "claude-sonnet-4-5-20250929",
                "input": [{
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "What is this?"},
                        {
                            "type": "input_image",
                            "image_url": "https://example.com/image.jpg",
                            "detail": "auto"
                        }
                    ]
                }]
            }))
            .unwrap(),
        );

        let prepared = prepare_request_with_remote_image_inlining(
            body,
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::BedrockAnthropic,
            ),
            ProviderFormat::BedrockAnthropic,
            |_url| {
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "image/jpeg".to_string(),
                        data: "abcd".to_string(),
                    })
                })
            },
        )
        .await
        .unwrap();
        let value: lingua::serde_json::Value = lingua::serde_json::from_slice(&prepared).unwrap();

        assert_eq!(
            value.get("anthropic_version").and_then(|v| v.as_str()),
            Some("bedrock-2023-05-31")
        );
        assert_eq!(
            value
                .pointer("/messages/0/content/1/source/type")
                .and_then(|v| v.as_str()),
            Some("base64")
        );
        assert!(value
            .pointer("/messages/0/content/1/source/data")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty()));
        assert_eq!(value.pointer("/messages/0/content/1/source/url"), None);
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
            router
                .provider_aliases(vertex_model, ProviderFormat::Google)
                .unwrap(),
            vec!["vertex".to_string()]
        );
        assert_eq!(
            router
                .provider_aliases(google_model, ProviderFormat::Google)
                .unwrap(),
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
            router
                .provider_aliases(vertex_model, ProviderFormat::Google)
                .unwrap(),
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
    fn provider_aliases_returns_only_registered_available_providers() {
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

        let aliases = router
            .provider_aliases(model, ProviderFormat::ChatCompletions)
            .expect("provider_aliases");
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
            router
                .provider_aliases(model, ProviderFormat::ChatCompletions)
                .unwrap(),
            vec!["other_gpt".to_string()],
            "provider_aliases returns only registered providers from available_providers"
        );
    }
}
