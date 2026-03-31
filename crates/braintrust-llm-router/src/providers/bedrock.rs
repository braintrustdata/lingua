use std::future::Future;
use std::pin::Pin;
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
use lingua::universal::message::{Message, UserContent, UserContentPart};
use lingua::util::media::MediaBlock;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Url;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{bedrock_event_stream, RawResponseStream};
use lingua::{ProviderFormat, TransformError};

const BEDROCK_REMOTE_MEDIA_MAX_BYTES: usize = 5 * 1024 * 1024;

type FetchMediaFuture<'a> = Pin<Box<dyn Future<Output = Result<MediaBlock>> + Send + 'a>>;

fn is_remote_image_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

async fn fetch_remote_image_as_base64(url: &str) -> Result<MediaBlock> {
    lingua::util::media::convert_media_to_base64(url, None, Some(BEDROCK_REMOTE_MEDIA_MAX_BYTES))
        .await
        .map_err(|e| Error::InvalidRequest(format!("failed to fetch image URL {url}: {e}")))
}

fn should_inline_remote_image_urls(format: ProviderFormat) -> bool {
    matches!(
        format,
        ProviderFormat::BedrockAnthropic | ProviderFormat::Converse
    )
}

/// Preserve the legacy proxy behavior that fetched remote image URLs before
/// Bedrock request translation. Lingua still owns source detection and target
/// serialization; this hook only prepares the normalized universal request
/// because the network fetch is async and Bedrock targets expect inline media bytes.
///
/// This is idempotent: once a remote URL has been replaced by inline base64 data,
/// subsequent runs leave the image untouched.
async fn prepare_bedrock_universal_request_with_fetch<F>(
    request: &mut lingua::UniversalRequest,
    target_format: ProviderFormat,
    fetch: F,
) -> std::result::Result<(), TransformError>
where
    F: for<'a> Fn(&'a str) -> FetchMediaFuture<'a> + Send + Sync,
{
    if !should_inline_remote_image_urls(target_format) {
        return Ok(());
    }

    inline_remote_image_urls_with_fetch(request, fetch)
        .await
        .map_err(|err| TransformError::ValidationFailed {
            target: target_format,
            reason: err.to_string(),
        })
}

async fn inline_remote_image_urls_with_fetch<F>(
    request: &mut lingua::UniversalRequest,
    fetch: F,
) -> Result<()>
where
    F: for<'a> Fn(&'a str) -> FetchMediaFuture<'a>,
{
    for message in &mut request.messages {
        let content = match message {
            Message::System { content }
            | Message::Developer { content }
            | Message::User { content } => content,
            Message::Assistant { .. } | Message::Tool { .. } => continue,
        };

        let UserContent::Array(parts) = content else {
            continue;
        };

        for part in parts {
            let UserContentPart::Image {
                image, media_type, ..
            } = part
            else {
                continue;
            };

            let Some(url) = image.as_str() else {
                continue;
            };

            if !is_remote_image_url(url) {
                continue;
            }

            let media_block = fetch(url).await?;
            *image = lingua::serde_json::Value::String(media_block.data);
            *media_type = Some(media_block.media_type);
        }
    }

    Ok(())
}
use reqwest_middleware::ClientWithMiddleware;

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
    client: ClientWithMiddleware,
    config: BedrockConfig,
}

