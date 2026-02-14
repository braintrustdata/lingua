use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub endpoint: Url,
    pub timeout: Option<Duration>,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://generativelanguage.googleapis.com/v1beta/")
                .expect("valid Google endpoint"),
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoogleProvider {
    client: Client,
    config: GoogleConfig,
}

impl GoogleProvider {
    pub fn new(config: GoogleConfig) -> Result<Self> {
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

    /// Create a Google provider from configuration parameters.
    pub fn from_config(endpoint: Option<&Url>, timeout: Option<Duration>) -> Result<Self> {
        let mut config = GoogleConfig::default();

        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }

        Self::new(config)
    }

    fn generate_url(&self, model: &str, stream: bool) -> Result<Url> {
        let path = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{model}")
        };
        let suffix = if stream {
            // Use alt=sse to get SSE format for streaming responses
            ":streamGenerateContent?alt=sse"
        } else {
            ":generateContent"
        };
        let url = self
            .config
            .endpoint
            .join(&(path + suffix))
            .map_err(|e| Error::InvalidRequest(format!("invalid Google model path: {e}")))?;
        Ok(url)
    }
}

#[async_trait]
impl crate::providers::Provider for GoogleProvider {
    fn id(&self) -> &'static str {
        "google"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Google]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = self.generate_url(&spec.model, false)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "google",
            http_url = %url,
            "sending request to Google"
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
            llm_provider = "google",
            http_status_code = status_code,
            "received response from Google"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "google".to_string(),
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

        // Note: Google uses endpoint-based streaming (:streamGenerateContent), not body parameter
        let url = self.generate_url(&spec.model, true)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "google",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Google"
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
            llm_provider = "google",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Google"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "google".to_string(),
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
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "google".to_string(),
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
