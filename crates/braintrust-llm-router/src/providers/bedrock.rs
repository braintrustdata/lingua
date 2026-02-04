use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sigv4::http_request::{
    sign, SignableBody, SignableRequest, SigningParams, SigningSettings,
};
use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::identity::Identity;
use bytes::Bytes;
use http::Request as HttpRequest;
use lingua::serde_json::Value;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{bedrock_event_stream, single_bytes_stream, RawResponseStream};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct BedrockConfig {
    pub endpoint: Url,
    pub service: String,
    pub timeout: Option<Duration>,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://bedrock-runtime.us-east-1.amazonaws.com/")
                .expect("valid Bedrock endpoint"),
            service: "bedrock".to_string(),
            timeout: None,
        }
    }
}

/// Bedrock API mode - determines which endpoint to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockMode {
    /// AWS Converse API - unified format for all Bedrock models
    Converse,
    /// Anthropic Messages API - native format for Claude models on Bedrock
    AnthropicMessages,
}

#[derive(Debug, Clone)]
pub struct BedrockProvider {
    client: Client,
    config: BedrockConfig,
}

impl BedrockProvider {
    pub fn new(config: BedrockConfig) -> Result<Self> {
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

    /// Create a Bedrock provider from configuration parameters.
    ///
    /// Extracts Bedrock-specific options from metadata:
    /// - `region`: AWS region (used to construct endpoint if not provided)
    /// - `service`: AWS service name (defaults to "bedrock")
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<Self> {
        let mut config = BedrockConfig::default();

        // Endpoint from param or construct from region
        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        } else if let Some(region) = metadata.get("region").and_then(Value::as_str) {
            let url = format!("https://bedrock-runtime.{region}.amazonaws.com/");
            config.endpoint = Url::parse(&url)
                .map_err(|e| Error::InvalidRequest(format!("invalid Bedrock region: {e}")))?;
        }

        if let Some(service) = metadata.get("service").and_then(Value::as_str) {
            config.service = service.to_string();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }

        Self::new(config)
    }

    /// Determine which Bedrock API mode to use based on model name.
    pub fn determine_mode(&self, model: &str) -> BedrockMode {
        if model.starts_with("anthropic.") {
            BedrockMode::AnthropicMessages
        } else {
            BedrockMode::Converse
        }
    }

    /// Build the invoke URL for a specific mode.
    pub fn invoke_url_for_mode(
        &self,
        model: &str,
        mode: &BedrockMode,
        stream: bool,
    ) -> Result<Url> {
        let path = match (mode, stream) {
            (BedrockMode::Converse, false) => format!("model/{model}/converse"),
            (BedrockMode::Converse, true) => format!("model/{model}/converse-stream"),
            (BedrockMode::AnthropicMessages, false) => format!("model/{model}/invoke"),
            (BedrockMode::AnthropicMessages, true) => {
                format!("model/{model}/invoke-with-response-stream")
            }
        };
        self.config
            .endpoint
            .join(&path)
            .map_err(|e| Error::InvalidRequest(format!("failed to build invoke url: {e}")))
    }

