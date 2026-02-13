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

    fn converse_url(&self, model: &str, stream: bool) -> Result<Url> {
        let path = if stream {
            format!("model/{model}/converse-stream")
        } else {
            format!("model/{model}/converse")
        };
        self.config
            .endpoint
            .join(&path)
            .map_err(|e| Error::InvalidRequest(format!("failed to build converse url: {e}")))
    }

    fn invoke_model_url(&self, model: &str, stream: bool) -> Result<Url> {
        let path = if stream {
            format!("model/{model}/invoke-with-response-stream")
        } else {
            format!("model/{model}/invoke")
        };
        self.config
            .endpoint
            .join(&path)
            .map_err(|e| Error::InvalidRequest(format!("failed to build invoke url: {e}")))
    }

    fn sign_request(&self, url: &Url, body: &[u8], auth: &AuthConfig) -> Result<HeaderMap> {
        if let AuthConfig::ApiKey { .. } = auth {
            // TODO: should this instead be: `let mut headers =
            // self.build_headers(client_headers)`?
            let mut headers = HeaderMap::new();
            auth.apply_headers(&mut headers)?;
            return Ok(headers);
        }

        let (access_key, secret_key, session_token, region, service) =
            auth.aws_credentials().ok_or_else(|| {
                Error::Auth("AwsSignatureV4 or ApiKey credentials required for Bedrock".into())
            })?;
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

    async fn send_signed(
        &self,
        url: Url,
        payload: Bytes,
        auth: &AuthConfig,
        stream: bool,
    ) -> Result<reqwest::Response> {
        #[cfg(not(feature = "tracing"))]
        let _ = stream;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "bedrock",
            http_url = %url,
            llm_streaming = stream,
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
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "bedrock",
            http_status_code = response.status().as_u16(),
            llm_streaming = stream,
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
                http: Some(UpstreamHttpError::new(status.as_u16(), headers, text)),
            });
        }

        Ok(response)
    }
}

#[async_trait]
impl crate::providers::Provider for BedrockProvider {
    fn id(&self) -> &'static str {
        "bedrock"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Converse, ProviderFormat::BedrockAnthropic]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let use_invoke = matches!(format, ProviderFormat::BedrockAnthropic);
        let url = if use_invoke {
            self.invoke_model_url(&spec.model, false)?
        } else {
            self.converse_url(&spec.model, false)?
        };
        let response = self.send_signed(url, payload, auth, false).await?;
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

        let use_invoke = matches!(format, ProviderFormat::BedrockAnthropic);
        let url = if use_invoke {
            self.invoke_model_url(&spec.model, true)?
        } else {
            self.converse_url(&spec.model, true)?
        };

        let response = self.send_signed(url, payload, auth, true).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> BedrockProvider {
        let config = BedrockConfig {
            endpoint: Url::parse("https://bedrock-runtime.us-east-1.amazonaws.com/").unwrap(),
            service: "bedrock".to_string(),
            timeout: None,
        };
        BedrockProvider::new(config).unwrap()
    }

    #[test]
    fn build_headers_supports_api_key_auth() {
        let provider = provider();
        let url = provider
            .converse_url("anthropic.claude-3-haiku-20240307-v1:0", false)
            .unwrap();
        let auth = AuthConfig::ApiKey {
            key: "test-api-key".into(),
            header: Some("x-api-key".into()),
            prefix: None,
        };

        let headers = provider.build_headers(&url, b"{}", &auth).expect("headers");
        assert_eq!(
            headers.get("content-type"),
            Some(&HeaderValue::from_static("application/json"))
        );
        assert_eq!(
            headers.get("x-api-key"),
            Some(&HeaderValue::from_static("test-api-key"))
        );
    }

    #[test]
    fn build_headers_rejects_unsupported_auth_modes() {
        let provider = provider();
        let url = provider
            .converse_url("anthropic.claude-3-haiku-20240307-v1:0", false)
            .unwrap();
        let auth = AuthConfig::OAuth {
            access_token: "token".into(),
            token_type: Some("Bearer".into()),
        };

        let err = provider.build_headers(&url, b"{}", &auth).unwrap_err();
        match err {
            Error::Auth(message) => {
                assert!(message.contains("AwsSignatureV4 or ApiKey"));
            }
            other => panic!("expected Error::Auth, got {other:?}"),
        }
    }
}
