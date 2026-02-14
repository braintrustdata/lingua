use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

const ANTHROPIC_VERSION: &str = "anthropic-version";
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
    client: Client,
    config: AnthropicConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = if config.timeout.is_some() {
            crate::client::build_client(&settings)?
        } else {
            default_client().or_else(|_| crate::client::build_client(&settings))?
        };
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

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "anthropic",
            http_url = %url,
            "sending request to Anthropic"
        );

        let mut headers = self.build_headers(client_headers);
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(payload)
            .send()
            .await?;

        #[cfg(feature = "tracing")]
        let status_code = response.status().as_u16();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "anthropic",
            http_status_code = status_code,
            "received response from Anthropic"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status, &text),
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

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "anthropic",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Anthropic"
        );

        let mut headers = self.build_headers(client_headers);
        auth.apply_headers(&mut headers)?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(payload)
            .send()
            .await?;

        #[cfg(feature = "tracing")]
        let status_code = response.status().as_u16();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "anthropic",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Anthropic"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status, &text),
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

fn extract_retry_after(status: StatusCode, _body: &str) -> Option<Duration> {
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Some(Duration::from_secs(2))
    } else {
        None
    }
}