impl BedrockProvider {
    pub fn new(config: BedrockConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
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

    fn sign_request(
        &self,
        url: &Url,
        body: &[u8],
        auth: &AuthConfig,
        client_headers: &ClientHeaders,
    ) -> Result<HeaderMap> {
        if let AuthConfig::ApiKey { .. } = auth {
            let mut headers =
                <Self as crate::providers::Provider>::build_headers(self, client_headers);
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

        let mut headers = <Self as crate::providers::Provider>::build_headers(self, client_headers);
        for (name, value) in signed_request.headers().iter() {
            headers.insert(
                name.clone(),
                HeaderValue::from_bytes(value.as_bytes())
                    .map_err(|e| Error::Auth(format!("invalid signed header value: {e}")))?,
            );
        }
        Ok(headers)
    }

    fn build_headers(
        &self,
        url: &Url,
        payload: &[u8],
        auth: &AuthConfig,
        client_headers: &ClientHeaders,
    ) -> Result<HeaderMap> {
        let mut headers = self.sign_request(url, payload, auth, client_headers)?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    async fn send_signed(
        &self,
        url: Url,
        payload: Bytes,
        auth: &AuthConfig,
        client_headers: &ClientHeaders,
    ) -> Result<reqwest::Response> {
        let headers = self.build_headers(&url, payload.as_ref(), auth, client_headers)?;
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
                provider: "bedrock".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: None,
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

    async fn prepare_universal_request(
        &self,
        request: &mut lingua::UniversalRequest,
        ctx: lingua::RequestPreparationContext,
    ) -> std::result::Result<(), TransformError> {
        prepare_bedrock_universal_request_with_fetch(request, ctx.target_format, |url| {
            Box::pin(fetch_remote_image_as_base64(url))
        })
        .await
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let use_invoke = matches!(format, ProviderFormat::BedrockAnthropic);
        let url = if use_invoke {
            self.invoke_model_url(&spec.model, false)?
        } else {
            self.converse_url(&spec.model, false)?
        };
        let response = self.send_signed(url, payload, auth, client_headers).await?;
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
            return self
                .complete_stream_via_complete(payload, auth, spec, format, client_headers)
                .await;
        }

        let use_invoke = matches!(format, ProviderFormat::BedrockAnthropic);
        let url = if use_invoke {
            self.invoke_model_url(&spec.model, true)?
        } else {
            self.converse_url(&spec.model, true)?
        };

        let response = self.send_signed(url, payload, auth, client_headers).await?;
        Ok(bedrock_event_stream(response))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self
            .config
            .endpoint
            .join("list-foundation-models")
            .expect("join models path");
        let body = b"{}";
        let mut headers = self.sign_request(&url, body, auth, &ClientHeaders::default())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ModelFlavor, ModelSpec};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn provider() -> BedrockProvider {
        let config = BedrockConfig {
            endpoint: Url::parse("https://bedrock-runtime.us-east-1.amazonaws.com/").unwrap(),
            service: "bedrock".to_string(),
            timeout: None,
        };
        BedrockProvider::new(config).unwrap()
    }

    fn bedrock_spec(model: &str, format: ProviderFormat) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format,
            flavor: ModelFlavor::Chat,
            display_name: None,
            parent: None,
            input_cost_per_mil_tokens: None,
            output_cost_per_mil_tokens: None,
            input_cache_read_cost_per_mil_tokens: None,
            multimodal: None,
            reasoning: None,
            max_input_tokens: None,
            max_output_tokens: None,
            supports_streaming: true,
            extra: Default::default(),
            available_providers: Default::default(),
        }
    }

    struct BedrockRequestPreparer<F> {
        fetch: F,
    }

    #[async_trait]
    impl<F> lingua::UniversalRequestPreparer for BedrockRequestPreparer<F>
    where
        F: for<'a> Fn(&'a str) -> FetchMediaFuture<'a> + Send + Sync,
    {
        async fn prepare_universal_request(
            &self,
            request: &mut lingua::UniversalRequest,
            ctx: lingua::RequestPreparationContext,
        ) -> std::result::Result<(), TransformError> {
            prepare_bedrock_universal_request_with_fetch(request, ctx.target_format, &self.fetch)
                .await
        }
    }

