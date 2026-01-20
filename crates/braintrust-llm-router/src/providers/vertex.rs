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

const DEFAULT_LOCATION: &str = "us-central1";

#[derive(Debug, Clone)]
pub struct VertexConfig {
    pub endpoint: Url,
    pub project: String,
    pub location: String,
    pub timeout: Option<Duration>,
}

impl Default for VertexConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://us-central1-aiplatform.googleapis.com/")
                .expect("valid Vertex endpoint"),
            project: String::new(),
            location: DEFAULT_LOCATION.to_string(),
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VertexProvider {
    client: Client,
    config: VertexConfig,
}

impl VertexProvider {
    pub fn new(config: VertexConfig) -> Result<Self> {
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

    /// Create a Vertex provider from configuration parameters.
    ///
    /// Extracts Vertex-specific options from metadata:
    /// - `project`: GCP project ID (required)
    /// - `location`: GCP region (defaults to us-central1)
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<Self> {
        let project = metadata
            .get("project")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidRequest("Vertex requires project".into()))?
            .to_string();

        let location = metadata
            .get("location")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_LOCATION.to_string());

        let endpoint = endpoint.cloned().unwrap_or_else(|| {
            Url::parse(&format!("https://{location}-aiplatform.googleapis.com/"))
                .expect("valid Vertex endpoint")
        });

        let config = VertexConfig {
            endpoint,
            project,
            location,
            timeout,
        };

        Self::new(config)
    }

    fn determine_mode(&self, model: &str) -> VertexMode {
        if model.starts_with("publishers/meta") {
            VertexMode::OpenApi
        } else if model.starts_with("publishers/") {
            VertexMode::Generative {
                model_path: model.to_string(),
            }
        } else {
            VertexMode::Generative {
                model_path: format!("publishers/google/models/{}", model),
            }
        }
    }

    fn endpoint_for_mode(&self, mode: &VertexMode, stream: bool) -> Result<Url> {
        let location = &self.config.location;
        match mode {
            VertexMode::Generative { model_path } => {
                let method = if stream {
                    "streamGenerateContent"
                } else {
                    "generateContent"
                };
                let mut url = self.config.endpoint.clone();
                let path = format!(
                    "v1/projects/{}/locations/{}/{}:{}",
                    self.config.project, location, model_path, method
                );
                url.set_path(&path);
                if stream {
                    url.query_pairs_mut().append_pair("alt", "sse");
                }
                Ok(url)
            }
            VertexMode::OpenApi => {
                let mut url = self.config.endpoint.clone();
                let path = format!(
                    "v1beta1/projects/{}/locations/{}/endpoints/openapi/chat/completions",
                    self.config.project, location
                );
                url.set_path(&path);
                Ok(url)
            }
        }
    }
}

#[async_trait]
impl crate::providers::Provider for VertexProvider {
    fn id(&self) -> &'static str {
        "vertex"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::Google
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let mode = self.determine_mode(&spec.model);
        let url = self.endpoint_for_mode(&mode, false)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "vertex",
            http_url = %url,
            "sending request to Vertex"
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
            llm_provider = "vertex",
            http_status_code = status_code,
            "received response from Vertex"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "vertex".to_string(),
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
        client_headers: &ClientHeaders,
    ) -> Result<RawResponseStream> {
        if !spec.supports_streaming {
            let response = self.complete(payload, auth, spec, client_headers).await?;
            return Ok(single_bytes_stream(response));
        }

        let mode = self.determine_mode(&spec.model);
        let url = self.endpoint_for_mode(&mode, true)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "vertex",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Vertex"
        );

        let mut headers = self.build_headers(client_headers);
        auth.apply_headers(&mut headers)?;

        // Router should have already added stream options to payload
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
            llm_provider = "vertex",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Vertex"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "vertex".to_string(),
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
        let mode = self.determine_mode("");
        let url = self.endpoint_for_mode(&mode, false)?;
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "vertex".to_string(),
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

enum VertexMode {
    Generative { model_path: String },
    OpenApi,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> VertexProvider {
        let config = VertexConfig {
            endpoint: Url::parse("https://us-central1-aiplatform.googleapis.com/").unwrap(),
            project: "test-project".into(),
            location: "us-central1".into(),
            timeout: None,
        };
        VertexProvider::new(config).unwrap()
    }

    #[test]
    fn selects_generative_mode_by_default() {
        let provider = provider();
        match provider.determine_mode("gemini-pro") {
            VertexMode::Generative { model_path } => {
                assert_eq!(model_path, "publishers/google/models/gemini-pro");
            }
            VertexMode::OpenApi => panic!("expected generative mode"),
        }
    }

    #[test]
    fn selects_openapi_mode_for_meta_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("publishers/meta/models/llama"),
            VertexMode::OpenApi
        ));
    }

    #[test]
    fn builds_generative_endpoint() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/google/models/gemini-pro");
        let url = provider.endpoint_for_mode(&mode, false).expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-pro:generateContent"
        );
    }

    #[test]
    fn builds_openapi_endpoint() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/meta/models/llama");
        let url = provider.endpoint_for_mode(&mode, false).expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1beta1/projects/test-project/locations/us-central1/endpoints/openapi/chat/completions"
        );
    }
}
