use std::time::Duration;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{sse_stream, RawResponseStream};
use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::{self, Value};
use lingua::ProviderFormat;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

pub const ANTHROPIC_VERSION: &str = "anthropic-version";
pub const DEFAULT_ANTHROPIC_VERSION_VALUE: &str = "2023-06-01";
pub const ANTHROPIC_PROMPT_CACHE_HEADER: &str = "x-anthropic-prompt-cache";
const ANTHROPIC_BETA: &str = "anthropic-beta";
const STRUCTURED_OUTPUTS_BETA: &str = "structured-outputs-2025-11-13";

fn prompt_cache_header_enabled(headers: &HeaderMap) -> bool {
    headers
        .get(ANTHROPIC_PROMPT_CACHE_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

fn strip_prompt_cache_header(headers: &mut HeaderMap) {
    headers.remove(ANTHROPIC_PROMPT_CACHE_HEADER);
}

fn apply_prompt_cache_header(
    payload: Bytes,
    format: ProviderFormat,
    headers: &HeaderMap,
) -> Result<Bytes> {
    // Non-Anthropic request formats do not have a native way to ask for
    // Anthropic cache_control, and caching changes Anthropic billing behavior.
    // Use this router-only header as the explicit opt-in bridge instead of
    // enabling prompt caching for every transform to Anthropic.
    if format != ProviderFormat::Anthropic || !prompt_cache_header_enabled(headers) {
        return Ok(payload);
    }

    let mut value: Value = serde_json::from_slice(&payload)?;
    let Some(object) = value.as_object_mut() else {
        return Err(Error::InvalidRequest(
            "Anthropic prompt cache header requires a JSON object request body".to_string(),
        ));
    };

    if object.contains_key("cache_control") {
        return Ok(payload);
    }

    object.insert(
        "cache_control".to_string(),
        serde_json::json!({ "type": "ephemeral" }),
    );
    Ok(Bytes::from(serde_json::to_vec(&value)?))
}

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub endpoint: Url,
    pub version: String,
    pub timeout: Option<Duration>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.anthropic.com/v1/")
                .expect("valid Anthropic endpoint"),
            version: "2023-06-01".to_string(),
            timeout: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_prompt_cache(value: &str) -> HeaderMap {
        let mut client_headers = ClientHeaders::new();
        client_headers.insert_if_allowed(ANTHROPIC_PROMPT_CACHE_HEADER, value);
        client_headers.to_json_headers()
    }

    #[test]
    fn prompt_cache_header_adds_top_level_cache_control() {
        let payload = Bytes::from_static(
            br#"{"model":"claude-sonnet-4-5","max_tokens":1024,"messages":[{"role":"user","content":"Hi"}]}"#,
        );
        let updated = apply_prompt_cache_header(
            payload,
            ProviderFormat::Anthropic,
            &headers_with_prompt_cache("true"),
        )
        .expect("cache header should apply");
        let parsed: Value = serde_json::from_slice(&updated).expect("valid json");

        assert_eq!(
            parsed
                .get("cache_control")
                .and_then(|cache_control| cache_control.get("type"))
                .and_then(Value::as_str),
            Some("ephemeral")
        );
    }

    #[test]
    fn prompt_cache_header_preserves_existing_cache_control() {
        let payload = Bytes::from_static(
            br#"{"model":"claude-sonnet-4-5","max_tokens":1024,"cache_control":{"type":"ephemeral","ttl":"1h"},"messages":[{"role":"user","content":"Hi"}]}"#,
        );
        let updated = apply_prompt_cache_header(
            payload,
            ProviderFormat::Anthropic,
            &headers_with_prompt_cache("true"),
        )
        .expect("cache header should apply");
        let parsed: Value = serde_json::from_slice(&updated).expect("valid json");

        assert_eq!(
            parsed
                .get("cache_control")
                .and_then(|cache_control| cache_control.get("ttl"))
                .and_then(Value::as_str),
            Some("1h")
        );
    }

    #[test]
    fn prompt_cache_header_does_not_apply_to_chat_completions_format() {
        let payload = Bytes::from_static(
            br#"{"model":"claude-sonnet-4-5","messages":[{"role":"user","content":"Hi"}]}"#,
        );
        let updated = apply_prompt_cache_header(
            payload,
            ProviderFormat::ChatCompletions,
            &headers_with_prompt_cache("true"),
        )
        .expect("cache header should be ignored");
        let parsed: Value = serde_json::from_slice(&updated).expect("valid json");

        assert!(parsed.get("cache_control").is_none());
    }

    #[test]
    fn prompt_cache_control_header_is_stripped_before_upstream() {
        let mut headers = headers_with_prompt_cache("true");
        assert!(headers.contains_key(ANTHROPIC_PROMPT_CACHE_HEADER));

        strip_prompt_cache_header(&mut headers);

        assert!(!headers.contains_key(ANTHROPIC_PROMPT_CACHE_HEADER));
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: ClientWithMiddleware,
    config: AnthropicConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: AnthropicConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    /// Create an Anthropic provider from configuration parameters.
    ///
    /// Extracts Anthropic-specific options from metadata:
    /// - `version`: Anthropic API version (defaults to "2023-06-01")
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, lingua::serde_json::Value>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        use lingua::serde_json::Value;
        let mut config = AnthropicConfig::default();

        if let Some(ep) = endpoint {
            config.endpoint = ep.clone();
        }
        if let Some(t) = timeout {
            config.timeout = Some(t);
        }
        if let Some(version) = metadata.get("version").and_then(Value::as_str) {
            config.version = version.to_string();
        }

        Self::new_with_client_settings(config, client_settings.unwrap_or_default())
    }

    fn messages_url(&self) -> Url {
        self.config
            .endpoint
            .join("messages")
            .expect("join messages path")
    }

    fn chat_completions_url(&self) -> Url {
        self.config
            .endpoint
            .join("chat/completions")
            .expect("join chat/completions path")
    }

    fn build_headers(&self, client_headers: &ClientHeaders) -> HeaderMap {
        let mut headers = client_headers.to_json_headers();

        headers.insert(
            ANTHROPIC_VERSION,
            HeaderValue::from_str(&self.config.version).expect("version header"),
        );

        // Respect caller override: only set default if missing.
        if !headers.contains_key(ANTHROPIC_BETA) {
            headers.insert(
                ANTHROPIC_BETA,
                HeaderValue::from_static(STRUCTURED_OUTPUTS_BETA),
            );
        }

        headers
    }
}

#[async_trait]
impl crate::providers::Provider for AnthropicProvider {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Anthropic, ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let mut base_headers = self.build_headers(client_headers);
        let payload = apply_prompt_cache_header(payload, format, &base_headers)?;
        strip_prompt_cache_header(&mut base_headers);

        let (url, headers) = if format == ProviderFormat::ChatCompletions {
            let mut h = client_headers.to_json_headers();
            strip_prompt_cache_header(&mut h);
            let key = auth.api_key().ok_or_else(|| {
                Error::Auth("Anthropic /chat/completions requires an API key".to_string())
            })?;
            h.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|e| Error::Auth(format!("invalid auth header: {e}")))?,
            );
            (self.chat_completions_url(), h)
        } else {
            auth.apply_headers(&mut base_headers)?;
            (self.messages_url(), base_headers)
        };

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
                provider: "anthropic".to_string(),
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

        // Router should have already added stream options to payload
        let mut base_headers = self.build_headers(client_headers);
        let payload = apply_prompt_cache_header(payload, format, &base_headers)?;
        strip_prompt_cache_header(&mut base_headers);

        let (url, headers) = if format == ProviderFormat::ChatCompletions {
            let mut h = client_headers.to_json_headers();
            strip_prompt_cache_header(&mut h);
            let key = auth.api_key().ok_or_else(|| {
                Error::Auth("Anthropic /chat/completions requires an API key".to_string())
            })?;
            h.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|e| Error::Auth(format!("invalid auth header: {e}")))?,
            );
            (self.chat_completions_url(), h)
        } else {
            auth.apply_headers(&mut base_headers)?;
            (self.messages_url(), base_headers)
        };

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
                provider: "anthropic".to_string(),
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
        let mut headers = self.build_headers(&ClientHeaders::default());
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "anthropic".to_string(),
                source: anyhow::anyhow!("status {}", response.status()),
                retry_after: None,
                http: None,
            })
        }
    }
}
