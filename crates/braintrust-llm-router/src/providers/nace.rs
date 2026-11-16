use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::ProviderFormat;
use reqwest::header::HeaderValue;
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::RawResponseStream;

const NACE_CLIENT: HeaderValue =
    HeaderValue::from_static(concat!("braintrust/", env!("CARGO_PKG_VERSION")));

#[derive(Debug, Clone)]
pub struct NaceConfig {
    pub endpoint: Url,
    pub timeout: Option<Duration>,
}

impl Default for NaceConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://console.nace.ai").expect("valid default URL"),
            timeout: None,
        }
    }
}

/// Nace Drex typed decisions, using either OpenAI Decisions or System One.
#[derive(Debug, Clone)]
pub struct NaceProvider {
    client: ClientWithMiddleware,
    config: NaceConfig,
}

impl NaceProvider {
    pub fn new(config: NaceConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: NaceConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if !matches!(config.endpoint.scheme(), "http" | "https") {
            return Err(Error::InvalidRequest(
                "Nace endpoint must use HTTP or HTTPS".into(),
            ));
        }
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        let mut config = NaceConfig::default();
        if let Some(endpoint) = endpoint {
            config.endpoint = endpoint.clone();
        }
        config.timeout = timeout;
        Self::new_with_client_settings(config, client_settings.unwrap_or_default())
    }

    /// Preserve an optional proxy prefix and avoid appending `/v1` twice.
    fn api_url(&self, path: &str) -> Result<Url> {
        let mut url = self.config.endpoint.clone();
        let has_version = url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            == Some("v1");
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            if !has_version {
                segments.push("v1");
            }
            segments.push(path);
        }
        Ok(url)
    }

    async fn response_bytes(response: reqwest::Response) -> Result<Bytes> {
        if response.status().is_success() {
            return Ok(response.bytes().await?);
        }
        let status = response.status();
        let headers = response.headers().clone();
        let text = response
            .text()
            .await
            .unwrap_or_else(|error| format!("failed to read response body: {error}"));
        Err(Error::Provider {
            provider: "nace".to_string(),
            source: anyhow::anyhow!("HTTP {status}: {text}"),
            // Preserve Retry-After in the HTTP details, like the other providers.
            retry_after: None,
            http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
        })
    }
}

#[async_trait]
impl crate::providers::Provider for NaceProvider {
    fn id(&self) -> &'static str {
        "nace"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Decisions, ProviderFormat::TypeSafe]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let path = match format {
            ProviderFormat::Decisions => "decisions",
            ProviderFormat::TypeSafe => "systemone",
            _ => return Err(Error::NoProvider(format)),
        };
        let url = self.api_url(path)?;
        let mut headers = client_headers.to_json_headers();
        auth.apply_headers(&mut headers)?;
        headers.insert("x-nace-client", NACE_CLIENT);
        if format == ProviderFormat::TypeSafe {
            // The TypeSafe score schema expects a dictionary of level labels.
            headers.insert("x-nace-legend", HeaderValue::from_static("object"));
        }
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

        Self::response_bytes(response).await
    }

    async fn complete_stream(
        &self,
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> Result<RawResponseStream> {
        Err(Error::InvalidRequest(
            "Nace Drex decisions do not support streaming".into(),
        ))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self.api_url("models")?;
        let mut headers = ClientHeaders::default().to_json_headers();
        auth.apply_headers(&mut headers)?;
        headers.insert("x-nace-client", NACE_CLIENT);
        let response = self.client.get(url).headers(headers).send().await?;
        Self::response_bytes(response).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_accepts_origin_and_versioned_proxy_endpoint() {
        for (endpoint, prefix) in [
            ("https://console.nace.ai", "https://console.nace.ai/v1"),
            ("https://console.nace.ai/", "https://console.nace.ai/v1"),
            ("https://console.nace.ai/v1", "https://console.nace.ai/v1"),
            (
                "https://proxy.example.com/nace/v1/",
                "https://proxy.example.com/nace/v1",
            ),
        ] {
            let provider =
                NaceProvider::from_config(Some(&Url::parse(endpoint).unwrap()), None, None)
                    .unwrap();
            for path in ["decisions", "systemone", "models"] {
                assert_eq!(
                    provider.api_url(path).unwrap().as_str(),
                    format!("{prefix}/{path}")
                );
            }
        }
    }
}
