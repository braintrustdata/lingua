use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "tracing")]
use tracing::Instrument;

use bytes::Bytes;

use crate::auth::AuthConfig;
use crate::catalog::{
    is_gemini_api_model, load_catalog_from_disk, ModelCatalog, ModelResolver, ModelSpec,
};
use crate::client::ClientSettings;
use crate::error::{Error, Result};
use crate::providers::{
    enable_streaming_payload, prepare_bedrock_request, requires_bedrock_request_preparation,
    rewrite_body_model_if_required, ClientHeaders, Provider,
};
use crate::retry::{RetryPolicy, RetryStrategy};
use crate::streaming::{
    transform_stream, transform_stream_with_capture, RawStreamChunkCapture, ResponseStream,
};
use lingua::serde_json::Value;
use lingua::ProviderFormat;
use lingua::{TransformError, TransformResult};
use serde::Deserialize;

// Re-export for convenience in dependent crates
pub use lingua::{extract_request_hints, RequestHints};
use reqwest::Url;

#[derive(Debug, Clone)]
pub struct CompleteResponseWithRaw {
    pub response: Bytes,
    pub raw_response: Bytes,
}

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
/// * `client_settings` - HTTP client settings, including optional DNS pins.
///
/// # Example
///
/// ```ignore
/// use braintrust_llm_router::{create_provider, Router, AuthConfig};
/// use std::collections::HashMap;
///
/// let metadata = HashMap::new();
/// let provider = create_provider("openai", None, None, None, &metadata, None)?;
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
    client_settings: Option<ClientSettings>,
) -> Result<Arc<dyn Provider>> {
    match kind {
        "openai" => Ok(Arc::new(OpenAIProvider::from_config(
            endpoint,
            endpoint_template,
            timeout,
            metadata,
            client_settings,
        )?)),
        "anthropic" => Ok(Arc::new(AnthropicProvider::from_config(
            endpoint,
            timeout,
            metadata,
            client_settings,
        )?)),
        "azure" => Ok(Arc::new(AzureProvider::from_config(
            endpoint,
            timeout,
            metadata,
            client_settings,
        )?)),
        "google" => Ok(Arc::new(GoogleProvider::from_config(
            endpoint,
            timeout,
            client_settings,
        )?)),
        "vertex" => Ok(Arc::new(VertexProvider::from_config(
            endpoint,
            timeout,
            metadata,
            client_settings,
        )?)),
        "bedrock" => Ok(Arc::new(BedrockProvider::from_config(
            endpoint,
            timeout,
            metadata,
            client_settings,
        )?)),
        "databricks" => Ok(Arc::new(DatabricksProvider::from_config(
            endpoint,
            timeout,
            client_settings,
        )?)),
        "mistral" => Ok(Arc::new(MistralProvider::from_config(
            endpoint,
            timeout,
            client_settings,
        )?)),
        kind if is_openai_compatible(kind) => Ok(Arc::new(
            OpenAIProvider::from_config(
                endpoint,
                endpoint_template,
                timeout,
                metadata,
                client_settings,
            )?
            .with_provider_alias(kind.to_ascii_lowercase()),
        )),
        other => Err(Error::InvalidRequest(format!(
            "unsupported provider kind: {other}"
        ))),
    }
}

/// Resolved route information from model resolution.
#[derive(Clone)]
pub struct ProviderRoute {
    provider_alias: String,
    provider: Arc<dyn Provider>,
    auth: AuthConfig,
    spec: Arc<ModelSpec>,
    format: ProviderFormat,
}

impl ProviderRoute {
    pub fn provider_alias(&self) -> &str {
        self.provider_alias.as_str()
    }

    pub fn model(&self) -> &str {
        self.spec.model.as_str()
    }
}

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

    /// Whether Lingua forwarded the request body without transforming it.
    pub lingua_passthrough: bool,

    /// The alias of the provider that was used to execute the request.
    pub provider_alias: String,

    /// The output format of the provider response, which may not always match
    /// the requested output format.
    pub provider_format: ProviderFormat,
}

#[cfg(test)]
type ResolvedProviderForTest = (
    String,
    Arc<dyn Provider>,
    AuthConfig,
    Arc<ModelSpec>,
    ProviderFormat,
);

/// Request prepared by the router and ready for execution.
pub struct PreparedRequest {
    inner: PreparedRequestInner,
}

/// Streaming request prepared by the router and ready for execution.
pub struct PreparedStreamRequest {
    inner: PreparedRequestInner,
}

struct PreparedRequestInner {
    provider: Arc<dyn Provider>,
    auth: AuthConfig,
    spec: Arc<ModelSpec>,
    format: ProviderFormat,
    payload: Bytes,
    output_format: ProviderFormat,
    strategy: RetryStrategy,
}

#[derive(Clone, Copy)]
struct RequestPreparationOptions {
    rewrite_body_model: bool,
}

impl Default for RequestPreparationOptions {
    fn default() -> Self {
        Self {
            rewrite_body_model: true,
        }
    }
}

