use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::RawResponseStream;
use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::Value;
use lingua::universal::decision::TypeSafeDecisionOptions;
use lingua::ProviderFormat;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

const DANGER_ZONE_IMAGES: HeaderName = HeaderName::from_static("typesafe-danger-zone-images");
const DANGER_ZONE_LONG_CONTEXT: HeaderName =
    HeaderName::from_static("typesafe-danger-zone-long-context");

pub(crate) fn typesafe_client_headers<'a>(
    params: &TypeSafeDecisionOptions,
    headers: &'a ClientHeaders,
) -> Cow<'a, ClientHeaders> {
    let mut headers = Cow::Borrowed(headers);
    for (name, enabled) in [
        (DANGER_ZONE_IMAGES, params.images),
        (DANGER_ZONE_LONG_CONTEXT, params.long_context),
    ] {
        if let Some(enabled) = enabled {
            headers
                .to_mut()
                .insert_if_allowed(name.as_str(), if enabled { "true" } else { "false" });
        }
    }
    headers
}

#[derive(Debug, Clone)]
pub struct TypeSafeConfig {
    pub endpoint: Url,
    pub timeout: Option<Duration>,
    /// Send `TypeSafe-Danger-Zone-Images: true`, which System One requires for
    /// image assets and box questions.
    pub danger_zone_images: bool,
    /// Send `TypeSafe-Danger-Zone-Long-Context: true` to allow long-context requests.
    pub danger_zone_long_context: bool,
}

impl Default for TypeSafeConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.typesafe.ai").expect("valid default URL"),
            timeout: None,
            danger_zone_images: false,
            danger_zone_long_context: false,
        }
    }
}

/// A boolean secret metadata flag, `false` when unset.
fn metadata_flag(metadata: &HashMap<String, Value>, key: &str) -> Result<bool> {
    match metadata.get(key) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(other) => Err(Error::InvalidRequest(format!(
            "typesafe metadata {key} must be a boolean, got {other}"
        ))),
    }
}

/// TypeSafe System One API provider for decision models such as Jev.
#[derive(Debug, Clone)]
pub struct TypeSafeProvider {
    client: ClientWithMiddleware,
    config: TypeSafeConfig,
}

impl TypeSafeProvider {
    pub fn new(config: TypeSafeConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: TypeSafeConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &HashMap<String, Value>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        let mut config = TypeSafeConfig::default();
        if let Some(endpoint) = endpoint {
            config.endpoint = endpoint.clone();
        }
        if let Some(timeout) = timeout {
            config.timeout = Some(timeout);
        }
        config.danger_zone_images = metadata_flag(metadata, "danger_zone_images")?;
        config.danger_zone_long_context = metadata_flag(metadata, "danger_zone_long_context")?;
        Self::new_with_client_settings(config, client_settings.unwrap_or_default())
    }

    /// An API URL under `/v1`, accepting an endpoint configured with or without `/v1`.
    fn api_url(&self, path: &str) -> Result<Url> {
        let mut url = self.config.endpoint.clone();
        let has_version = url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            == Some("v1");
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            if !has_version {
                segments.push("v1");
            }
            segments.push(path);
        }
        Ok(url)
    }
}

#[async_trait]
impl crate::providers::Provider for TypeSafeProvider {
    fn id(&self) -> &'static str {
        "typesafe"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::TypeSafe]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        _spec: &ModelSpec,
        format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        if format != ProviderFormat::TypeSafe {
            return Err(Error::NoProvider(format));
        }
        let url = self.api_url("systemone")?;
        let mut headers = client_headers.to_json_headers();
        auth.apply_headers(&mut headers)?;
        for (header, enabled) in [
            (DANGER_ZONE_IMAGES, self.config.danger_zone_images),
            (
                DANGER_ZONE_LONG_CONTEXT,
                self.config.danger_zone_long_context,
            ),
        ] {
            if headers.get(&header).is_some_and(|value| value == "false") {
                headers.remove(&header);
            } else if enabled && !headers.contains_key(&header) {
                headers.insert(header, HeaderValue::from_static("true"));
            }
        }

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
            let text = response
                .text()
                .await
                .unwrap_or_else(|error| format!("failed to read response body: {error}"));
            return Err(Error::Provider {
                provider: "typesafe".to_string(),
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
        _payload: Bytes,
        _auth: &AuthConfig,
        _spec: &ModelSpec,
        _format: ProviderFormat,
        _client_headers: &ClientHeaders,
    ) -> Result<RawResponseStream> {
        Err(Error::InvalidRequest(
            "TypeSafe System One does not support streaming".into(),
        ))
    }

    async fn health_check(&self, auth: &AuthConfig) -> Result<()> {
        let url = self.api_url("models")?;
        let mut headers = ClientHeaders::default().to_json_headers();
        auth.apply_headers(&mut headers)?;
        let response = self.client.get(url).headers(headers).send().await?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(Error::Provider {
            provider: "typesafe".to_string(),
            source: anyhow::anyhow!("status {}", response.status()),
            retry_after: None,
            http: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_accepts_origin_or_versioned_endpoint() {
        for (endpoint, expected) in [
            (
                "https://api.typesafe.ai",
                "https://api.typesafe.ai/v1/systemone",
            ),
            (
                "https://api.typesafe.ai/",
                "https://api.typesafe.ai/v1/systemone",
            ),
            (
                "https://api.typesafe.ai/v1",
                "https://api.typesafe.ai/v1/systemone",
            ),
            (
                "https://proxy.example.com/typesafe/v1/",
                "https://proxy.example.com/typesafe/v1/systemone",
            ),
        ] {
            let provider = TypeSafeProvider::from_config(
                Some(&Url::parse(endpoint).unwrap()),
                None,
                &HashMap::new(),
                None,
            )
            .unwrap();
            assert_eq!(provider.api_url("systemone").unwrap().as_str(), expected);
        }
    }
}
