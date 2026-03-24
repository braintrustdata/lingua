use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use lingua::serde_json::Value;
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::{StatusCode, Url};
use reqwest_middleware::ClientWithMiddleware;

use crate::auth::AuthConfig;
use crate::catalog::ModelSpec;
use crate::client::{build_middleware_client, ClientSettings};
use crate::error::{Error, Result, UpstreamHttpError};
use crate::providers::ClientHeaders;
use crate::streaming::{sse_stream, RawResponseStream};
use lingua::ProviderFormat;

const DEFAULT_LOCATION: &str = "us-central1";
const ANTHROPIC_DEFAULT_LOCATION: &str = "us-east5";

#[derive(Debug, Clone)]
pub struct VertexConfig {
    pub api_base: Option<String>,
    pub project: String,
    pub location: String,
    pub timeout: Option<Duration>,
}

impl Default for VertexConfig {
    fn default() -> Self {
        Self {
            api_base: None,
            project: String::new(),
            location: DEFAULT_LOCATION.to_string(),
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VertexProvider {
    client: ClientWithMiddleware,
    config: VertexConfig,
}

#[derive(serde::Deserialize)]
struct VertexModelExtra {
    #[serde(default)]
    locations: Vec<String>,
}

impl VertexProvider {
    pub fn new(config: VertexConfig) -> Result<Self> {
        let mut settings = ClientSettings::default();
        if let Some(timeout) = config.timeout {
            settings.request_timeout = timeout;
        }
        let client = build_middleware_client(&settings)?;
        Ok(Self { client, config })
    }

    /// Create a Vertex provider from configuration parameters.
    ///
    /// Extracts Vertex-specific options from metadata:
    /// - `project`: GCP project ID (required)
    /// - `location`: GCP region (defaults to us-central1)
    /// - `api_base`: Custom API base URL (optional)
    pub fn from_config(
        endpoint: Option<&Url>,
        timeout: Option<Duration>,
        metadata: &std::collections::HashMap<String, Value>,
    ) -> Result<Self> {
        let project = metadata
            .get("project")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidRequest("Vertex requires project".into()))?
            .to_string();

        let location = metadata
            .get("location")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_LOCATION.to_string());

        let api_base_from_metadata = metadata
            .get("api_base")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let api_base = api_base_from_metadata.or_else(|| {
            endpoint
                .filter(|url| {
                    // Ignore the global aiplatform.googleapis.com endpoint — Vertex requires
                    // the location-prefixed hostname (e.g. us-east5-aiplatform.googleapis.com)
                    // for operations like rawPredict.
                    url.host_str() != Some("aiplatform.googleapis.com")
                })
                .map(|url| url.as_str().to_string())
        });

        let config = VertexConfig {
            api_base,
            project,
            location,
            timeout,
        };

        Self::new(config)
    }

    fn resolve_location(&self, spec: &ModelSpec, default_location: &str) -> String {
        // Precedence: model spec locations > secret metadata location > default.
        if let Ok(extra) = ::serde_json::from_value::<VertexModelExtra>(
            ::serde_json::Value::Object(spec.extra.clone()),
        ) {
            if !extra.locations.is_empty() {
                let idx = rand::thread_rng().gen_range(0..extra.locations.len());
                return extra.locations[idx].clone();
            }
        }
        if !self.config.location.is_empty() {
            return self.config.location.clone();
        }
        default_location.to_string()
    }

    fn base_url(&self, location: &str) -> Result<Url> {
        if let Some(ref api_base) = self.config.api_base {
            return Url::parse(api_base)
                .map_err(|e| Error::InvalidRequest(format!("Invalid Vertex api_base URL: {e}")));
        }
        let url_str = if location == "global" {
            "https://aiplatform.googleapis.com/".to_string()
        } else {
            format!("https://{location}-aiplatform.googleapis.com/")
        };
        Url::parse(&url_str)
            .map_err(|e| Error::InvalidRequest(format!("Invalid Vertex endpoint URL: {e}")))
    }

    fn default_location_for_mode(mode: &VertexMode) -> &'static str {
        match mode {
            VertexMode::Anthropic { .. } => ANTHROPIC_DEFAULT_LOCATION,
            _ => DEFAULT_LOCATION,
        }
    }

