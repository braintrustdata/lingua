use std::time::Duration;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::{send_with_response_header_capture, ClientHeaders};
use crate::streaming::{sse_stream, RawResponseStream};
use async_trait::async_trait;
use bytes::Bytes;
use lingua::ProviderFormat;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub endpoint: Url,
    pub timeout: Option<Duration>,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://generativelanguage.googleapis.com/v1beta/")
                .expect("valid Google endpoint"),
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoogleProvider {
    client: ClientWithMiddleware,
    config: GoogleConfig,
}

impl GoogleProvider {
    pub fn new(config: GoogleConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: GoogleConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    /// Create a Google provider from configuration parameters.
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        let mut config = GoogleConfig::default();

        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }

        Self::new_with_client_settings(config, client_settings.unwrap_or_default())
    }

    fn chat_completions_url(&self) -> Result<Url> {
        let mut url = self.config.endpoint.clone();
        let has_openai_suffix = url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            == Some("openai");
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            if !has_openai_suffix {
                segments.push("openai");
            }
            segments.push("chat");
            segments.push("completions");
        }
        Ok(url)
    }

    fn generate_url(&self, model: &str, stream: bool) -> Result<Url> {
        let path = if model.starts_with("models/") {
            model.to_string()
        } else {
            format!("models/{model}")
        };
        let suffix = if stream {
            // Use alt=sse to get SSE format for streaming responses
            ":streamGenerateContent?alt=sse"
        } else {
            ":generateContent"
        };
        let url = self
            .config
            .endpoint
            .join(&(path + suffix))
            .map_err(|e| Error::InvalidRequest(format!("invalid Google model path: {e}")))?;
        Ok(url)
    }

    fn url_for_format(&self, model: &str, stream: bool, format: ProviderFormat) -> Result<Url> {
        if format == ProviderFormat::ChatCompletions {
            return self.chat_completions_url();
        }

        self.generate_url(model, stream)
    }

    fn apply_auth_headers(
        &self,
        headers: &mut HeaderMap,
        auth: &AuthConfig,
        format: ProviderFormat,
    ) -> Result<()> {
        if format == ProviderFormat::ChatCompletions {
            if let Some(key) = auth.api_key() {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {key}"))
                        .map_err(|e| Error::Auth(format!("invalid auth header: {e}")))?,
                );
                return Ok(());
            }
        }

        auth.apply_headers(headers)
    }
}

#[async_trait]
impl crate::providers::Provider for GoogleProvider {
    fn id(&self) -> &'static str {
        "google"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Google, ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let url = self.url_for_format(&spec.model, false, format)?;

        let mut headers = self.build_headers(client_headers);
        self.apply_auth_headers(&mut headers, auth, format)?;

