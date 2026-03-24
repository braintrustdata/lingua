use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

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
        let mut settings = ClientSettings::default();
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

        Self::new(config)
    }

    fn messages_url(&self) -> Url {
        self.config
            .endpoint
            .join("messages")
            .expect("join messages path")
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
        vec![ProviderFormat::Anthropic]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = self.messages_url();

        let mut headers = self.build_headers(client_headers);
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url.clone())
            .headers(headers)
            .body(payload)
            .send()
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
            let response = self
                .complete(payload, auth, spec, format, client_headers)
                .await?;
            return Ok(single_bytes_stream(response));
        }

        // Router should have already added stream options to payload
        let url = self.messages_url();

        let mut headers = self.build_headers(client_headers);
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url.clone())
            .headers(headers)
            .body(payload)
            .send()
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