    fn determine_mode(&self, model: &str) -> VertexMode {
        if model.starts_with("publishers/meta") {
            VertexMode::OpenApi {
                api_version: "v1beta1",
            }
        } else if model.starts_with("publishers/qwen") {
            VertexMode::OpenApi { api_version: "v1" }
        } else if model.starts_with("publishers/anthropic/") {
            VertexMode::Anthropic {
                model_path: model.to_string(),
            }
        } else if model.starts_with("publishers/") {
            VertexMode::Generative {
                model_path: model.to_string(),
            }
        } else {
            VertexMode::Generative {
                model_path: format!("publishers/google/models/{}", model),
            }
        }
    }

    fn endpoint_for_mode(
        &self,
        mode: &VertexMode,
        base_url: &Url,
        location: &str,
        stream: bool,
    ) -> Result<Url> {
        match mode {
            VertexMode::Generative { model_path } => {
                let method = if stream {
                    "streamGenerateContent"
                } else {
                    "generateContent"
                };
                let mut url = base_url.clone();
                let path = format!(
                    "v1/projects/{}/locations/{}/{}:{}",
                    self.config.project, location, model_path, method
                );
                url.set_path(&path);
                if stream {
                    url.query_pairs_mut().append_pair("alt", "sse");
                }
                Ok(url)
            }
            VertexMode::Anthropic { model_path } => {
                let method = if stream {
                    "streamRawPredict"
                } else {
                    "rawPredict"
                };
                let mut url = base_url.clone();
                let path = format!(
                    "v1/projects/{}/locations/{}/{}:{}",
                    self.config.project, location, model_path, method
                );
                url.set_path(&path);
                Ok(url)
            }
            VertexMode::OpenApi { api_version } => {
                let mut url = base_url.clone();
                let path = format!(
                    "{}/projects/{}/locations/{}/endpoints/openapi/chat/completions",
                    api_version, self.config.project, location
                );
                url.set_path(&path);
                Ok(url)
            }
        }
    }
}

