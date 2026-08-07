use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::ProviderFormat;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::anthropic::{ANTHROPIC_VERSION, DEFAULT_ANTHROPIC_VERSION_VALUE};
use crate::providers::{ClientHeaders, Provider};
use crate::streaming::{sse_stream, RawResponseStream};

#[derive(Debug, Clone)]
struct AzureAiGatewayConfig {
    pub endpoint: Url,
}

#[derive(Debug, Clone)]
pub struct AzureAiGatewayProvider {
    client: ClientWithMiddleware,
    config: AzureAiGatewayConfig,
}

impl AzureAiGatewayProvider {
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        let endpoint = endpoint
            .cloned()
            .ok_or_else(|| Error::InvalidRequest("Azure AI Gateway requires endpoint".into()))?;
        let mut settings = client_settings.unwrap_or_default();
        if let Some(timeout) = timeout {
            settings.request_timeout = timeout;
        }
        Ok(Self {
            client: build_middleware_client(&settings)?,
            config: AzureAiGatewayConfig { endpoint },
        })
    }

    fn url_for_format(&self, format: ProviderFormat) -> Result<Url> {
        let path = match format {
            ProviderFormat::Responses => ["openai", "v1", "responses"].as_slice(),
            ProviderFormat::Anthropic => ["anthropic", "v1", "messages"].as_slice(),
            _ => ["openai", "v1", "chat", "completions"].as_slice(),
        };
        let mut url = self.config.endpoint.clone();
        let mut segments = url.path_segments_mut().map_err(|_| {
            Error::InvalidRequest("Azure AI Gateway endpoint must be absolute".into())
        })?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
        drop(segments);
        Ok(url)
    }

    fn headers(
        &self,
        client_headers: &ClientHeaders,
        auth: &AuthConfig,
        format: ProviderFormat,
    ) -> Result<HeaderMap> {
        let mut headers = client_headers.to_json_headers();
        auth.apply_headers(&mut headers)?;
        if format == ProviderFormat::Anthropic {
            headers.insert(
                ANTHROPIC_VERSION,
                HeaderValue::from_static(DEFAULT_ANTHROPIC_VERSION_VALUE),
            );
        }
        Ok(headers)
    }

    async fn send(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<reqwest::Response> {
        let url = self.url_for_format(format)?;
        let response = self
            .client
            .post(url.clone())
            .headers(self.headers(client_headers, auth, format)?)
            .body(payload)
            .send()
            .await?;
        #[cfg(feature = "tracing")]
        {
            let span = tracing::Span::current();
            span.record("http.url", tracing::field::display(&url));
            span.record("http.status_code", response.status().as_u16());
        }
        Ok(response)
    }

    async fn into_response(response: reqwest::Response) -> Result<Bytes> {
        if response.status().is_success() {
            return Ok(response.bytes().await?);
        }
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        Err(Error::Provider {
            provider: "azure_ai_gateway".to_string(),
            source: anyhow::anyhow!("HTTP {status}: {text}"),
            retry_after: None,
            http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
        })
    }
}

#[async_trait]
impl Provider for AzureAiGatewayProvider {
    fn id(&self) -> &'static str {
        "azure_ai_gateway"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![
            ProviderFormat::ChatCompletions,
            ProviderFormat::Responses,
            ProviderFormat::Anthropic,
        ]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        Self::into_response(self.send(payload, auth, format, client_headers).await?).await
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
        let response = self.send(payload, auth, format, client_headers).await?;
        if response.status().is_success() {
            return Ok(sse_stream(response));
        }
        let status = response.status();
        let headers = response.headers().clone();
        let text = response.text().await?;
        Err(Error::Provider {
            provider: "azure_ai_gateway".to_string(),
            source: anyhow::anyhow!("HTTP {status}: {text}"),
            retry_after: None,
            http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
        })
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let response = self
            .client
            .get(self.url_for_format(ProviderFormat::ChatCompletions)?)
            .headers(self.headers(
                &ClientHeaders::default(),
                auth,
                ProviderFormat::ChatCompletions,
            )?)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "azure_ai_gateway".to_string(),
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

    fn provider() -> AzureAiGatewayProvider {
        AzureAiGatewayProvider::from_config(
            Some(&Url::parse("https://gateway.example/default/models").unwrap()),
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn resolves_all_gateway_routes() {
        let provider = provider();
        assert_eq!(
            provider
                .url_for_format(ProviderFormat::ChatCompletions)
                .unwrap()
                .as_str(),
            "https://gateway.example/default/models/openai/v1/chat/completions"
        );
        assert_eq!(
            provider
                .url_for_format(ProviderFormat::Responses)
                .unwrap()
                .as_str(),
            "https://gateway.example/default/models/openai/v1/responses"
        );
        assert_eq!(
            provider
                .url_for_format(ProviderFormat::Anthropic)
                .unwrap()
                .as_str(),
            "https://gateway.example/default/models/anthropic/v1/messages"
        );
    }

    #[test]
    fn uses_api_key_for_anthropic_requests() {
        let auth = AuthConfig::ApiKey {
            key: "runtime-key".into(),
            header: Some("api-key".into()),
            prefix: None,
        };
        let headers = provider()
            .headers(&ClientHeaders::default(), &auth, ProviderFormat::Anthropic)
            .unwrap();
        assert_eq!(
            headers.get("api-key").and_then(|value| value.to_str().ok()),
            Some("runtime-key")
        );
        assert_eq!(
            headers
                .get(ANTHROPIC_VERSION)
                .and_then(|value| value.to_str().ok()),
            Some(DEFAULT_ANTHROPIC_VERSION_VALUE)
        );
    }
}
