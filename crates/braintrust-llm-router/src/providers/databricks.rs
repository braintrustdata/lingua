use std::{collections::BTreeMap, time::Duration};

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{sse_stream, RawResponseStream};
use async_trait::async_trait;
use bytes::Bytes;
use lingua::providers::openai::params::OpenAIResponsesParams;
use lingua::ProviderFormat;
use reqwest::header::HeaderMap;
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;

#[derive(Debug, Clone)]
pub struct DatabricksConfig {
    pub api_base: Url,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct DatabricksProvider {
    client: ClientWithMiddleware,
    config: DatabricksConfig,
}

impl DatabricksProvider {
    pub fn new(config: DatabricksConfig) -> Result<Self> {
        Self::new_with_client_settings(config, ClientSettings::default())
    }

    pub fn new_with_client_settings(
        config: DatabricksConfig,
        mut settings: ClientSettings,
    ) -> Result<Self> {
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    pub fn from_config(
        api_base: Option<&Url>,
        timeout: Option<Duration>,
        client_settings: Option<ClientSettings>,
    ) -> Result<Self> {
        let api_base = api_base
            .cloned()
            .ok_or_else(|| Error::InvalidRequest("databricks provider requires api_base".into()))?;
        Self::new_with_client_settings(
            DatabricksConfig { api_base, timeout },
            client_settings.unwrap_or_default(),
        )
    }

    // This does not support Databrick's new AI gateway URL format yet, only
    // their model serving endpoints.
    fn serving_url(&self, model: &str, format: ProviderFormat) -> Result<Url> {
        let mut url = self.config.api_base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::InvalidRequest("endpoint must be absolute".into()))?;
            segments.pop_if_empty();
            segments.push("serving-endpoints");
            if format == ProviderFormat::Responses {
                segments.push("responses");
            } else {
                segments.push(model);
                segments.push("invocations");
            }
        }
        Ok(url)
    }

    fn prepare_payload(payload: Bytes, format: ProviderFormat) -> Result<Bytes> {
        if format != ProviderFormat::Responses {
            return Ok(payload);
        }
        let params: OpenAIResponsesParams = serde_json::from_slice(&payload)?;
        if params.store != Some(false) {
            return Ok(payload);
        }
        let mut fields: BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_slice(&payload)?;
        fields.remove("store");
        Ok(Bytes::from(serde_json::to_vec(&fields)?))
    }
}

#[async_trait]
impl crate::providers::Provider for DatabricksProvider {
    fn id(&self) -> &'static str {
        "databricks"
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
        let url = self.serving_url(&spec.model, format)?;
        let payload = Self::prepare_payload(payload, format)?;

        let mut headers = client_headers.to_json_headers();
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
                provider: "databricks".to_string(),
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

        let url = self.serving_url(&spec.model, format)?;
        let payload = Self::prepare_payload(payload, format)?;

        let mut headers = client_headers.to_json_headers();
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
                provider: "databricks".to_string(),
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
        let url = provider
            .serving_url("my-model", ProviderFormat::ChatCompletions)
            .unwrap();
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
        let url = provider
            .serving_url("llama-3-1-8b", ProviderFormat::ChatCompletions)
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://adb-123.azuredatabricks.net/serving-endpoints/llama-3-1-8b/invocations"
        );
    }

    #[test]
    fn from_config_requires_api_base() {
        let err = DatabricksProvider::from_config(None, None, None).unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn responses_preserve_tools_and_use_responses_endpoint() {
        use crate::providers::Provider;
        use futures::StreamExt;
        use wiremock::matchers::{body_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let provider = DatabricksProvider::new(DatabricksConfig {
            api_base: Url::parse(&server.uri()).unwrap(),
            timeout: None,
        })
        .unwrap();
        let auth = crate::auth::api_key_auth("test-token");
        let spec: ModelSpec = serde_json::from_str(
            r#"{"model":"databricks-gpt-6-1-sol","format":"openai","flavor":"chat"}"#,
        )
        .unwrap();
        assert!(provider
            .provider_formats()
            .contains(&ProviderFormat::Responses));

        for (streaming, store) in [
            (false, None),
            (true, None),
            (false, Some(false)),
            (true, Some(false)),
            (true, Some(true)),
        ] {
            let mut expected = serde_json::json!({
                "model": "databricks-gpt-6-1-sol",
                "input": "hello",
                "stream": streaming,
                "tools": [{"type": "custom", "name": "exec", "format": {"type": "text"}}],
                "future_parameter": {"nested": [null, {"store": true}]},
            });
            let mut request = expected.clone();
            if let Some(store) = store {
                request["store"] = store.into();
                if store {
                    expected["store"] = true.into();
                }
            }
            let payload = Bytes::from(serde_json::to_vec(&request).unwrap());
            let _mock = Mock::given(method("POST"))
                .and(path("/serving-endpoints/responses"))
                .and(header("authorization", "Bearer test-token"))
                .and(body_json(expected))
                .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
                .expect(1)
                .mount_as_scoped(&server)
                .await;
            if streaming {
                let mut stream = provider
                    .complete_stream(
                        payload,
                        &auth,
                        &spec,
                        ProviderFormat::Responses,
                        &ClientHeaders::default(),
                    )
                    .await
                    .unwrap();
                while let Some(chunk) = stream.next().await {
                    chunk.unwrap();
                }
            } else {
                provider
                    .complete(
                        payload,
                        &auth,
                        &spec,
                        ProviderFormat::Responses,
                        &ClientHeaders::default(),
                    )
                    .await
                    .unwrap();
            }
        }
    }
}