#[async_trait]
impl crate::providers::Provider for VertexProvider {
    fn id(&self) -> &'static str {
        "vertex"
    }

    fn provider_formats(&self) -> Vec<ProviderFormat> {
        vec![ProviderFormat::Google, ProviderFormat::VertexAnthropic]
    }

    async fn complete(
        &self,
        payload: Bytes,
        auth: &AuthConfig,
        spec: &ModelSpec,
        _format: ProviderFormat,
        client_headers: &ClientHeaders,
    ) -> Result<Bytes> {
        let mode = self.determine_mode(&spec.model);
        let location = self.resolve_location(spec, Self::default_location_for_mode(&mode));
        let base_url = self.base_url(&location)?;
        let url = self.endpoint_for_mode(&mode, &base_url, &location, false)?;

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
                provider: "vertex".to_string(),
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
            return self
                .complete_stream_via_complete(payload, auth, spec, format, client_headers)
                .await;
        }

        let mode = self.determine_mode(&spec.model);
        let location = self.resolve_location(spec, Self::default_location_for_mode(&mode));
        let base_url = self.base_url(&location)?;
        let url = self.endpoint_for_mode(&mode, &base_url, &location, true)?;

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
                provider: "vertex".to_string(),
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
        let mode = self.determine_mode("");
        let location = &self.config.location;
        let base_url = self.base_url(location)?;
        let url = self.endpoint_for_mode(&mode, &base_url, location, false)?;
        let mut headers = HeaderMap::new();
        auth.apply_headers(&mut headers)?;

        let response = self.client.get(url).headers(headers).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::Provider {
                provider: "vertex".to_string(),
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

enum VertexMode {
    Generative { model_path: String },
    Anthropic { model_path: String },
    OpenApi { api_version: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> VertexProvider {
        let config = VertexConfig {
            api_base: None,
            project: "test-project".into(),
            location: "us-central1".into(),
            timeout: None,
        };
        VertexProvider::new(config).unwrap()
    }

    fn base_url(location: &str) -> Url {
        if location == "global" {
            Url::parse("https://aiplatform.googleapis.com/").unwrap()
        } else {
            Url::parse(&format!("https://{location}-aiplatform.googleapis.com/")).unwrap()
        }
    }

    #[test]
    fn selects_generative_mode_by_default() {
        let provider = provider();
        match provider.determine_mode("gemini-pro") {
            VertexMode::Generative { model_path } => {
                assert_eq!(model_path, "publishers/google/models/gemini-pro");
            }
            _ => panic!("expected generative mode"),
        }
    }

    #[test]
    fn selects_openapi_mode_for_meta_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("publishers/meta/models/llama"),
            VertexMode::OpenApi {
                api_version: "v1beta1"
            }
        ));
    }

    #[test]
    fn selects_openapi_mode_for_qwen_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("publishers/qwen/models/qwen2"),
            VertexMode::OpenApi { api_version: "v1" }
        ));
    }

    #[test]
    fn selects_anthropic_mode_for_anthropic_models() {
        let provider = provider();
        assert!(matches!(
            provider.determine_mode("publishers/anthropic/models/claude-haiku-4-5"),
            VertexMode::Anthropic { .. }
        ));
    }

    #[test]
    fn builds_generative_endpoint() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/google/models/gemini-pro");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", false)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-pro:generateContent"
        );
    }

    #[test]
    fn builds_anthropic_rawpredict_endpoint() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/anthropic/models/claude-haiku-4-5");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", false)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/anthropic/models/claude-haiku-4-5:rawPredict"
        );
    }

    #[test]
    fn builds_anthropic_stream_rawpredict_endpoint() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/anthropic/models/claude-haiku-4-5");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", true)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/anthropic/models/claude-haiku-4-5:streamRawPredict"
        );
    }

    #[test]
    fn builds_anthropic_endpoint_with_version_suffix() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/anthropic/models/claude-3-5-haiku@20241022");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", false)
            .expect("url");
        // @ must NOT be percent-encoded — Vertex requires the literal @ in the model path
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/anthropic/models/claude-3-5-haiku@20241022:rawPredict"
        );
    }

    #[test]
    fn builds_openapi_endpoint_for_meta() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/meta/models/llama");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", false)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1beta1/projects/test-project/locations/us-central1/endpoints/openapi/chat/completions"
        );
    }

    #[test]
    fn builds_openapi_endpoint_for_qwen() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/qwen/models/qwen2");
        let bu = base_url("us-central1");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "us-central1", false)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/endpoints/openapi/chat/completions"
        );
    }

    #[test]
    fn base_url_returns_global_endpoint_for_global_location() {
        let provider = provider();
        let url = provider.base_url("global").expect("url");
        assert_eq!(url.as_str(), "https://aiplatform.googleapis.com/");
    }

    #[test]
    fn base_url_returns_regional_endpoint() {
        let provider = provider();
        let url = provider.base_url("us-east5").expect("url");
        assert_eq!(url.as_str(), "https://us-east5-aiplatform.googleapis.com/");
    }

    #[test]
    fn base_url_uses_api_base_override() {
        let config = VertexConfig {
            api_base: Some("https://custom.example.com".into()),
            project: "test-project".into(),
            location: "us-central1".into(),
            timeout: None,
        };
        let provider = VertexProvider::new(config).unwrap();
        let url = provider.base_url("us-east5").expect("url");
        assert_eq!(url.as_str(), "https://custom.example.com/");
    }

    #[test]
    fn builds_generative_endpoint_with_global_location() {
        let provider = provider();
        let mode = provider.determine_mode("publishers/google/models/gemini-3.1-pro-preview");
        let bu = base_url("global");
        let url = provider
            .endpoint_for_mode(&mode, &bu, "global", false)
            .expect("url");
        assert_eq!(
            url.as_str(),
            "https://aiplatform.googleapis.com/v1/projects/test-project/locations/global/publishers/google/models/gemini-3.1-pro-preview:generateContent"
        );
    }

    #[test]
    fn resolve_location_uses_spec_locations() {
        let provider = provider();
        let spec = ModelSpec {
            model: "publishers/google/models/gemini-3.1-pro-preview".to_string(),
            format: ProviderFormat::Google,
            flavor: crate::catalog::ModelFlavor::Chat,
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
            extra: {
                let mut map = ::serde_json::Map::new();
                map.insert(
                    "locations".into(),
                    ::serde_json::Value::Array(vec![::serde_json::Value::String(
                        "europe-west4".into(),
                    )]),
                );
                map
            },
            available_providers: vec![],
        };
        assert_eq!(
            provider.resolve_location(&spec, DEFAULT_LOCATION),
            "europe-west4"
        );
    }

    #[test]
    fn resolve_location_falls_back_to_config() {
        let provider = provider();
        let spec = ModelSpec {
            model: "publishers/google/models/gemini-pro".to_string(),
            format: ProviderFormat::Google,
            flavor: crate::catalog::ModelFlavor::Chat,
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
            extra: ::serde_json::Map::new(),
            available_providers: vec![],
        };
        assert_eq!(
            provider.resolve_location(&spec, DEFAULT_LOCATION),
            "us-central1"
        );
    }

    #[test]
    fn from_config_ignores_global_api_base() {
        let global_endpoint = Url::parse("https://aiplatform.googleapis.com/").unwrap();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("project".into(), Value::String("my-project".into()));
        metadata.insert("location".into(), Value::String("us-east5".into()));

        let provider =
            VertexProvider::from_config(Some(&global_endpoint), None, &metadata).unwrap();
        assert!(provider.config.api_base.is_none());
        assert_eq!(provider.config.location, "us-east5");
    }

    #[test]
    fn from_config_preserves_custom_endpoint() {
        let custom_endpoint = Url::parse("https://my-proxy.example.com/").unwrap();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("project".into(), Value::String("my-project".into()));
        metadata.insert("location".into(), Value::String("us-east5".into()));

        let provider =
            VertexProvider::from_config(Some(&custom_endpoint), None, &metadata).unwrap();
        assert_eq!(
            provider.config.api_base.as_deref(),
            Some("https://my-proxy.example.com/")
        );
    }

    #[test]
    fn from_config_reads_api_base_from_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("project".into(), Value::String("my-project".into()));
        metadata.insert(
            "api_base".into(),
            Value::String("https://custom-proxy.example.com".into()),
        );

        let provider = VertexProvider::from_config(None, None, &metadata).unwrap();
        assert_eq!(
            provider.config.api_base.as_deref(),
            Some("https://custom-proxy.example.com")
        );
    }

    #[test]
    fn from_config_ignores_empty_api_base() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("project".into(), Value::String("my-project".into()));
        metadata.insert("api_base".into(), Value::String("".into()));

        let provider = VertexProvider::from_config(None, None, &metadata).unwrap();
        assert!(provider.config.api_base.is_none());
    }

    #[test]
    fn from_config_ignores_empty_location() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("project".into(), Value::String("my-project".into()));
        metadata.insert("location".into(), Value::String("  ".into()));

        let provider = VertexProvider::from_config(None, None, &metadata).unwrap();
        assert_eq!(provider.config.location, "us-central1");
    }
}
