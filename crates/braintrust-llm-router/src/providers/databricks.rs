use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode, Url};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{default_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{single_bytes_stream, sse_stream, RawResponseStream};
use lingua::ProviderFormat;

#[derive(Debug, Clone)]
pub struct DatabricksConfig {
    pub api_base: Url,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct DatabricksProvider {
    client: Client,
    config: DatabricksConfig,
}

impl DatabricksProvider {
    pub fn new(config: DatabricksConfig) -> Result<Self> {
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

    pub fn from_config(api_base: Option<&Url>, timeout: Option<Duration>) -> Result<Self> {
        let api_base = api_base
            .cloned()
            .ok_or_else(|| Error::InvalidRequest("databricks provider requires api_base".into()))?;
        Self::new(DatabricksConfig { api_base, timeout })
    }

    /// Normalize a payload for Databricks serving endpoints:
    /// - Strip `stream_options` (unsupported)
    /// - Convert `max_completion_tokens` → `max_tokens` (unsupported)
    fn normalize_payload(&self, payload: Bytes) -> Bytes {
        let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&payload) else {
            return payload;
        };
        let Some(obj) = value.as_object_mut() else {
            return payload;
        };
        let mut changed = false;
        if obj.remove("stream_options").is_some() {
            changed = true;
        }
        if let Some(max_completion_tokens) = obj.remove("max_completion_tokens") {
            obj.entry("max_tokens").or_insert(max_completion_tokens);
            changed = true;
        }
        if changed {
            return Bytes::from(serde_json::to_vec(&value).unwrap_or_else(|_| payload.to_vec()));
        }
        payload
    }

    // This does not support Databrick's new AI gateway URL format yet, only
    // their model serving endpoints.
    fn serving_url(&self, model: &str) -> Result<Url> {
        let mut url = self.config.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("serving-endpoints");
            segments.push(model);
            segments.push("invocations");
        }
        Ok(url)
    }
}

fn extract_retry_after(status: StatusCode) -> Option<Duration> {
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        Some(Duration::from_secs(2))
    } else {
        None
    }
}

#[async_trait]
impl crate::providers::Provider for DatabricksProvider {
    fn id(&self) -> &'static str {
        "databricks"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::ChatCompletions]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let payload = self.normalize_payload(payload);
        let url = self.serving_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "databricks",
            http_url = %url,
            "sending request to Databricks"
        );

        let mut headers = client_headers.to_json_headers();
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
            llm_provider = "databricks",
            http_status_code = status_code,
            "received response from Databricks"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "databricks".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
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
        let payload = self.normalize_payload(payload);

        if !spec.supports_streaming {
            let response = self
                .complete(payload, auth, spec, format, client_headers)
                .await?;
            return Ok(single_bytes_stream(response));
        }

        let url = self.serving_url(&spec.model)?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            target: "bt.router.provider.http",
            llm_provider = "databricks",
            http_url = %url,
            llm_streaming = true,
            "sending streaming request to Databricks"
        );

        let mut headers = client_headers.to_json_headers();
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
            llm_provider = "databricks",
            http_status_code = status_code,
            llm_streaming = true,
            "received streaming response from Databricks"
        );

        if !response.status().is_success() {
            let status = response.status();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(Error::Provider {
                provider: "databricks".to_string(),
                source: anyhow::anyhow!("HTTP {status}: {text}"),
                retry_after: extract_retry_after(status),
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
        let mut url = self.config.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("serving-endpoints");
        }
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "databricks".to_string(),
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

    #[test]
    fn serving_url_appends_model_and_invocations() {
        let config = DatabricksConfig {
            api_base: Url::parse("https://adb-123.azuredatabricks.net").unwrap(),
            timeout: None,
        };
        let provider = DatabricksProvider::new(config).unwrap();
        let url = provider.serving_url("my-model").unwrap();
        assert_eq!(
            url.as_str(),
            "https://adb-123.azuredatabricks.net/serving-endpoints/my-model/invocations"
        );
    }

    #[test]
    fn serving_url_with_trailing_slash_in_base() {
        let config = DatabricksConfig {
            api_base: Url::parse("https://adb-123.azuredatabricks.net/").unwrap(),
            timeout: None,
        };
        let provider = DatabricksProvider::new(config).unwrap();
        let url = provider.serving_url("llama-3-1-8b").unwrap();
        assert_eq!(
            url.as_str(),
            "https://adb-123.azuredatabricks.net/serving-endpoints/llama-3-1-8b/invocations"
        );
    }

    #[test]
    fn from_config_requires_api_base() {
        let err = DatabricksProvider::from_config(None, None).unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    fn make_provider() -> DatabricksProvider {
        DatabricksProvider::new(DatabricksConfig {
            api_base: Url::parse("https://adb-123.azuredatabricks.net").unwrap(),
            timeout: None,
        })
        .unwrap()
    }

    #[test]
    fn normalize_payload_strips_stream_options() {
        let provider = make_provider();
        let payload = Bytes::from(
            r#"{"model":"m","messages":[],"stream":true,"stream_options":{"include_usage":true}}"#,
        );
        let out = provider.normalize_payload(payload);
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(value.get("stream_options").is_none());
        assert!(value.get("stream").is_some());
    }

    #[test]
    fn normalize_payload_converts_max_completion_tokens() {
        let provider = make_provider();
        let payload = Bytes::from(r#"{"model":"m","messages":[],"max_completion_tokens":50}"#);
        let out = provider.normalize_payload(payload);
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(value.get("max_completion_tokens").is_none());
        assert_eq!(value.get("max_tokens").and_then(|v| v.as_i64()), Some(50));
    }

    #[test]
    fn normalize_payload_preserves_max_tokens_when_both_present() {
        let provider = make_provider();
        let payload = Bytes::from(
            r#"{"model":"m","messages":[],"max_tokens":100,"max_completion_tokens":50}"#,
        );
        let out = provider.normalize_payload(payload);
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert!(value.get("max_completion_tokens").is_none());
        assert_eq!(value.get("max_tokens").and_then(|v| v.as_i64()), Some(100));
    }

    #[test]
    fn normalize_payload_no_op_when_nothing_to_strip() {
        let provider = make_provider();
        let raw = r#"{"model":"m","messages":[],"stream":true}"#;
        let out = provider.normalize_payload(Bytes::from(raw));
        assert_eq!(std::str::from_utf8(&out).unwrap(), raw);
    }
}