    fn sign_request(&self, url: &Url, body: &[u8], auth: &AuthConfig) -> Result<HeaderMap> {
        let (access_key, secret_key, session_token, region, service) = auth
            .aws_credentials()
            .ok_or_else(|| Error::Auth("AWS credentials required for Bedrock".into()))?;
        let service = if service.is_empty() {
            &self.config.service
        } else {
            service
        };

        let mut header_pairs: Vec<(String, String)> =
            vec![("content-type".to_string(), "application/json".to_string())];
        let mut builder = HttpRequest::builder()
            .method("POST")
            .uri(url.as_str())
            .header("content-type", "application/json");
        if let Some(token) = session_token {
            builder = builder.header("x-amz-security-token", token);
            header_pairs.push(("x-amz-security-token".to_string(), token.to_string()));
        }
        let host_header_value = url
            .host_str()
            .map(|host| match url.port() {
                Some(port) => format!("{host}:{port}"),
                None => host.to_string(),
            })
            .ok_or_else(|| Error::InvalidRequest("Bedrock endpoint missing host".into()))?;
        builder = builder.header("host", host_header_value.as_str());
        header_pairs.push(("host".to_string(), host_header_value));
        let request = builder
            .body(body.to_vec())
            .map_err(|e| Error::InvalidRequest(format!("failed to build http request: {e}")))?;

        let signing_settings = SigningSettings::default();
        let credentials = Credentials::new(
            access_key,
            secret_key,
            session_token.map(|token| token.to_string()),
            None,
            "braintrust-llm-router",
        );
        let identity: Identity = credentials.into();
        let signing_params: SigningParams = v4::SigningParams::builder()
            .identity(&identity)
            .region(region)
            .name(service)
            .time(SystemTime::now())
            .settings(signing_settings)
            .build()
            .map_err(|e| Error::Auth(format!("failed to build signing params: {e}")))?
            .into();

        let signable = SignableRequest::new(
            request.method().as_str(),
            request.uri().to_string(),
            header_pairs
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str())),
            SignableBody::Bytes(body),
        )
        .map_err(|e| Error::Auth(format!("failed to construct signable request: {e}")))?;
        let (instructions, _) = sign(signable, &signing_params)
            .map_err(|e| Error::Auth(format!("failed to sign request: {e}")))?
            .into_parts();

        let mut signed_request = request;
        instructions.apply_to_request_http1x(&mut signed_request);

        let mut headers = HeaderMap::new();
        for (name, value) in signed_request.headers().iter() {
            headers.insert(
                name.clone(),
                HeaderValue::from_bytes(value.as_bytes())
                    .map_err(|e| Error::Auth(format!("invalid signed header value: {e}")))?,
            );
        }
        Ok(headers)
    }

    fn build_headers(&self, url: &Url, payload: &[u8], auth: &AuthConfig) -> Result<HeaderMap> {
        let mut headers = self.sign_request(url, payload, auth)?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

#[async_trait]
impl crate::providers::Provider for BedrockProvider {
    fn id(&self) -> &'static str {
        "bedrock"
    }

    fn format(&self) -> ProviderFormat {
        ProviderFormat::Converse
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        _client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let mode = self.determine_mode(&spec.model);
        let url = self.invoke_url_for_mode(&spec.model, &mode, false)?;
        let payload = if mode == BedrockMode::AnthropicMessages {
            prepare_anthropic_payload(payload)?
        } else {
            payload
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "bedrock",
            http_url = %url,
            "sending request to Bedrock"
        );

        let headers = self.build_headers(&url, payload.as_ref(), auth)?;

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
            llm_provider = "bedrock",
            http_status_code = status_code,
            "received response from Bedrock"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "bedrock".to_string(),
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

        // Router should have already added stream options to payload
        let mode = self.determine_mode(&spec.model);
        let url = self.invoke_url_for_mode(&spec.model, &mode, true)?;
        let payload = if mode == BedrockMode::AnthropicMessages {
            prepare_anthropic_payload(payload)?
        } else {
            payload
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "bedrock",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Bedrock"
        );

        let headers = self.build_headers(&url, payload.as_ref(), auth)?;

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
            llm_provider = "bedrock",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Bedrock"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "bedrock".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status, &text),
                http: Some(UpstreamHttpError::new(
                    status.as_u16(),
                    headers,
                    text.clone(),
                )),
            });
        }

        Ok(bedrock_event_stream(response))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self
            .config
            .endpoint
            .join("list-foundation-models")
            .expect("join models path");
        let body = b"{}";
        let mut headers = self.sign_request(&url, body, auth)?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body.to_vec())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "bedrock".to_string(),
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

/// Prepare an Anthropic-format payload for Bedrock by adding anthropic_version.
pub fn prepare_anthropic_payload(payload: Bytes) -> Result<Bytes> {
    let mut body: lingua::serde_json::Value = lingua::serde_json::from_slice(&payload)
        .map_err(|e| Error::InvalidRequest(format!("failed to parse payload: {e}")))?;
    if let Some(obj) = body.as_object_mut() {
        obj.insert(
            "anthropic_version".into(),
            lingua::serde_json::Value::String("bedrock-2023-05-31".into()),
        );
    }
    let bytes = lingua::serde_json::to_vec(&body)
        .map_err(|e| Error::InvalidRequest(format!("failed to serialize payload: {e}")))?;
    Ok(Bytes::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> BedrockProvider {
        let config = BedrockConfig {
            endpoint: Url::parse("https://bedrock-runtime.us-east-1.amazonaws.com/").unwrap(),
            service: "bedrock".into(),
            timeout: None,
        };
        BedrockProvider::new(config).unwrap()
    }

    #[test]
    fn selects_anthropic_mode_for_claude_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("anthropic.claude-3-sonnet-20240229-v1:0"),
            BedrockMode::AnthropicMessages
        ));
        assert!(matches!(
            provider.determine_mode("anthropic.claude-3-haiku-20240307-v1:0"),
            BedrockMode::AnthropicMessages
        ));
    }

    #[test]
    fn selects_converse_mode_for_other_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("amazon.titan-text-express-v1"),
            BedrockMode::Converse
        ));
        assert!(matches!(
            provider.determine_mode("meta.llama3-70b-instruct-v1:0"),
            BedrockMode::Converse
        ));
    }

    #[test]
    fn builds_invoke_endpoint_for_anthropic() {
        let provider = provider();
        let url = provider
            .invoke_url_for_mode(
                "anthropic.claude-3-sonnet",
                &BedrockMode::AnthropicMessages,
                false,
            )
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/anthropic.claude-3-sonnet/invoke"
        );
    }

    #[test]
    fn builds_invoke_stream_endpoint_for_anthropic() {
        let provider = provider();
        let url = provider
            .invoke_url_for_mode(
                "anthropic.claude-3-sonnet",
                &BedrockMode::AnthropicMessages,
                true,
            )
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/anthropic.claude-3-sonnet/invoke-with-response-stream"
        );
    }

    #[test]
    fn builds_converse_endpoint_for_others() {
        let provider = provider();
        let url = provider
            .invoke_url_for_mode("amazon.titan-text-express", &BedrockMode::Converse, false)
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/amazon.titan-text-express/converse"
        );
    }

    #[test]
    fn prepares_anthropic_payload_with_version() {
        let payload = Bytes::from(r#"{"model":"claude","messages":[]}"#);
        let result = prepare_anthropic_payload(payload).unwrap();
        let body: lingua::serde_json::Value = lingua::serde_json::from_slice(&result).unwrap();
        assert_eq!(body.get("anthropic_version").unwrap(), "bedrock-2023-05-31");
    }
}