        let response = send_with_response_header_capture(
            self.client.post(url.clone()).headers(headers).body(payload),
            client_headers,
        )
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
                provider: "google".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: None,
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
            return self
                .complete_stream_via_complete(payload, auth, spec, format, client_headers)
                .await;
        }

        let url = self.url_for_format(&spec.model, true, format)?;

        let mut headers = self.build_headers(client_headers);
        self.apply_auth_headers(&mut headers, auth, format)?;

        let response = send_with_response_header_capture(
            self.client.post(url.clone()).headers(headers).body(payload),
            client_headers,
        )
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
                provider: "google".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: None,
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
        let url = self
            .config
            .endpoint
            .join("models")
            .expect("join models path");
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "google".to_string(),
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
    use crate::catalog::ModelFlavor;
    use crate::providers::Provider;
    use lingua::serde_json::json;
    use reqwest::header::HeaderValue;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(endpoint: Url) -> GoogleProvider {
        GoogleProvider::new(GoogleConfig {
            endpoint,
            timeout: None,
        })
        .expect("provider")
    }

    fn spec(model: &str) -> ModelSpec {
        ModelSpec {
            model: model.to_string(),
            format: ProviderFormat::Google,
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

    fn api_key_auth() -> AuthConfig {
        AuthConfig::ApiKey {
            key: "test-key".into(),
            header: Some("x-goog-api-key".into()),
            prefix: None,
        }
    }

    #[test]
    fn chat_completions_url_joins_openai_path() {
        let provider =
            provider(Url::parse("https://generativelanguage.googleapis.com/v1beta/").unwrap());
        let url = provider
            .url_for_format("gemini-2.5-flash", false, ProviderFormat::ChatCompletions)
            .expect("url");

        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn chat_completions_url_does_not_duplicate_openai_suffix() {
        let provider = provider(
            Url::parse("https://generativelanguage.googleapis.com/v1beta/openai/").unwrap(),
        );
        let url = provider
            .url_for_format("gemini-2.5-flash", false, ProviderFormat::ChatCompletions)
            .expect("url");

        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn native_streaming_url_keeps_generate_content_sse_path() {
        let provider =
            provider(Url::parse("https://generativelanguage.googleapis.com/v1beta/").unwrap());
        let url = provider
            .url_for_format("gemini-2.5-flash", true, ProviderFormat::Google)
            .expect("url");

        assert_eq!(
            url.as_str(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn chat_completions_api_key_auth_uses_bearer() {
        let provider =
            provider(Url::parse("https://generativelanguage.googleapis.com/v1beta/").unwrap());
        let mut headers = HeaderMap::new();
        provider
            .apply_auth_headers(
                &mut headers,
                &api_key_auth(),
                ProviderFormat::ChatCompletions,
            )
            .expect("headers");

        assert_eq!(
            headers.get("authorization"),
            Some(&HeaderValue::from_static("Bearer test-key"))
        );
        assert!(headers.get("x-goog-api-key").is_none());
    }

    #[test]
    fn native_api_key_auth_preserves_configured_header() {
        let provider =
            provider(Url::parse("https://generativelanguage.googleapis.com/v1beta/").unwrap());
        let mut headers = HeaderMap::new();
        provider
            .apply_auth_headers(&mut headers, &api_key_auth(), ProviderFormat::Google)
            .expect("headers");

        assert_eq!(
            headers.get("x-goog-api-key"),
            Some(&HeaderValue::from_static("test-key"))
        );
        assert!(headers.get("authorization").is_none());
    }

    #[tokio::test]
    async fn complete_chat_completions_posts_to_openai_compatible_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/openai/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(body_json(json!({
                "model": "gemini-2.5-flash",
                "messages": [{"role": "user", "content": "Ping"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test",
                "choices": []
            })))
            .mount(&server)
            .await;

        let provider = provider(Url::parse(&format!("{}/v1beta/", server.uri())).unwrap());
        let payload = Bytes::from(
            serde_json::to_vec(&json!({
                "model": "gemini-2.5-flash",
                "messages": [{"role": "user", "content": "Ping"}]
            }))
            .expect("json"),
        );

        let response = provider
            .complete(
                payload,
                &api_key_auth(),
                &spec("gemini-2.5-flash"),
                ProviderFormat::ChatCompletions,
                &ClientHeaders::default(),
            )
            .await
            .expect("complete");
        let parsed: serde_json::Value = serde_json::from_slice(&response).expect("json");

        assert_eq!(
            parsed.get("id").and_then(serde_json::Value::as_str),
            Some("chatcmpl-test")
        );
    }

    #[tokio::test]
    async fn complete_stream_chat_completions_posts_to_same_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/openai/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(body_json(json!({
                "model": "gemini-2.5-flash",
                "stream": true,
                "messages": [{"role": "user", "content": "Ping"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_string("data: {\"choices\":[]}\n\n"))
            .mount(&server)
            .await;

        let provider = provider(Url::parse(&format!("{}/v1beta/", server.uri())).unwrap());
        let payload = Bytes::from(
            serde_json::to_vec(&json!({
                "model": "gemini-2.5-flash",
                "stream": true,
                "messages": [{"role": "user", "content": "Ping"}]
            }))
            .expect("json"),
        );

        let _stream = provider
            .complete_stream(
                payload,
                &api_key_auth(),
                &spec("gemini-2.5-flash"),
                ProviderFormat::ChatCompletions,
                &ClientHeaders::default(),
            )
            .await
            .expect("stream");
    }
}
