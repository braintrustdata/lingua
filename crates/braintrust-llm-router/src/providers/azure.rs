use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::Value;
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
pub struct AzureConfig {
    pub endpoint: Url,
    pub deployment: Option<String>,
    pub api_version: String,
    pub timeout: Option<Duration>,
    pub no_named_deployment: bool,
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://example.openai.azure.com/")
                .expect("valid Azure endpoint"),
            deployment: None,
            api_version: "2023-07-01-preview".to_string(),
            timeout: None,
            no_named_deployment: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AzureProvider {
    client: Client,
    config: AzureConfig,
}

impl AzureProvider {
    pub fn new(config: AzureConfig) -> Result<Self> {
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

    /// Create an Azure provider from configuration parameters.
    ///
    /// Extracts Azure-specific options from metadata:
    /// - `deployment`: Azure deployment name
    /// - `api_version`: API version string
    /// - `no_named_deployment`: Skip deployment in URL path
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<Self> {
        let endpoint = endpoint
            .cloned()
            .ok_or_else(|| Error::InvalidRequest("Azure requires endpoint".into()))?;

        let mut config = AzureConfig {
            endpoint,
            ..Default::default()
        };

        if let Some(t) = timeout {
            config.timeout = Some(t);
        }
        if let Some(deployment) = metadata.get("deployment").and_then(Value::as_str) {
            config.deployment = Some(deployment.to_string());
        }
        if let Some(version) = metadata.get("api_version").and_then(Value::as_str) {
            config.api_version = version.to_string();
        }
        if let Some(no_named) = metadata.get("no_named_deployment").and_then(Value::as_bool) {
            config.no_named_deployment = no_named;
        }

        Self::new(config)
    }

    fn deployment_for_request(&self, model: &str) -> Result<String> {
        if let Some(deployment) = &self.config.deployment {
            return Ok(deployment.clone());
        }
        if !model.is_empty() {
            return Ok(normalize_deployment(model));
        }
        Err(Error::InvalidRequest(
            "Azure provider requires a deployment name".into(),
        ))
    }

    fn chat_url(&self, model: &str) -> Result<Url> {
        let deployment = if self.config.no_named_deployment {
            None
        } else {
            Some(self.deployment_for_request(model)?)
        };

        let mut url = self.config.endpoint.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("Azure endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("openai");
            if let Some(dep) = deployment.as_deref() {
                segments.push("deployments");
                segments.push(dep);
            }
            segments.push("chat");
            segments.push("completions");
        }

        if !url.query_pairs().any(|(key, _)| key == "api-version") {
            url.query_pairs_mut()
                .append_pair("api-version", &self.config.api_version);
        }
        Ok(url)
    }
}

#[async_trait]
impl crate::providers::Provider for AzureProvider {
    fn id(&self) -> &'static str {
        "azure"
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
        let url = self.chat_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "azure",
            http_url = %url,
            "sending request to Azure"
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
            llm_provider = "azure",
            http_status_code = status_code,
            "received response from Azure"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "azure".to_string(),
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
        let url = self.chat_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "azure",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Azure"
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
            llm_provider = "azure",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Azure"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "azure".to_string(),
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
        let url = self.chat_url("")?;
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "azure".to_string(),
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

fn normalize_deployment(name: &str) -> String {
    if name.contains("gpt-3.5") {
        name.replace("gpt-3.5", "gpt-35")
    } else {
        name.to_string()
    }
}
