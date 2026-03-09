use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest::{StatusCode, Url};
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{override_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct DatabricksConfig {
    pub api_base: Url,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct DatabricksProvider {
    client: ClientWithMiddleware,
    config: DatabricksConfig,
}

impl DatabricksProvider {
    pub fn new(config: DatabricksConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client =
            override_client().or_else(|_| crate::client::build_middleware_client(&settings))?;
        Ok(Self { client, config })
    }

    pub fn from_config(api_base: Option<&Url>, timeout: Option<Duration>) -> Result<Self> {
        let api_base = api_base
            .cloned()
            .ok_or_else(|| Error::InvalidRequest("databricks provider requires api_base".into()))?;
        Self::new(DatabricksConfig { api_base, timeout })
    }

    // This does not support Databrick's new AI gateway URL format yet, only
    // their model serving endpoints.
    fn serving_url(&self, model: &str) -> Result<Url> {
        let mut url = self.config.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("serving-endpoints");
            segments.push(model);
            segments.push("invocations");
        }
        Ok(url)
    }
}

fn extract_retry_after(status: StatusCode) -> Option<Duration> {
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Some(Duration::from_secs(2))
    } else {
        None
    }
}

#[async_trait]
impl crate::providers::Provider for DatabricksProvider {
    fn id(&self) -> &'static str {
        "databricks"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = self.serving_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "databricks",
            http_url = %url,
            "sending request to Databricks"
        );

        let mut headers = client_headers.to_json_headers();
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
            llm_provider = "databricks",
            http_status_code = status_code,
            "received response from Databricks"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "databricks".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
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

        let url = self.serving_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "databricks",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Databricks"
        );

        let mut headers = client_headers.to_json_headers();
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
            llm_provider = "databricks",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Databricks"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "databricks".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
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
        let mut url = self.config.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("serving-endpoints");
        }
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "databricks".to_string(),
                source: anyhow::anyhow!("status {}", response.status()),
                retry_after: None,
                http: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serving_url_appends_model_and_invocations() {
        let config = DatabricksConfig {
            api_base: Url::parse("https://adb-123.azuredatabricks.net").unwrap(),
            timeout: None,
        };
        let provider = DatabricksProvider::new(config).unwrap();
        let url = provider.serving_url("my-model").unwrap();
        assert_eq!(
            url.as_str(),
            "https://adb-123.azuredatabricks.net/serving-endpoints/my-model/invocations"
        );
    }

    #[test]
    fn serving_url_with_trailing_slash_in_base() {
        let config = DatabricksConfig {
            api_base: Url::parse("https://adb-123.azuredatabricks.net/").unwrap(),
            timeout: None,
        };
        let provider = DatabricksProvider::new(config).unwrap();
        let url = provider.serving_url("llama-3-1-8b").unwrap();
        assert_eq!(
            url.as_str(),
            "https://adb-123.azuredatabricks.net/serving-endpoints/llama-3-1-8b/invocations"
        );
    }

    #[test]
    fn from_config_requires_api_base() {
        let err = DatabricksProvider::from_config(None, None).unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }
}
