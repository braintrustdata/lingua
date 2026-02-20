use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::Value;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub endpoint: Url,
    pub endpoint_template: Option<String>,
    pub organization: Option<String>,
    pub project: Option<String>,
    pub timeout: Option<Duration>,
    pub api_version: Option<String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.openai.com/v1").expect("valid default URL"),
            endpoint_template: None,
            organization: None,
            project: None,
            timeout: None,
            api_version: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    client: Client,
    config: OpenAIConfig,
    endpoint_template: Option<String>,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = if config.timeout.is_some() {
            crate::client::build_client(&settings)?
        } else {
            default_client().or_else(|_| crate::client::build_client(&settings))?
        };
        let endpoint_template = config.endpoint_template.clone();
        Ok(Self {
            client,
            endpoint_template,
            config,
        })
    }

    /// Create an OpenAI provider from configuration parameters.
    ///
    /// Extracts OpenAI-specific options from metadata:
    /// - `organization_id`: OpenAI organization ID
    /// - `project`: OpenAI project ID
    /// - `api_version`: API version string
    pub fn from_config(
        endpoint: Option<&Url>,
        endpoint_template: Option<&str>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<Self> {
        let mut config = OpenAIConfig::default();

        // Endpoint (either template or direct URL)
        if let Some(template) = endpoint_template {
            config.endpoint_template = Some(template.to_string());
            // Parse template with placeholder replaced for validation
            let fallback = template.replace("<model>", "default");
            config.endpoint = Url::parse(&fallback)
                .map_err(|e| Error::InvalidRequest(format!("invalid endpoint template: {e}")))?;
        } else if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }

        // Timeout
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }

        // OpenAI-specific metadata
        if let Some(org) = metadata.get("organization_id").and_then(Value::as_str) {
            config.organization = Some(org.to_string());
        }
        if let Some(project) = metadata.get("project").and_then(Value::as_str) {
            config.project = Some(project.to_string());
        }
        if let Some(version) = metadata.get("api_version").and_then(Value::as_str) {
            config.api_version = Some(version.to_string());
        }

        Self::new(config)
    }

    fn resolve_base(&self, model: Option<&str>) -> Result<Url> {
        if let Some(template) = &self.endpoint_template {
            let replacement = model.filter(|s| !s.is_empty()).unwrap_or("default");
            let resolved = template.replace("<model>", replacement);
            Url::parse(&resolved).map_err(|e| {
                Error::InvalidRequest(format!("invalid OpenAI endpoint template: {e}"))
            })
        } else {
            Ok(self.config.endpoint.clone())
        }
    }

    fn chat_url(&self, model: Option<&str>) -> Result<Url> {
        let mut url = self.resolve_base(model)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("chat");
            segments.push("completions");
        }
        if let Some(version) = &self.config.api_version {
            url.query_pairs_mut().append_pair("api-version", version);
        }
        Ok(url)
    }

    fn responses_url(&self, model: Option<&str>) -> Result<Url> {
        let mut url = self.resolve_base(model)?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("responses");
        }
        if let Some(version) = &self.config.api_version {
            url.query_pairs_mut().append_pair("api-version", version);
        }
        Ok(url)
    }

    fn apply_headers(&self, headers: &mut HeaderMap) {
        if let Some(org) = &self.config.organization {
            headers.insert(
                "OpenAI-Organization",
                HeaderValue::from_str(org).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        if let Some(project) = &self.config.project {
            headers.insert(
                "OpenAI-Project",
                HeaderValue::from_str(project).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
    }

    fn build_headers(&self, client_headers: &ClientHeaders) -> HeaderMap {
        let mut headers = client_headers.to_json_headers();
        self.apply_headers(&mut headers);
        headers
    }
}

#[async_trait]
impl crate::providers::Provider for OpenAIProvider {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions, ProviderFormat::Responses]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = match format {
            ProviderFormat::Responses => self.responses_url(Some(&spec.model))?,
            _ => self.chat_url(Some(&spec.model))?,
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "openai",
            http_url = %url,
            "sending request to OpenAI"
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
            llm_provider = "openai",
            http_status_code = status_code,
            "received response from OpenAI"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "openai".to_string(),
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
        let url = match format {
            ProviderFormat::Responses => self.responses_url(Some(&spec.model))?,
            _ => self.chat_url(Some(&spec.model))?,
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "openai",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to OpenAI"
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
            llm_provider = "openai",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from OpenAI"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "openai".to_string(),
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
        let url = self.chat_url(None)?;
        let mut headers = self.build_headers(&ClientHeaders::default());
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "openai".to_string(),
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

/// Information about an OpenAI-compatible provider's default endpoint.
#[derive(Debug, Clone, Copy)]
pub struct OpenAICompatibleEndpoint {
    /// The base URL for the provider's API.
    pub url: &'static str,
    /// Whether the URL is a template containing `<model>` placeholder.
    pub is_template: bool,
}

/// Returns true if the provider kind uses the OpenAI API format.
///
/// These providers accept OpenAI-formatted requests and can be used
/// with the `OpenAIProvider` implementation.
pub fn is_openai_compatible(kind: &str) -> bool {
    matches!(
        kind,
        "groq"
            | "fireworks"
            | "perplexity"
            | "together"
            | "replicate"
            | "lepton"
            | "baseten"
            | "cerebras"
            | "xai"
            | "xAI"
            | "ollama"
            | "databricks"
    )
}

/// Returns the default endpoint for an OpenAI-compatible provider.
///
/// Returns `None` for unknown providers or providers that require
/// explicit endpoint configuration (like databricks).
pub fn openai_compatible_endpoint(kind: &str) -> Option<OpenAICompatibleEndpoint> {
    match kind {
        "groq" => Some(OpenAICompatibleEndpoint {
            url: "https://api.groq.com/openai/v1",
            is_template: false,
        }),
        "fireworks" => Some(OpenAICompatibleEndpoint {
            url: "https://api.fireworks.ai/inference/v1",
            is_template: false,
        }),
        "perplexity" => Some(OpenAICompatibleEndpoint {
            url: "https://api.perplexity.ai",
            is_template: false,
        }),
        "together" => Some(OpenAICompatibleEndpoint {
            url: "https://api.together.xyz/v1",
            is_template: false,
        }),
        "replicate" => Some(OpenAICompatibleEndpoint {
            url: "https://openai-proxy.replicate.com/v1",
            is_template: false,
        }),
        "lepton" => Some(OpenAICompatibleEndpoint {
            url: "https://<model>.lepton.run/api/v1/",
            is_template: true,
        }),
        "baseten" => Some(OpenAICompatibleEndpoint {
            url: "https://inference.baseten.co/v1",
            is_template: false,
        }),
        "cerebras" => Some(OpenAICompatibleEndpoint {
            url: "https://api.cerebras.ai/v1",
            is_template: false,
        }),
        "xai" | "xAI" => Some(OpenAICompatibleEndpoint {
            url: "https://api.x.ai/v1",
            is_template: false,
        }),
        "ollama" => Some(OpenAICompatibleEndpoint {
            url: "http://127.0.0.1:11434/v1",
            is_template: false,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_template_endpoint() {
        let config = OpenAIConfig {
            endpoint_template: Some("https://<model>.lepton.run/api/v1/".into()),
            ..Default::default()
        };
        let provider = OpenAIProvider::new(config).unwrap();
        let url = provider.chat_url(Some("my-model")).expect("url");
        assert_eq!(url.host_str().unwrap(), "my-model.lepton.run");
        assert!(url.path().ends_with("/chat/completions"));
    }

    #[test]
    fn resolves_responses_url() {
        let provider = OpenAIProvider::new(OpenAIConfig::default()).unwrap();
        let url = provider.responses_url(Some("gpt-4o")).expect("url");
        assert_eq!(url.as_str(), "https://api.openai.com/v1/responses");
    }

    #[test]
    fn resolves_template_responses_url() {
        let config = OpenAIConfig {
            endpoint_template: Some("https://<model>.lepton.run/api/v1/".into()),
            ..Default::default()
        };
        let provider = OpenAIProvider::new(config).unwrap();
        let url = provider.responses_url(Some("my-model")).expect("url");
        assert_eq!(url.host_str().unwrap(), "my-model.lepton.run");
        assert!(url.path().ends_with("/responses"));
    }

    #[test]
    fn uses_fallback_when_no_model_provided() {
        let config = OpenAIConfig {
            endpoint_template: Some("https://<model>.lepton.run/api/v1/".into()),
            ..Default::default()
        };
        let provider = OpenAIProvider::new(config).unwrap();
        let url = provider.chat_url(None).expect("url");
        assert!(url.as_str().contains("default.lepton.run"));
    }
}