async fn prepare_provider_request(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    stream: bool,
    options: RequestPreparationOptions,
) -> Result<(Bytes, Option<ProviderFormat>, ProviderFormat, bool)> {
    if requires_bedrock_request_preparation(format) {
        let bytes = prepare_bedrock_request(body, spec, format).await?;
        return Ok((bytes, Some(format), format, false));
    }

    let model_override = options.rewrite_body_model.then_some(spec.model.as_str());
    let (transformed, detected_format, actual_format, maybe_rewrite_model, lingua_passthrough) =
        match lingua::transform_request(body.clone(), format, model_override) {
            Ok(TransformResult::PassThrough(bytes)) => (bytes, None, format, true, true),
            Ok(TransformResult::Transformed {
                bytes,
                source_format,
                actual_target_format,
            }) => (
                bytes,
                Some(source_format),
                actual_target_format,
                false,
                false,
            ),
            Err(TransformError::UnsupportedTargetFormat(_)) => (body, None, format, true, false),
            Err(err) => return Err(err.into()),
        };

    let transformed = if options.rewrite_body_model && maybe_rewrite_model {
        rewrite_body_model_if_required(transformed, actual_format, &spec.model)
    } else {
        transformed
    };

    if stream {
        // TODO: Fold streaming intent into `lingua::transform_request` once we
        // are ready to update its Rust/WASM/Python/TS call sites together.
        Ok((
            enable_streaming_payload(transformed, actual_format),
            detected_format,
            actual_format,
            lingua_passthrough,
        ))
    } else {
        Ok((
            transformed,
            detected_format,
            actual_format,
            lingua_passthrough,
        ))
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
        output_format: ProviderFormat,
        route: &ProviderRoute,
        stream: bool,
        options: RequestPreparationOptions,
    ) -> Result<(PreparedRequestInner, RouterMetadata)> {
        let (payload, detected_format, actual_format, lingua_passthrough) =
            prepare_provider_request(body, route.spec.as_ref(), route.format, stream, options)
                .await?;
        Ok((
            PreparedRequestInner {
                provider: route.provider.clone(),
                auth: route.auth.clone(),
                spec: route.spec.clone(),
                format: actual_format,
                payload,
                output_format,
                strategy: self.retry_policy.strategy(),
            },
            RouterMetadata {
                detected_input_format: detected_format.unwrap_or(route.format),
                lingua_passthrough,
                provider_alias: route.provider_alias.clone(),
                provider_format: actual_format,
            },
        ))
    }

    /// Create a prepared request from raw body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `output_format` - The output format, or None to auto-detect from body
    /// * `route` - The already-resolved provider route to prepare for
    /// * `preserve_body_model` - Keep the request body's model instead of rewriting it to the route model
    ///
    /// The body will be automatically transformed to the selected provider format if needed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.create_request",
            skip(self, body, route),
            fields(llm.model = %route.model())
        )
    )]
    pub async fn create_request(
        &self,
        body: Bytes,
        output_format: ProviderFormat,
        route: &ProviderRoute,
        preserve_body_model: bool,
    ) -> Result<(PreparedRequest, RouterMetadata)> {
        let (inner, metadata) = self
            .create_prepared_request_internal(
                body,
                output_format,
                route,
                false,
                RequestPreparationOptions {
                    rewrite_body_model: !preserve_body_model,
                },
            )
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
        request: PreparedRequest,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        Ok(self
            .complete_internal(request, client_headers)
            .await?
            .response)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete",
            skip(self, request, client_headers),
            fields(llm.model = %request.inner.spec.model)
        )
    )]
    pub async fn complete_with_raw_response(
        &self,
        request: PreparedRequest,
        client_headers: &ClientHeaders,
    ) -> Result<CompleteResponseWithRaw> {
        self.complete_internal(request, client_headers).await
    }

    async fn complete_internal(
        &self,
        request: PreparedRequest,
        client_headers: &ClientHeaders,
    ) -> Result<CompleteResponseWithRaw> {
        let PreparedRequestInner {
            provider,
            auth,
            spec,
            format,
            payload,
            output_format,
            strategy,
        } = request.inner;
        let fallback_response_model = spec.model.clone();
        let response_bytes = self
            .execute_with_retry(
                provider,
                &auth,
                spec,
                format,
                payload,
                strategy,
                client_headers,
            )
            .await?;
        let result = lingua::transform_response(response_bytes.clone(), output_format).map_err(
            |source| Error::ResponseTransform {
                source,
                raw_response: response_bytes.clone(),
            },
        )?;
        let response = match result {
            TransformResult::PassThrough(bytes) => bytes,
            TransformResult::Transformed { bytes, .. } => {
                replace_transformed_response_model(bytes, &fallback_response_model)?
            }
        };
        Ok(CompleteResponseWithRaw {
            response,
            raw_response: response_bytes,
        })
    }

    /// Create a prepared streaming request from raw body bytes.
    ///
    /// # Arguments
    ///
    /// * `body` - Raw request body bytes in any supported format (OpenAI, Anthropic, Google, etc.)
    /// * `output_format` - The output format, or None to auto-detect from body
    /// * `route` - The already-resolved provider route to prepare for
    /// * `preserve_body_model` - Keep the request body's model instead of rewriting it to the route model
    ///
    /// The body will be automatically transformed to the selected provider format if needed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.create_stream_request",
            skip(self, body, route),
            fields(llm.model = %route.model())
        )
    )]
    pub async fn create_stream_request(
        &self,
        body: Bytes,
        output_format: ProviderFormat,
        route: &ProviderRoute,
        preserve_body_model: bool,
    ) -> Result<(PreparedStreamRequest, RouterMetadata)> {
        let (inner, metadata) = self
            .create_prepared_request_internal(
                body,
                output_format,
                route,
                true,
                RequestPreparationOptions {
                    rewrite_body_model: !preserve_body_model,
                },
            )
            .await?;
        Ok((PreparedStreamRequest { inner }, metadata))
    }

    /// Execute a prepared streaming request.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete_stream",
            skip(self, request, client_headers, gateway_request_id),
            fields(llm.model = %request.inner.spec.model)
        )
    )]
    pub async fn complete_stream(
        &self,
        request: PreparedStreamRequest,
        client_headers: &ClientHeaders,
        gateway_request_id: Option<String>,
    ) -> Result<ResponseStream> {
        self.complete_stream_internal(request, client_headers, gateway_request_id, None)
            .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "bt.router.complete_stream",
            skip(self, request, client_headers, gateway_request_id, raw_chunk_capture),
            fields(llm.model = %request.inner.spec.model)
        )
    )]
    pub async fn complete_stream_with_raw_response_capture(
        &self,
        request: PreparedStreamRequest,
        client_headers: &ClientHeaders,
        gateway_request_id: Option<String>,
        raw_chunk_capture: RawStreamChunkCapture,
    ) -> Result<ResponseStream> {
        self.complete_stream_internal(
            request,
            client_headers,
            gateway_request_id,
            Some(raw_chunk_capture),
        )
        .await
    }

    async fn complete_stream_internal(
        &self,
        request: PreparedStreamRequest,
        client_headers: &ClientHeaders,
        gateway_request_id: Option<String>,
        raw_chunk_capture: Option<RawStreamChunkCapture>,
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
        let allow_full_response_fallback = spec.supports_streaming;
        let raw_stream = provider
            .clone()
            .complete_stream(payload, &auth, spec.as_ref(), format, client_headers)
            .await?;
        Ok(match raw_chunk_capture {
            Some(capture) => transform_stream_with_capture(
                raw_stream,
                output_format,
                allow_full_response_fallback,
                gateway_request_id,
                Some(capture),
            ),
            None => transform_stream(
                raw_stream,
                output_format,
                allow_full_response_fallback,
                gateway_request_id,
            ),
        })
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
    pub fn resolve_provider_routes(
        &self,
        model: &str,
        output_format: ProviderFormat,
        fallback_aliases: &[String],
    ) -> Result<Vec<ProviderRoute>> {
        if !fallback_aliases.is_empty() {
            return self.resolve_provider_routes_for_failover(
                model,
                output_format,
                fallback_aliases,
            );
        }

        let (spec, catalog_format, aliases) = self.resolver.resolve(model)?;
        let routes: Vec<Result<ProviderRoute>> = aliases
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
        let mut successes: Vec<ProviderRoute> = routes
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
        if !fallback_aliases.is_empty() {
            let mut seen: HashSet<String> = successes
                .first()
                .map(|route| route.provider_alias.clone())
                .into_iter()
                .collect();
            let mut routes: Vec<ProviderRoute> = successes.into_iter().take(1).collect();
            routes.extend(
                fallback_aliases
                    .iter()
                    .filter(|alias| aliases.contains(alias))
                    .filter(|alias| seen.insert((*alias).clone()))
                    .filter_map(|alias| {
                        self.resolve_provider(
                            output_format,
                            spec.clone(),
                            catalog_format,
                            alias.clone(),
                        )
                        .ok()
                    }),
            );
            successes = routes;
        }
        if successes.is_empty() && fallback_aliases.is_empty() {
            if is_gemini_api_model(model) && catalog_format != ProviderFormat::Google {
                return Err(Error::NoProvider(ProviderFormat::Google));
            }
            if let Some(fallback_alias) = self.formats.get(&catalog_format).cloned() {
                match self.resolve_provider(
                    output_format,
                    spec.clone(),
                    catalog_format,
                    fallback_alias.clone(),
                ) {
                    Ok(route) => successes.push(route),
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
        }
        if successes.is_empty() {
            #[cfg(feature = "tracing")]
            if let Some(error) = first_error.as_ref() {
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

    fn resolve_provider_routes_for_failover(
        &self,
        model: &str,
        output_format: ProviderFormat,
        fallback_aliases: &[String],
    ) -> Result<Vec<ProviderRoute>> {
        let resolved_models = self.resolver.resolve_all_equivalent_model_routes(model)?;
        let (_, first_catalog_format, _) = resolved_models
            .first()
            .ok_or_else(|| Error::UnknownModel(model.to_string()))?;
        let mut first_error = None;
        let mut routes = Vec::new();
        let mut seen = HashSet::new();

        if let Some((spec, catalog_format, aliases)) = resolved_models.first() {
            for alias in aliases {
                match self.resolve_provider(
                    output_format,
                    spec.clone(),
                    *catalog_format,
                    alias.to_string(),
                ) {
                    Ok(route) => {
                        seen.insert(route.provider_alias.clone());
                        routes.push(route);
                        break;
                    }
                    Err(err) => {
                        if first_error.is_none() {
                            first_error = Some(err);
                        }
                    }
                }
            }
        }

        for fallback_alias in fallback_aliases {
            if seen.contains(fallback_alias) {
                continue;
            }

            for (spec, catalog_format, aliases) in &resolved_models {
                if !aliases
                    .iter()
                    .any(|alias| self.alias_matches_provider(alias, fallback_alias))
                {
                    continue;
                }

                match self.resolve_provider(
                    output_format,
                    spec.clone(),
                    *catalog_format,
                    fallback_alias.clone(),
                ) {
                    Ok(route) => {
                        seen.insert(route.provider_alias.clone());
                        routes.push(route);
                        break;
                    }
                    Err(err) => {
                        if first_error.is_none() {
                            first_error = Some(err);
                        }
                    }
                }
            }
        }

        if routes.is_empty() {
            return Err(first_error.unwrap_or_else(|| Error::NoProvider(*first_catalog_format)));
        }

        Ok(routes)
    }

    fn alias_matches_provider(&self, resolver_alias: &str, provider_alias: &str) -> bool {
        if resolver_alias == provider_alias {
            return true;
        }

        if let Some(provider_id) = default_alias_provider_id(resolver_alias) {
            return self
                .providers
                .get(provider_alias)
                .is_some_and(|provider| provider.matches_provider_alias(provider_id));
        }

        if self
            .providers
            .get(provider_alias)
            .is_some_and(|provider| provider.matches_provider_alias(resolver_alias))
        {
            return true;
        }

        default_alias_provider_id(provider_alias)
            .is_some_and(|provider_id| provider_id == resolver_alias)
    }

    #[cfg(test)]
    fn resolve_providers(
        &self,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Vec<ResolvedProviderForTest>> {
        self.resolve_provider_routes(model, output_format, &[])
            .map(|routes| {
                routes
                    .into_iter()
                    .map(|route| {
                        (
                            route.provider_alias,
                            route.provider,
                            route.auth,
                            route.spec,
                            route.format,
                        )
                    })
                    .collect()
            })
    }

    fn resolve_provider(
        &self,
        output_format: ProviderFormat,
        spec: Arc<ModelSpec>,
        catalog_format: ProviderFormat,
        alias: String,
    ) -> Result<ProviderRoute> {
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
        let format = if provider_formats.contains(&ProviderFormat::Responses)
            && spec.requires_responses_api()
        {
            ProviderFormat::Responses
        } else if provider.id() == "azure" && catalog_format == ProviderFormat::Anthropic {
            // Anthropic on Azure only supports the messages format and isn’t
            // interchangeable with other APIs.
            ProviderFormat::Anthropic
        } else if provider.id() == "anthropic" {
            // Native Anthropic has two endpoints: /v1/messages (Anthropic format) and the
            // OpenAI-compatible /v1/chat/completions (ChatCompletions format). Use
            // /chat/completions only when the catalog entry was explicitly registered as
            // OpenAI format; otherwise always use the native Messages API.
            if catalog_format == ProviderFormat::ChatCompletions {
                ProviderFormat::ChatCompletions
            } else {
                ProviderFormat::Anthropic
            }
        } else if provider.id() == "bedrock" {
            // Bedrock supports both native Converse/invoke endpoints and an
            // OpenAI-compatible Chat Completions endpoint. Use the OpenAI-compatible
            // endpoint only when the model entry explicitly declares OpenAI format;
            // otherwise preserve the catalog's Bedrock wire format.
            if catalog_format == ProviderFormat::ChatCompletions {
                ProviderFormat::ChatCompletions
            } else {
                catalog_format
            }
        } else if provider.id() == "google" {
            // Google supports both native GenerateContent and an OpenAI-compatible
            // Chat Completions endpoint. Match Anthropic/Bedrock behavior: use the
            // compatibility endpoint only when the catalog explicitly declares it.
            if catalog_format == ProviderFormat::ChatCompletions {
                ProviderFormat::ChatCompletions
            } else {
                catalog_format
            }
        } else if provider.id() == "vertex"
            && output_format == ProviderFormat::ChatCompletions
            && spec.format == ProviderFormat::ChatCompletions
            && !spec.available_providers.is_empty()
        {
            ProviderFormat::ChatCompletions
        } else if output_format != catalog_format && provider_formats.contains(&output_format) {
            output_format
        } else {
            catalog_format
        };
        let auth = self
            .auth_configs
            .get(&alias)
            .cloned()
            .ok_or_else(|| Error::NoAuth(alias.clone()))?;
        Ok(ProviderRoute {
            provider_alias: alias,
            provider,
            auth,
            spec,
            format,
        })
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
                    http.response.version = tracing::field::Empty,
                );
                async {
                    provider
                        .complete(payload.clone(), auth, &spec, format, client_headers)
                        .await
                }
                .instrument(span.clone())
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

fn replace_transformed_response_model(bytes: Bytes, model: &str) -> Result<Bytes> {
    if !bytes
        .windows(br#""model":"transformed""#.len())
        .any(|window| window == br#""model":"transformed""#)
    {
        return Ok(bytes);
    }

    #[derive(Deserialize)]
    struct ResponseModelView {
        model: Option<String>,
    }

    let model_view: ResponseModelView = lingua::serde_json::from_slice(&bytes)?;
    if model_view.model.as_deref() != Some("transformed") {
        return Ok(bytes);
    }

    let mut value: Value = lingua::serde_json::from_slice(&bytes)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("model".to_string(), Value::String(model.to_string()));
    }

    lingua::serde_json::to_vec(&value)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)
}

fn default_alias_provider_id(alias: &str) -> Option<&'static str> {
    match alias {
        "OPENAI_API_KEY" => Some("openai"),
        "ANTHROPIC_API_KEY" => Some("anthropic"),
        "GEMINI_API_KEY" => Some("google"),
        "MISTRAL_API_KEY" => Some("mistral"),
        "AWS_DEFAULT_CREDENTIALS" => Some("bedrock"),
        "GOOGLE_DEFAULT_CREDENTIALS" => Some("vertex"),
        "AZURE_DEFAULT_CREDENTIALS" => Some("azure"),
        "DATABRICKS_DEFAULT_CREDENTIALS" => Some("databricks"),
        _ => None,
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
                if entry.provider.id() == "google" && format == ProviderFormat::ChatCompletions {
                    continue;
                }
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
    use crate::streaming::{RawResponseStream, StreamChunk};
    use async_trait::async_trait;
    use futures::{stream, StreamExt};
    use reqwest::header::HeaderMap;
    use std::sync::Mutex;

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

    struct StaticProvider {
        response: Bytes,
        stream_chunks: Vec<Bytes>,
    }

    #[async_trait]
    impl Provider for StaticProvider {
        fn id(&self) -> &'static str {
            "static"
        }

        fn provider_formats(&self) -> Vec<ProviderFormat> {
            vec![ProviderFormat::ChatCompletions]
        }

        async fn complete(
            &self,
            _payload: Bytes,
            _auth: &AuthConfig,
            _spec: &ModelSpec,
            _format: ProviderFormat,
            _client_headers: &ClientHeaders,
        ) -> Result<Bytes> {
            Ok(self.response.clone())
        }

        async fn complete_stream(
            &self,
            _payload: Bytes,
            _auth: &AuthConfig,
            _spec: &ModelSpec,
            _format: ProviderFormat,
            _client_headers: &ClientHeaders,
        ) -> Result<RawResponseStream> {
            Ok(Box::pin(stream::iter(
                self.stream_chunks
                    .clone()
                    .into_iter()
                    .map(|chunk| Ok(StreamChunk::data(chunk))),
            )))
        }

        async fn health_check(&self, _auth: &AuthConfig) -> Result<()> {
            Ok(())
        }
    }

    struct FakeOpenAICompatibleProvider {
        alias: &'static str,
    }

    #[async_trait]
    impl Provider for FakeOpenAICompatibleProvider {
        fn id(&self) -> &'static str {
            "openai"
        }

        fn matches_provider_alias(&self, alias: &str) -> bool {
            self.alias == alias
        }

        fn provider_formats(&self) -> Vec<ProviderFormat> {
            vec![ProviderFormat::ChatCompletions]
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

    fn router_with_static_provider(provider: StaticProvider) -> Router {
        let model = "gpt-5-mini";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "openai",
                provider,
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build()
            .expect("router builds")
    }

    fn chat_request_body() -> Bytes {
        Bytes::from_static(
            br#"{"model":"gpt-5-mini","messages":[{"role":"user","content":"hello"}]}"#,
        )
    }

    fn chat_response_body() -> Bytes {
        Bytes::from_static(
            br#"{"id":"chatcmpl-test","object":"chat.completion","created":0,"model":"gpt-5-mini","choices":[{"index":0,"message":{"role":"assistant","content":"hello"},"finish_reason":"stop"}]}"#,
        )
    }

    fn chat_stream_chunk_body() -> Bytes {
        Bytes::from_static(
            br#"{"id":"chatcmpl-test","object":"chat.completion.chunk","created":0,"model":"gpt-5-mini","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}"#,
        )
    }

    #[test]
    fn replace_transformed_response_model_replaces_exact_placeholder() {
        let response = Bytes::from_static(
            br#"{"id":"msg_transformed","type":"message","model":"transformed","content":[]}"#,
        );

        let patched =
            replace_transformed_response_model(response, "global.anthropic.claude-opus-4-8")
                .expect("response model patches");
        let parsed: Value = serde_json::from_slice(&patched).expect("valid response json");

        assert_eq!(
            parsed.get("model").and_then(Value::as_str),
            Some("global.anthropic.claude-opus-4-8")
        );
    }

    #[test]
    fn replace_transformed_response_model_preserves_real_model() {
        let response = Bytes::from_static(
            br#"{"id":"msg_123","type":"message","model":"claude-sonnet-4-5","content":[]}"#,
        );

        let patched = replace_transformed_response_model(
            response.clone(),
            "global.anthropic.claude-opus-4-8",
        )
        .expect("response model patches");

        assert_eq!(patched, response);
    }

    #[test]
    fn replace_transformed_response_model_preserves_missing_model() {
        let response = Bytes::from_static(br#"{"id":"msg_123","type":"message","content":[]}"#);

        let patched = replace_transformed_response_model(
            response.clone(),
            "global.anthropic.claude-opus-4-8",
        )
        .expect("response model patches");

        assert_eq!(patched, response);
    }

    fn resolved_aliases(
        router: &Router,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<Vec<String>> {
        router
            .resolve_provider_routes(model, output_format, &[])
            .map(|routes| {
                routes
                    .into_iter()
                    .map(|route| route.provider_alias)
                    .collect()
            })
    }

    fn explicit_route_aliases(
        router: &Router,
        model: &str,
        output_format: ProviderFormat,
        aliases: &[&str],
    ) -> Result<Vec<String>> {
        let aliases = aliases
            .iter()
            .map(|alias| alias.to_string())
            .collect::<Vec<_>>();
        router
            .resolve_provider_routes(model, output_format, &aliases)
            .map(|routes| {
                routes
                    .into_iter()
                    .map(|route| route.provider_alias)
                    .collect()
            })
    }

    async fn create_test_request(
        router: &Router,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<(PreparedRequest, RouterMetadata)> {
        let routes = router.resolve_provider_routes(model, output_format, &[])?;
        let route = routes
            .first()
            .ok_or_else(|| Error::NoProvider(output_format))?;
        router
            .create_request(body, output_format, route, false)
            .await
    }

    async fn create_test_stream_request(
        router: &Router,
        body: Bytes,
        model: &str,
        output_format: ProviderFormat,
    ) -> Result<(PreparedStreamRequest, RouterMetadata)> {
        let routes = router.resolve_provider_routes(model, output_format, &[])?;
        let route = routes
            .first()
            .ok_or_else(|| Error::NoProvider(output_format))?;
        router
            .create_stream_request(body, output_format, route, false)
            .await
    }

    #[tokio::test]
    async fn prepare_provider_request_enables_stream_for_google_to_chat_completions() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-5-mini","contents":[{"role":"user","parts":[{"text":"hello"}]}]}"#,
        );
        let spec = openai_spec("gpt-5-mini", ModelFlavor::Chat);

        let (payload, _, _) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            true,
            RequestPreparationOptions::default(),
        )
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

        let (payload, _, _) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            false,
            RequestPreparationOptions::default(),
        )
        .await
        .expect("request prepares");

        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");
        assert_eq!(parsed.get("stream"), None);
        assert_eq!(parsed.get("stream_options"), None);
    }

    #[tokio::test]
    async fn prepare_provider_request_does_not_read_model_for_vertex_anthropic() {
        let body = Bytes::from_static(
            br#"{"model":"claude-sonnet-4-6","messages":[{"role":"user","content":"Ping"}]}"#,
        );
        let spec = ModelSpec {
            model: "publishers/anthropic/models/claude-sonnet-4-6".to_string(),
            format: ProviderFormat::Anthropic,
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
            available_providers: vec!["vertex".to_string()],
        };

        let (payload, _, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::VertexAnthropic,
            false,
            RequestPreparationOptions::default(),
        )
        .await
        .expect("request prepares");
        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");

        assert_eq!(actual_format, ProviderFormat::VertexAnthropic);
        assert_eq!(parsed.get("model"), None);
        assert!(parsed.get("anthropic_version").is_some());
        assert!(parsed.get("messages").is_some());
    }

    #[tokio::test]
    async fn prepare_provider_request_does_not_rewrite_model_for_google_pass_through() {
        let body = Bytes::from_static(
            br#"{"model":"models/gemini-2.5-flash","contents":[{"role":"user","parts":[{"text":"Ping"}]}]}"#,
        );
        let spec = ModelSpec {
            model: "models/gemini-2.5-pro".to_string(),
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
            available_providers: vec!["google".to_string()],
        };

        let (payload, _, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::Google,
            false,
            RequestPreparationOptions::default(),
        )
        .await
        .expect("request prepares");
        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");

        assert_eq!(actual_format, ProviderFormat::Google);
        assert_eq!(
            parsed.get("model").and_then(Value::as_str),
            Some("models/gemini-2.5-flash")
        );
        assert!(parsed.get("contents").is_some());
    }

    #[tokio::test]
    async fn prepare_provider_request_rewrites_same_format_chat_model_without_losing_native_fields()
    {
        let body = Bytes::from_static(
            br#"{"model":"gpt-4","messages":[{"role":"user","name":"example_user","content":"Ping"}]}"#,
        );
        let spec = openai_spec("gpt-4o", ModelFlavor::Chat);

        let (payload, _, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            false,
            RequestPreparationOptions::default(),
        )
        .await
        .expect("request prepares");
        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");

        assert_eq!(actual_format, ProviderFormat::ChatCompletions);
        assert_eq!(parsed.get("model").and_then(Value::as_str), Some("gpt-4o"));
        assert_eq!(
            parsed.pointer("/messages/0/name").and_then(Value::as_str),
            Some("example_user")
        );
    }

    #[tokio::test]
    async fn prepare_provider_request_can_preserve_same_format_body_model() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-4","messages":[{"role":"user","name":"example_user","content":"Ping"}]}"#,
        );
        let spec = openai_spec("gpt-4o", ModelFlavor::Chat);

        let (payload, _, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            false,
            RequestPreparationOptions {
                rewrite_body_model: false,
            },
        )
        .await
        .expect("request prepares");
        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");

        assert_eq!(actual_format, ProviderFormat::ChatCompletions);
        assert_eq!(parsed.get("model").and_then(Value::as_str), Some("gpt-4"));
        assert_eq!(
            parsed.pointer("/messages/0/name").and_then(Value::as_str),
            Some("example_user")
        );
    }

    #[tokio::test]
    async fn prepare_provider_request_can_preserve_body_model_across_format_transform() {
        let body = Bytes::from_static(
            br#"{"model":"claude-3-5-haiku-20241022","max_tokens":128,"messages":[{"role":"user","content":"Ping"}]}"#,
        );
        let spec = openai_spec("gpt-4o", ModelFlavor::Chat);

        let (payload, detected_format, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            false,
            RequestPreparationOptions {
                rewrite_body_model: false,
            },
        )
        .await
        .expect("request prepares");
        let parsed: Value = serde_json::from_slice(&payload).expect("valid request json");

        assert_eq!(detected_format, Some(ProviderFormat::Anthropic));
        assert_eq!(actual_format, ProviderFormat::ChatCompletions);
        assert_eq!(
            parsed.get("model").and_then(Value::as_str),
            Some("claude-3-5-haiku-20241022")
        );
    }

    #[tokio::test]
    async fn prepare_provider_request_upgrades_actual_format_to_responses_for_reasoning_plus_tools()
    {
        // A chat-completions request with reasoning_effort + tools should have its actual_format
        // upgraded to Responses so the router sends it to the correct endpoint.
        let body = Bytes::from(
            serde_json::json!({
                "model": "gpt-5.4-mini",
                "messages": [{"role": "user", "content": "Tokyo weather?"}],
                "reasoning_effort": "medium",
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get weather",
                        "parameters": {
                            "type": "object",
                            "properties": {"location": {"type": "string"}},
                            "required": ["location"]
                        }
                    }
                }]
            })
            .to_string(),
        );
        let spec = openai_spec("gpt-5.4-mini", ModelFlavor::Chat);

        let (_, _, actual_format) = prepare_provider_request(
            body,
            &spec,
            ProviderFormat::ChatCompletions,
            false,
            RequestPreparationOptions::default(),
        )
        .await
        .expect("request prepares");

        assert_eq!(
            actual_format,
            ProviderFormat::Responses,
            "actual_format must be Responses so the router uses the /v1/responses endpoint"
        );
    }

    fn dummy_auth() -> AuthConfig {
        AuthConfig::ApiKey {
            key: "test".into(),
            header: Some("authorization".into()),
            prefix: Some("Bearer".into()),
        }
    }

    #[tokio::test]
    async fn complete_with_raw_response_returns_response_and_raw_response() {
        let raw_response = chat_response_body();
        let router = router_with_static_provider(StaticProvider {
            response: raw_response.clone(),
            stream_chunks: Vec::new(),
        });
        let (prepared, _) = create_test_request(
            &router,
            chat_request_body(),
            "gpt-5-mini",
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("request prepares");

        let result = router
            .complete_with_raw_response(prepared, &ClientHeaders::default())
            .await
            .expect("complete succeeds");

        assert_eq!(result.response, raw_response);
        assert_eq!(result.raw_response, raw_response);
    }

    #[tokio::test]
    async fn complete_with_raw_response_preserves_raw_response_on_transform_error() {
        let raw_response = Bytes::from_static(b"not-json");
        let router = router_with_static_provider(StaticProvider {
            response: raw_response.clone(),
            stream_chunks: Vec::new(),
        });
        let (prepared, _) = create_test_request(
            &router,
            chat_request_body(),
            "gpt-5-mini",
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("request prepares");

        let err = router
            .complete_with_raw_response(prepared, &ClientHeaders::default())
            .await
            .expect_err("transform fails");

        assert!(matches!(err, Error::ResponseTransform { .. }));
        assert_eq!(err.raw_response(), Some(&raw_response));
    }

    #[tokio::test]
    async fn complete_stream_raw_chunk_capture_runs_before_transform_error() {
        let raw_chunk = Bytes::from_static(b"not-json");
        let router = router_with_static_provider(StaticProvider {
            response: Bytes::new(),
            stream_chunks: vec![raw_chunk.clone()],
        });
        let (prepared, _) = create_test_stream_request(
            &router,
            chat_request_body(),
            "gpt-5-mini",
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("stream request prepares");
        let captured = Arc::new(Mutex::new(Vec::new()));
        let capture: RawStreamChunkCapture = Arc::new({
            let captured = Arc::clone(&captured);
            move |chunk: &StreamChunk| {
                captured
                    .lock()
                    .expect("capture lock poisoned")
                    .push(chunk.data.clone());
            }
        });

        let mut stream = router
            .complete_stream_with_raw_response_capture(
                prepared,
                &ClientHeaders::default(),
                Some("request-id".to_string()),
                capture,
            )
            .await
            .expect("stream starts");
        let first = stream.next().await.expect("stream item");

        assert!(first.is_err());
        assert_eq!(
            captured.lock().expect("capture lock poisoned").as_slice(),
            &[raw_chunk]
        );
    }

    #[tokio::test]
    async fn complete_methods_work_without_raw_response_capture() {
        let raw_response = chat_response_body();
        let router = router_with_static_provider(StaticProvider {
            response: raw_response.clone(),
            stream_chunks: vec![chat_stream_chunk_body()],
        });
        let (prepared, _) = create_test_request(
            &router,
            chat_request_body(),
            "gpt-5-mini",
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("request prepares");
        let response = router
            .complete(prepared, &ClientHeaders::default())
            .await
            .expect("complete succeeds");
        assert_eq!(response, raw_response);

        let (prepared_stream, _) = create_test_stream_request(
            &router,
            chat_request_body(),
            "gpt-5-mini",
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("stream request prepares");
        let mut response_stream = router
            .complete_stream(
                prepared_stream,
                &ClientHeaders::default(),
                Some("request-id".to_string()),
            )
            .await
            .expect("stream starts");
        let first = response_stream
            .next()
            .await
            .expect("stream item")
            .expect("stream item succeeds");
        assert!(!first.data.is_empty());
    }

    fn google_chat_router(model: &str) -> Router {
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), google_spec(model));
        Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds")
    }

    #[tokio::test]
    async fn gemini_native_catalog_transforms_chat_completions_request_to_google() {
        let model = "gemini-2.5-flash";
        let router = google_chat_router(model);
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(
            metadata.detected_input_format,
            ProviderFormat::ChatCompletions
        );
        assert!(!metadata.lingua_passthrough);
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);

        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn bare_gemini_native_catalog_transforms_prefixed_chat_completions_payload() {
        let model = "gemini-2.5-flash";
        let router = google_chat_router(model);
        let body = Bytes::from_static(
            br#"{"model":"models/gemini-2.5-flash","messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(
            metadata.detected_input_format,
            ProviderFormat::ChatCompletions
        );
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn prefixed_gemini_native_catalog_uses_native_google_format() {
        let model = "models/gemini-2.5-flash";
        let router = google_chat_router(model);
        let body = Bytes::from_static(
            br#"{"model":"models/gemini-2.5-flash","messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(
            metadata.detected_input_format,
            ProviderFormat::ChatCompletions
        );
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn prefixed_gemini_key_with_bare_spec_model_uses_native_google_format() {
        let model = "models/gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), google_spec("gemini-2.5-flash"));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");
        let body = Bytes::from_static(
            br#"{"model":"models/gemini-2.5-flash","messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn prefixed_gemini_chat_completions_catalog_uses_native_google_format() {
        let model = "models/gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");
        let body = Bytes::from_static(
            br#"{"model":"models/gemini-2.5-flash","seed":42,"logprobs":true,"messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
        assert!(payload.get("seed").is_none());
        assert!(payload.get("logprobs").is_none());
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn gemini_chat_completions_catalog_uses_native_google_format() {
        let model = "gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","seed":42,"logprobs":true,"messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
        assert!(payload.get("seed").is_none());
        assert!(payload.get("logprobs").is_none());
        assert!(payload.get("contents").is_some());
        assert!(payload.get("messages").is_none());
    }

    #[tokio::test]
    async fn explicit_gemini_chat_completions_catalog_uses_chat_completions_format() {
        let model = "gemini-2.5-flash";
        let mut spec = openai_spec(model, ModelFlavor::Chat);
        spec.available_providers = vec!["google".into()];
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), spec);
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","logprobs":true,"messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::ChatCompletions);
        assert_eq!(request.inner.format, ProviderFormat::ChatCompletions);
        assert_eq!(request.inner.payload, body);
        assert_eq!(payload.get("logprobs"), Some(&Value::Bool(true)));
        assert!(payload.get("messages").is_some());
        assert!(payload.get("contents").is_none());
    }

    #[tokio::test]
    async fn gemini_chat_completions_with_reasoning_and_tools_does_not_upgrade_to_responses() {
        let model = "gemini-2.5-flash";
        let router = google_chat_router(model);
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","reasoning_effort":"medium","messages":[{"role":"user","content":"Ping"}],"tools":[{"type":"function","function":{"name":"lookup","parameters":{"type":"object","properties":{},"required":[]}}}]}"#,
        );

        let (request, metadata) = create_test_request(
            &router,
            body.clone(),
            model,
            ProviderFormat::ChatCompletions,
        )
        .await
        .expect("create request");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(request.inner.format, ProviderFormat::Google);
        assert_ne!(request.inner.payload, body);
    }

    #[tokio::test]
    async fn gemini_stream_chat_completions_request_uses_native_google_streaming() {
        let model = "gemini-2.5-flash";
        let router = google_chat_router(model);
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-flash","messages":[{"role":"user","content":"Ping"}]}"#,
        );

        let (request, metadata) =
            create_test_stream_request(&router, body, model, ProviderFormat::ChatCompletions)
                .await
                .expect("create stream request");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(metadata.provider_alias, "google");
        assert_eq!(metadata.provider_format, ProviderFormat::Google);
        assert_eq!(payload.get("stream"), None);
        assert!(payload.get("contents").is_some());
    }

    #[test]
    fn native_google_request_still_uses_google_format() {
        let model = "gemini-2.5-flash";
        let router = google_chat_router(model);

        let routes = router
            .resolve_providers(model, ProviderFormat::Google)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (alias, provider, _, _, format) = &routes[0];
        assert_eq!(alias, "google");
        assert_eq!(provider.id(), "google");
        assert_eq!(*format, ProviderFormat::Google);
    }

    #[test]
    fn vertex_google_model_does_not_use_google_chat_completions() {
        let model = "publishers/google/models/gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), google_spec(model));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
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
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (alias, provider, _, _, format) = &routes[0];
        assert_eq!(alias, "vertex");
        assert_eq!(provider.id(), "vertex");
        assert_eq!(*format, ProviderFormat::Google);
    }

    #[test]
    fn openai_model_does_not_route_to_google_when_google_supports_chat_completions() {
        let model = "gpt-5-mini";
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
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (alias, provider, _, _, format) = &routes[0];
        assert_eq!(alias, "openai");
        assert_eq!(provider.id(), "openai");
        assert_eq!(*format, ProviderFormat::ChatCompletions);
    }

    #[test]
    fn openai_model_without_openai_provider_does_not_fallback_to_google_chat_completions() {
        let model = "gpt-5-mini";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");

        let err = match router.resolve_providers(model, ProviderFormat::ChatCompletions) {
            Ok(_) => panic!("should not route OpenAI model to Google fallback"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            Error::NoProvider(ProviderFormat::ChatCompletions)
        ));
    }

    #[test]
    fn gemini_chat_completions_catalog_without_google_provider_does_not_fallback_to_openai() {
        let model = "gemini-2.5-flash";
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

        let err = match router.resolve_providers(model, ProviderFormat::ChatCompletions) {
            Ok(_) => panic!("should not route Gemini model to OpenAI fallback"),
            Err(err) => err,
        };

        assert!(matches!(err, Error::NoProvider(ProviderFormat::Google)));
    }

    #[test]
    fn gemini_model_falls_back_to_default_google_provider_alias() {
        let model = "gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "google-prod",
                FakeProvider {
                    name: "google",
                    formats: vec![ProviderFormat::Google, ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::Google],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves via default Google provider");

        assert_eq!(routes.len(), 1);
        let (alias, provider, _, _, format) = &routes[0];
        assert_eq!(alias, "google-prod");
        assert_eq!(provider.id(), "google");
        assert_eq!(*format, ProviderFormat::Google);
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
    fn vertex_google_model_with_openai_format_keeps_google_transport() {
        let model = "publishers/google/models/gemini-2.5-flash";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));

        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "vertex",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::Google],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format) = routes[0];
        assert_eq!(format, ProviderFormat::Google);
    }

    #[test]
    fn custom_vertex_openai_model_uses_chat_completions_transport() {
        let model = "publishers/google/models/gemini-3.1-flash-lite-preview";
        let mut spec = openai_spec(model, ModelFlavor::Chat);
        spec.available_providers = vec!["custom-vertex".into()];

        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), spec);

        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "custom-vertex",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::Google],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (alias, _, _, _, format) = &routes[0];
        assert_eq!(alias, "custom-vertex");
        assert_eq!(*format, ProviderFormat::ChatCompletions);
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].format, ProviderFormat::Responses);
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].format, ProviderFormat::Responses);
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].format, ProviderFormat::ChatCompletions);
    }

    #[test]
    fn bedrock_openai_catalog_format_uses_chat_completions_transport() {
        let bedrock_spec =
            |model: &str, format: ProviderFormat, providers: Vec<String>| ModelSpec {
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
                available_providers: providers,
            };
        let model = "us.anthropic.claude-sonnet-4-6";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(
            model.into(),
            bedrock_spec(
                model,
                ProviderFormat::ChatCompletions,
                vec!["bedrock".into()],
            ),
        );
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "bedrock",
                FakeProvider {
                    name: "bedrock",
                    formats: vec![ProviderFormat::Converse, ProviderFormat::BedrockAnthropic],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format) = routes[0];
        assert_eq!(format, ProviderFormat::ChatCompletions);
    }

    #[test]
    fn bedrock_converse_catalog_format_keeps_converse_transport_for_chat_output() {
        let bedrock_spec = |model: &str, format: ProviderFormat| ModelSpec {
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
        };
        let model = "amazon.nova-lite-v1:0";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), bedrock_spec(model, ProviderFormat::Converse));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "bedrock",
                FakeProvider {
                    name: "bedrock",
                    formats: vec![ProviderFormat::Converse, ProviderFormat::BedrockAnthropic],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_providers(model, ProviderFormat::ChatCompletions)
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        let (_, _, _, _, format) = routes[0];
        assert_eq!(format, ProviderFormat::Converse);
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].format, ProviderFormat::ChatCompletions);
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
            .expect("resolves");
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].provider_alias(), "azure");
        assert_eq!(routes[0].provider.id(), "azure");
        assert_eq!(routes[0].format, ProviderFormat::Responses);
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
            .resolve_provider_routes("base-model", ProviderFormat::ChatCompletions, &[])
            .is_ok());
        assert!(router
            .resolve_provider_routes("custom-model", ProviderFormat::ChatCompletions, &[])
            .is_ok());
    }

    #[test]
    fn overlay_catalog_failover_routes_use_equivalent_custom_model() {
        let base = ModelCatalog::empty();
        let mut custom = ModelCatalog::empty();
        let mut primary = openai_spec("custom-primary", ModelFlavor::Chat);
        primary.available_providers = vec!["provider-a".to_string()];
        let mut fallback = openai_spec("custom-fallback", ModelFlavor::Chat);
        fallback.available_providers = vec!["provider-b".to_string()];
        custom.insert("custom-primary".into(), primary);
        custom.insert("custom-fallback".into(), fallback);
        custom
            .add_fallback_models(
                "custom-primary".to_string(),
                vec!["custom-fallback".to_string()],
            )
            .expect("equivalence is valid");

        let router = Router::builder()
            .with_overlay_catalog(Arc::new(base), custom)
            .add_provider(
                "provider-a",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "provider-b",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                "custom-primary",
                ProviderFormat::ChatCompletions,
                &["provider-a".to_string(), "provider-b".to_string()],
            )
            .expect("failover routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![
                ("provider-a", "custom-primary"),
                ("provider-b", "custom-fallback"),
            ]
        );
    }

    #[test]
    fn overlay_catalog_failover_routes_use_equivalent_base_model() {
        let mut base = ModelCatalog::empty();
        base.insert(
            "base-fallback".into(),
            openai_spec_with_available_providers("base-fallback", ModelFlavor::Chat),
        );
        let mut custom = ModelCatalog::empty();
        let mut primary = openai_spec("custom-primary", ModelFlavor::Chat);
        primary.available_providers = vec!["provider-a".to_string()];
        custom.insert("custom-primary".into(), primary);
        custom
            .add_external_fallback_models(
                "custom-primary".to_string(),
                vec!["base-fallback".to_string()],
            )
            .expect("equivalence is valid");

        let router = Router::builder()
            .with_overlay_catalog(Arc::new(base), custom)
            .add_provider(
                "provider-a",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                "custom-primary",
                ProviderFormat::ChatCompletions,
                &["provider-a".to_string(), "openai".to_string()],
            )
            .expect("failover routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![
                ("provider-a", "custom-primary"),
                ("openai", "base-fallback"),
            ]
        );
        assert!(router.catalog().get("base-fallback").is_some());
        assert!(router.catalog().get("custom-primary").is_none());
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
    fn fallback_provider_routes_append_after_primary() {
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

        assert_eq!(
            explicit_route_aliases(
                &router,
                model,
                ProviderFormat::ChatCompletions,
                &["azure", "openai"]
            )
            .expect("routes"),
            vec!["openai".to_string(), "azure".to_string()]
        );
    }

    #[test]
    fn fallback_provider_routes_filter_unavailable_and_unregistered_aliases() {
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
            .build()
            .expect("router builds");

        assert_eq!(
            explicit_route_aliases(
                &router,
                model,
                ProviderFormat::ChatCompletions,
                &["custom", "openai", "azure"]
            )
            .expect("routes"),
            vec!["openai".to_string()]
        );
    }

    #[test]
    fn fallback_provider_routes_do_not_treat_openai_provider_id_as_allowlist_match() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        let mut spec = openai_spec(model, ModelFlavor::Chat);
        spec.available_providers = vec!["openai".into()];
        catalog.insert(model.into(), spec);
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
                "cerebras",
                FakeOpenAICompatibleProvider { alias: "cerebras" },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        assert_eq!(
            explicit_route_aliases(
                &router,
                model,
                ProviderFormat::ChatCompletions,
                &["cerebras"]
            )
            .expect("routes"),
            vec!["openai".to_string()]
        );
    }

    #[test]
    fn fallback_provider_routes_match_named_openai_secret_for_default_alias() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        let mut spec = openai_spec(model, ModelFlavor::Chat);
        spec.available_providers = vec!["OPENAI_API_KEY".into()];
        catalog.insert(model.into(), spec);
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "my-openai",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        assert_eq!(
            explicit_route_aliases(
                &router,
                model,
                ProviderFormat::ChatCompletions,
                &["my-openai"]
            )
            .expect("routes"),
            vec!["my-openai".to_string()]
        );
    }

    #[test]
    fn failover_routes_match_named_openai_when_equivalents_omit_available_providers() {
        let model = "custom-primary";
        let fallback_model = "gpt-4o";
        let mut base = ModelCatalog::empty();
        base.insert(
            fallback_model.into(),
            openai_spec(fallback_model, ModelFlavor::Chat),
        );
        let mut custom = ModelCatalog::empty();
        let mut primary = openai_spec(model, ModelFlavor::Chat);
        primary.available_providers = vec!["provider-a".to_string()];
        custom.insert(model.into(), primary);
        custom
            .add_external_fallback_models(model.to_string(), vec![fallback_model.to_string()])
            .expect("equivalence is valid");

        let router = Router::builder()
            .with_overlay_catalog(Arc::new(base), custom)
            .add_provider(
                "provider-a",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "my-openai",
                FakeOpenAICompatibleProvider { alias: "openai" },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &["provider-a".to_string(), "my-openai".to_string()],
            )
            .expect("routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![("provider-a", model), ("my-openai", fallback_model)]
        );
    }

    #[test]
    fn failover_routes_match_named_openai_compatible_provider_id() {
        let model = "custom-primary";
        let fallback_model = "llama-4-scout";
        let mut base = ModelCatalog::empty();
        let mut fallback = openai_spec(fallback_model, ModelFlavor::Chat);
        fallback.available_providers = vec!["cerebras".to_string()];
        base.insert(fallback_model.into(), fallback);
        let mut custom = ModelCatalog::empty();
        let mut primary = openai_spec(model, ModelFlavor::Chat);
        primary.available_providers = vec!["provider-a".to_string()];
        custom.insert(model.into(), primary);
        custom
            .add_external_fallback_models(model.to_string(), vec![fallback_model.to_string()])
            .expect("equivalence is valid");

        let router = Router::builder()
            .with_overlay_catalog(Arc::new(base), custom)
            .add_provider(
                "provider-a",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "my-cerebras",
                FakeOpenAICompatibleProvider { alias: "cerebras" },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &["provider-a".to_string(), "my-cerebras".to_string()],
            )
            .expect("routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![("provider-a", model), ("my-cerebras", fallback_model)]
        );
    }

    #[tokio::test]
    async fn failover_request_payload_uses_equivalent_route_model_for_same_format() {
        let model = "gpt-4o";
        let fallback_model = "other-provider/gpt-4o";
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "gpt-4o": {
    "format": "openai",
    "flavor": "chat",
    "available_providers": ["provider-a"],
    "fallback_models": ["other-provider/gpt-4o"]
  },
  "other-provider/gpt-4o": {
    "format": "openai",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "provider-a",
                FakeProvider {
                    name: "openai",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "my-openai",
                FakeOpenAICompatibleProvider { alias: "openai" },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &["provider-a".to_string(), "my-openai".to_string()],
            )
            .expect("routes resolve");
        let fallback_route = routes
            .iter()
            .find(|route| route.provider_alias() == "my-openai")
            .expect("fallback route exists");
        assert_eq!(fallback_route.model(), fallback_model);

        let body = Bytes::from_static(
            br#"{"model":"gpt-4o","messages":[{"role":"user","content":"Ping"}]}"#,
        );
        let (request, _) = router
            .create_request(body, ProviderFormat::ChatCompletions, fallback_route, false)
            .await
            .expect("request prepares");
        let payload: Value = serde_json::from_slice(&request.inner.payload).expect("json");

        assert_eq!(
            payload.get("model").and_then(Value::as_str),
            Some(fallback_model)
        );
    }

    #[test]
    fn fallback_provider_routes_do_not_match_openai_default_alias_to_compatible_provider() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        let mut spec = openai_spec(model, ModelFlavor::Chat);
        spec.available_providers = vec!["OPENAI_API_KEY".into()];
        catalog.insert(model.into(), spec);
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
                "cerebras",
                FakeOpenAICompatibleProvider { alias: "cerebras" },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let err = match explicit_route_aliases(
            &router,
            model,
            ProviderFormat::ChatCompletions,
            &["cerebras"],
        ) {
            Ok(_) => panic!("OpenAI-compatible provider should not satisfy OpenAI default alias"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            Error::NoProvider(ProviderFormat::ChatCompletions)
        ));
    }

    #[test]
    fn failover_routes_match_named_secrets_by_concrete_provider_id() {
        let model = "claude-sonnet-4-6";
        let vertex_model = "publishers/anthropic/models/claude-sonnet-4-6";
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "available_providers": ["ANTHROPIC_API_KEY"],
    "fallback_models": ["publishers/anthropic/models/claude-sonnet-4-6"]
  },
  "publishers/anthropic/models/claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "available_providers": ["GOOGLE_DEFAULT_CREDENTIALS"]
  }
}"#,
        )
        .expect("catalog parses");
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "my-anthropic",
                FakeProvider {
                    name: "anthropic",
                    formats: vec![ProviderFormat::Anthropic],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "my-vertex",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::VertexAnthropic],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &["my-anthropic".to_string(), "my-vertex".to_string()],
            )
            .expect("routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![("my-anthropic", model), ("my-vertex", vertex_model),]
        );
    }

    #[test]
    fn failover_routes_match_named_secrets_when_equivalents_omit_available_providers() {
        let model = "claude-sonnet-4-6";
        let vertex_model = "publishers/anthropic/models/claude-sonnet-4-6";
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "fallback_models": ["publishers/anthropic/models/claude-sonnet-4-6"]
  },
  "publishers/anthropic/models/claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "my-anthropic",
                FakeProvider {
                    name: "anthropic",
                    formats: vec![ProviderFormat::Anthropic],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "my-vertex",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::VertexAnthropic],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &["my-anthropic".to_string(), "my-vertex".to_string()],
            )
            .expect("routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![("my-anthropic", model), ("my-vertex", vertex_model),]
        );
    }

    #[test]
    fn fallback_provider_routes_do_not_use_format_default_for_ineligible_alias() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(model.into(), openai_spec(model, ModelFlavor::Chat));
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "custom",
                FakeProvider {
                    name: "custom",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![ProviderFormat::ChatCompletions],
            )
            .build()
            .expect("router builds");

        let err = match explicit_route_aliases(
            &router,
            model,
            ProviderFormat::ChatCompletions,
            &["custom"],
        ) {
            Ok(_) => panic!("ineligible fallback alias should not use format-default provider"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            Error::NoProvider(ProviderFormat::ChatCompletions)
        ));
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
            .resolve_provider_routes(model, ProviderFormat::ChatCompletions, &[])
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

    #[test]
    fn fallback_alias_resolution_skips_missing_without_format_fallback() {
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

        let err = match router.resolve_provider_routes(
            model,
            ProviderFormat::ChatCompletions,
            &["missing".to_string()],
        ) {
            Ok(_) => panic!("missing fallback aliases should not use format-default provider"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            Error::NoProvider(ProviderFormat::ChatCompletions)
        ));
    }

    #[test]
    fn fallback_alias_resolution_preserves_requested_order_and_skips_missing() {
        let model = "gpt-4o";
        let mut catalog = ModelCatalog::empty();
        catalog.insert(
            model.into(),
            openai_spec_with_available_providers(model, ModelFlavor::Chat),
        );
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "azure",
                FakeProvider {
                    name: "azure",
                    formats: vec![ProviderFormat::ChatCompletions],
                },
                dummy_auth(),
                vec![],
            )
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
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &[
                    "azure".to_string(),
                    "missing".to_string(),
                    "openai".to_string(),
                ],
            )
            .expect("alias route resolution succeeds");
        let aliases: Vec<&str> = routes.iter().map(|route| route.provider_alias()).collect();

        assert_eq!(aliases, vec!["openai", "azure"]);
    }

    #[test]
    fn failover_routes_use_equivalent_provider_native_vertex_model() {
        let model = "claude-sonnet-4-6";
        let vertex_model = "publishers/anthropic/models/claude-sonnet-4-6";
        let catalog = ModelCatalog::from_json_str(
            r#"{
  "claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat",
    "available_providers": ["anthropic"],
    "fallback_models": ["publishers/anthropic/models/claude-sonnet-4-6"]
  },
  "publishers/anthropic/models/claude-sonnet-4-6": {
    "format": "anthropic",
    "flavor": "chat"
  }
}"#,
        )
        .expect("catalog parses");
        let catalog = catalog.map_specs(|_, spec| {
            let mut spec = spec.clone();
            spec.available_providers = spec
                .available_providers
                .iter()
                .map(|provider| {
                    if provider == "anthropic" {
                        "ANTHROPIC_API_KEY".to_string()
                    } else {
                        provider.clone()
                    }
                })
                .collect();
            spec
        });
        let router = Router::builder()
            .with_catalog(Arc::new(catalog))
            .add_provider(
                "ANTHROPIC_API_KEY",
                FakeProvider {
                    name: "anthropic",
                    formats: vec![ProviderFormat::Anthropic],
                },
                dummy_auth(),
                vec![],
            )
            .add_provider(
                "GOOGLE_DEFAULT_CREDENTIALS",
                FakeProvider {
                    name: "vertex",
                    formats: vec![ProviderFormat::VertexAnthropic],
                },
                dummy_auth(),
                vec![],
            )
            .build()
            .expect("router builds");

        let routes = router
            .resolve_provider_routes(
                model,
                ProviderFormat::ChatCompletions,
                &[
                    "ANTHROPIC_API_KEY".to_string(),
                    "GOOGLE_DEFAULT_CREDENTIALS".to_string(),
                ],
            )
            .expect("failover routes resolve");
        let route_info: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.provider_alias(), route.model()))
            .collect();

        assert_eq!(
            route_info,
            vec![
                ("ANTHROPIC_API_KEY", model),
                ("GOOGLE_DEFAULT_CREDENTIALS", vertex_model),
            ]
        );
    }
}