    async fn transform_bedrock_request_with_fetch<F>(
        body: Bytes,
        spec: &ModelSpec,
        format: ProviderFormat,
        fetch: F,
    ) -> std::result::Result<lingua::TransformResult, TransformError>
    where
        F: for<'a> Fn(&'a str) -> FetchMediaFuture<'a> + Send + Sync,
    {
        let preparer = BedrockRequestPreparer { fetch };
        lingua::transform_request_with_universal_preparation(
            body,
            format,
            Some(&spec.model),
            &preparer,
        )
        .await
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

        let headers = provider
            .build_headers(&url, b"{}", &auth, &ClientHeaders::default())
            .expect("headers");
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

        let err = provider
            .build_headers(&url, b"{}", &auth, &ClientHeaders::default())
            .unwrap_err();
        match err {
            Error::Auth(message) => {
                assert!(message.contains("AwsSignatureV4 or ApiKey"));
            }
            other => panic!("expected Error::Auth, got {other:?}"),
        }
    }

    #[test]
    fn should_inline_remote_image_urls_matches_bedrock_formats() {
        assert!(should_inline_remote_image_urls(
            ProviderFormat::BedrockAnthropic
        ));
        assert!(should_inline_remote_image_urls(ProviderFormat::Converse));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Anthropic));
        assert!(!should_inline_remote_image_urls(
            ProviderFormat::ChatCompletions
        ));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Responses));
        assert!(!should_inline_remote_image_urls(ProviderFormat::Google));
    }

    #[tokio::test]
    async fn prepare_request_passes_through_same_format_converse_without_fetch() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "modelId": "anthropic.claude-3-haiku-20240307-v1:0",
                "messages": [{
                    "role": "user",
                    "content": [{"text": "Hello"}]
                }]
            }))
            .unwrap(),
        );

        let prepared = transform_bedrock_request_with_fetch(
            body.clone(),
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::Converse,
            ),
            ProviderFormat::Converse,
            |_url| {
                Box::pin(async {
                    panic!("fetch should not be called for same-format converse requests");
                })
            },
        )
        .await
        .unwrap();

        assert!(prepared.is_passthrough());
        assert_eq!(prepared.into_bytes(), body);
    }

    #[tokio::test]
    async fn prepare_request_inlines_remote_chat_image_for_converse() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "What is this?"},
                        {"type": "image_url", "image_url": {"url": "https://example.com/image.jpg"}}
                    ]
                }]
            }))
            .unwrap(),
        );

        let prepared = transform_bedrock_request_with_fetch(
            body,
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::Converse,
            ),
            ProviderFormat::Converse,
            |_url| {
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "image/jpeg".to_string(),
                        data: "abcd".to_string(),
                    })
                })
            },
        )
        .await
        .unwrap();
        let value: lingua::serde_json::Value =
            lingua::serde_json::from_slice(prepared.as_bytes()).unwrap();

        let bytes = value
            .pointer("/messages/0/content/1/image/source/bytes")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(
            value
                .pointer("/messages/0/content/1/image/format")
                .and_then(|v| v.as_str()),
            Some("jpeg")
        );
    }

    #[tokio::test]
    async fn prepare_request_inlines_remote_responses_image_for_bedrock_anthropic() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "model": "claude-sonnet-4-5-20250929",
                "input": [{
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "What is this?"},
                        {
                            "type": "input_image",
                            "image_url": "https://example.com/image.jpg",
                            "detail": "auto"
                        }
                    ]
                }]
            }))
            .unwrap(),
        );

        let prepared = transform_bedrock_request_with_fetch(
            body,
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::BedrockAnthropic,
            ),
            ProviderFormat::BedrockAnthropic,
            |_url| {
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "image/jpeg".to_string(),
                        data: "abcd".to_string(),
                    })
                })
            },
        )
        .await
        .unwrap();
        let value: lingua::serde_json::Value =
            lingua::serde_json::from_slice(prepared.as_bytes()).unwrap();

        assert_eq!(
            value.get("anthropic_version").and_then(|v| v.as_str()),
            Some("bedrock-2023-05-31")
        );
        assert_eq!(
            value
                .pointer("/messages/0/content/1/source/type")
                .and_then(|v| v.as_str()),
            Some("base64")
        );
        assert!(value
            .pointer("/messages/0/content/1/source/data")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty()));
        assert_eq!(value.pointer("/messages/0/content/1/source/url"), None);
    }

    #[tokio::test]
    async fn prepare_request_returns_validation_error_when_remote_image_fetch_fails() {
        let fetch_calls = Arc::new(AtomicUsize::new(0));
        let body = Bytes::from(
            lingua::serde_json::to_vec(&lingua::serde_json::json!({
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "What is this?"},
                        {"type": "image_url", "image_url": {"url": "https://example.com/image.jpg"}}
                    ]
                }]
            }))
            .unwrap(),
        );

        let err = transform_bedrock_request_with_fetch(
            body,
            &bedrock_spec(
                "anthropic.claude-3-haiku-20240307-v1:0",
                ProviderFormat::Converse,
            ),
            ProviderFormat::Converse,
            {
                let fetch_calls = Arc::clone(&fetch_calls);
                move |url| {
                    fetch_calls.fetch_add(1, Ordering::SeqCst);
                    Box::pin(async move {
                        Err(Error::InvalidRequest(format!(
                            "failed to fetch image URL {url}: network error"
                        )))
                    })
                }
            },
        )
        .await
        .expect_err("fetch failure should surface as a validation error");

        assert_eq!(fetch_calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            err,
            TransformError::ValidationFailed {
                target: ProviderFormat::Converse,
                ref reason
            } if reason.contains("failed to fetch image URL")
        ));
    }
}
