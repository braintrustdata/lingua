use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, RawResponseStream};
use lingua::ProviderFormat;

use super::openai::OpenAIConfig;

/// OpenAI Responses API provider.
///
/// This provider handles requests to OpenAI's `/v1/responses` endpoint,
/// which is separate from the Chat Completions API.
#[derive(Debug, Clone)]
pub struct OpenAIResponsesProvider {
    client: Client,
    config: OpenAIConfig,
}

impl OpenAIResponsesProvider {
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
        Ok(Self { client, config })
    }

    fn responses_url(&self) -> Result<reqwest::Url> {
        let mut url = self.config.endpoint.clone();
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
impl crate::providers::Provider for OpenAIResponsesProvider {
    fn id(&self) -> &'static str {
        "openai-responses"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Responses]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = self.responses_url()?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "openai-responses",
            http_url = %url,
            "sending request to OpenAI Responses API"
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
            llm_provider = "openai-responses",
            http_status_code = status_code,
            "received response from OpenAI Responses API"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "openai-responses".to_string(),
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
        // Responses API doesn't support streaming, return single-bytes stream
        let response = self
            .complete(payload, auth, spec, format, client_headers)
            .await?;
        Ok(single_bytes_stream(response))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self.responses_url()?;
        let mut headers = self.build_headers(&ClientHeaders::default());
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "openai-responses".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_url_builds_correctly() {
        let provider = OpenAIResponsesProvider::new(OpenAIConfig::default()).unwrap();
        let url = provider.responses_url().expect("url");
        assert_eq!(url.as_str(), "https://api.openai.com/v1/responses");
    }
}
