use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::Value as MetadataValue;
use reqwest::header::HeaderMap;
use reqwest::{StatusCode, Url};
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
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
    client: ClientWithMiddleware,
    config: AzureConfig,
}

impl AzureProvider {
    pub fn new(config: AzureConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
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
        metadata: &std::collections::HashMap<String, MetadataValue>,
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
        if let Some(deployment) = metadata
            .get("deployment")
            .and_then(MetadataValue::as_str)
            .filter(|s| !s.is_empty())
        {
            config.deployment = Some(deployment.to_string());
        }
        if let Some(version) = metadata.get("api_version").and_then(MetadataValue::as_str) {
            config.api_version = version.to_string();
        }
        if let Some(no_named) = metadata
            .get("no_named_deployment")
            .and_then(MetadataValue::as_bool)
        {
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

    fn responses_url(&self) -> Result<Url> {
        let mut url = self.config.endpoint.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("Azure endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("openai");
            segments.push("v1");
            segments.push("responses");
        }
        Ok(url)
    }

    fn url_for_format(&self, model: &str, format: ProviderFormat) -> Result<Url> {
        match format {
            ProviderFormat::Responses => self.responses_url(),
            _ => self.chat_url(model),
        }
    }

    fn prepare_payload(
        &self,
        payload: Bytes,
        model: &str,
        format: ProviderFormat,
    ) -> Result<Bytes> {
        if format != ProviderFormat::Responses {
            return Ok(payload);
        }

        let mut value: crate::serde_json::Value = crate::serde_json::from_slice(&payload)?;
        let object = value.as_object_mut().ok_or_else(|| {
            Error::InvalidRequest("Azure Responses payload must be a JSON object".into())
        })?;
        object.insert(
            "model".to_string(),
            crate::serde_json::Value::String(self.deployment_for_request(model)?),
        );

        Ok(Bytes::from(crate::serde_json::to_vec(&value)?))
    }
}

#[async_trait]
impl crate::providers::Provider for AzureProvider {
    fn id(&self) -> &'static str {
        "azure"
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
        let payload = self.prepare_payload(payload, &spec.model, format)?;
        let url = self.url_for_format(&spec.model, format)?;

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
        let payload = self.prepare_payload(payload, &spec.model, format)?;
        let url = self.url_for_format(&spec.model, format)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::{self, json};
    use std::collections::HashMap;

    fn endpoint() -> Url {
        Url::parse("https://myorg.openai.azure.com/").unwrap()
    }

    fn make_provider(metadata: HashMap<String, MetadataValue>) -> AzureProvider {
        AzureProvider::from_config(Some(&endpoint()), None, &metadata).unwrap()
    }

    #[test]
    fn empty_deployment_string_falls_back_to_model_name() {
        let mut metadata = HashMap::new();
        metadata.insert("deployment".into(), MetadataValue::String("".into()));
        let provider = make_provider(metadata);

        assert!(provider.config.deployment.is_none());

        let url = provider.chat_url("gpt-4o").unwrap();
        assert_eq!(
            url.as_str(),
            "https://myorg.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2023-07-01-preview"
        );
    }

    #[test]
    fn explicit_deployment_overrides_model_name() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "deployment".into(),
            MetadataValue::String("my-deploy".into()),
        );
        let provider = make_provider(metadata);

        let url = provider.chat_url("gpt-4o").unwrap();
        assert_eq!(
            url.as_str(),
            "https://myorg.openai.azure.com/openai/deployments/my-deploy/chat/completions?api-version=2023-07-01-preview"
        );
    }

    #[test]
    fn resolves_responses_url() {
        let provider = make_provider(HashMap::new());
        let url = provider.responses_url().unwrap();

        assert_eq!(
            url.as_str(),
            "https://myorg.openai.azure.com/openai/v1/responses"
        );
    }

    #[test]
    fn responses_payload_uses_explicit_deployment_name() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "deployment".into(),
            MetadataValue::String("my-deploy".into()),
        );
        let provider = make_provider(metadata);
        let payload =
            Bytes::from(serde_json::to_vec(&json!({"model": "gpt-5-pro", "input": "hi"})).unwrap());

        let payload = provider
            .prepare_payload(payload, "gpt-5-pro", ProviderFormat::Responses)
            .unwrap();
        let json: crate::serde_json::Value = serde_json::from_slice(&payload).unwrap();

        assert_eq!(
            json.get("model").and_then(|v| v.as_str()),
            Some("my-deploy")
        );
    }

    #[test]
    fn responses_payload_falls_back_to_model_name() {
        let provider = make_provider(HashMap::new());
        let payload = Bytes::from(
            serde_json::to_vec(&json!({"model": "placeholder", "input": "hi"})).unwrap(),
        );

        let payload = provider
            .prepare_payload(payload, "gpt-4o", ProviderFormat::Responses)
            .unwrap();
        let json: crate::serde_json::Value = serde_json::from_slice(&payload).unwrap();

        assert_eq!(json.get("model").and_then(|v| v.as_str()), Some("gpt-4o"));
    }
}
