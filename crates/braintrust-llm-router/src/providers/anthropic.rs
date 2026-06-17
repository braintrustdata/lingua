use std::time::Duration;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::{send_with_response_header_capture, ClientHeaders};
use crate::streaming::{sse_stream, RawResponseStream};
use async_trait::async_trait;
use bytes::Bytes;
use lingua::ProviderFormat;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

pub const ANTHROPIC_VERSION: &str = "anthropic-version";
pub const DEFAULT_ANTHROPIC_VERSION_VALUE: &str = "2023-06-01";
const ANTHROPIC_BETA: &str = "anthropic-beta";
const STRUCTURED_OUTPUTS_BETA: &str = "structured-outputs-2025-11-13";

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub endpoint: Url,
    pub version: String,
    pub timeout: Option<Duration>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.anthropic.com/v1/")
                .expect("valid Anthropic endpoint"),
            version: "2023-06-01".to_string(),
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: ClientWithMiddleware,
    config: AnthropicConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: AnthropicConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    /// Create an Anthropic provider from configuration parameters.
    ///
    /// Extracts Anthropic-specific options from metadata:
    /// - `version`: Anthropic API version (defaults to "2023-06-01")
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, lingua::serde_json::Value>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        use lingua::serde_json::Value;
        let mut config = AnthropicConfig::default();

        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }
        if let Some(version) = metadata.get("version").and_then(Value::as_str) {
            config.version = version.to_string();
        }

        Self::new_with_client_settings(config, client_settings.unwrap_or_default())
    }

    fn messages_url(&self) -> Url {
        self.config
            .endpoint
            .join("messages")
            .expect("join messages path")
    }

    fn chat_completions_url(&self) -> Url {
        self.config
            .endpoint
            .join("chat/completions")
            .expect("join chat/completions path")
    }

    fn build_headers(&self, client_headers: &ClientHeaders) -> HeaderMap {
        let mut headers = client_headers.to_json_headers();

        headers.insert(
            ANTHROPIC_VERSION,
            HeaderValue::from_str(&self.config.version).expect("version header"),
        );

        // Respect caller override: only set default if missing.
        if !headers.contains_key(ANTHROPIC_BETA) {
            headers.insert(
                ANTHROPIC_BETA,
                HeaderValue::from_static(STRUCTURED_OUTPUTS_BETA),
            );
        }

        headers
    }
}

#[async_trait]
impl crate::providers::Provider for AnthropicProvider {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Anthropic, ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let (url, headers) = if format == ProviderFormat::ChatCompletions {
            let mut h = client_headers.to_json_headers();
            let key = auth.api_key().ok_or_else(|| {
                Error::Auth("Anthropic /chat/completions requires an API key".to_string())
            })?;
            h.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|e| Error::Auth(format!("invalid auth header: {e}")))?,
            );
            (self.chat_completions_url(), h)
        } else {
            let mut h = self.build_headers(client_headers);
            auth.apply_headers(&mut h)?;
            (self.messages_url(), h)
        };

        let response = send_with_response_header_capture(
            self.client.post(url.clone()).headers(headers).body(payload),
            client_headers,
        )
        .await?;

        #[cfg(feature = "tracing")]
        {
            let span = tracing::Span::current();
            span.record("http.url", tracing::field::display(&url));
            span.record("http.status_code", response.status().as_u16());
        }

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: None,
                http: Some(UpstreamHttpError::new(
                    status.as_u16(),
                    headers,
                    text.clone(),
                )),
            });
        }

        Ok(response.bytes().await?)
    }

    async fn complete_stream(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<RawResponseStream> {
        if !spec.supports_streaming {
            return self
                .complete_stream_via_complete(payload, auth, spec, format, client_headers)
                .await;
        }

        // Router should have already added stream options to payload
        let (url, headers) = if format == ProviderFormat::ChatCompletions {
            let mut h = client_headers.to_json_headers();
            let key = auth.api_key().ok_or_else(|| {
                Error::Auth("Anthropic /chat/completions requires an API key".to_string())
            })?;
            h.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|e| Error::Auth(format!("invalid auth header: {e}")))?,
            );
            (self.chat_completions_url(), h)
        } else {
            let mut h = self.build_headers(client_headers);
            auth.apply_headers(&mut h)?;
            (self.messages_url(), h)
        };

        let response = send_with_response_header_capture(
            self.client.post(url.clone()).headers(headers).body(payload),
            client_headers,
        )
        .await?;

        #[cfg(feature = "tracing")]
        {
            let span = tracing::Span::current();
            span.record("http.url", tracing::field::display(&url));
            span.record("http.status_code", response.status().as_u16());
        }

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: None,
                http: Some(UpstreamHttpError::new(
                    status.as_u16(),
                    headers,
                    text.clone(),
                )),
            });
        }
        Ok(sse_stream(response))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self
            .config
            .endpoint
            .join("models")
            .expect("join models path");
        let mut headers = self.build_headers(&ClientHeaders::default());
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("status {}", response.status()),
                retry_after: None,
                http: None,
            })
        }
    }
}
