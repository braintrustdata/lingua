use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use lingua::processing::{adapter_for_format, adapters};
use lingua::universal::message::{Message, UserContent, UserContentPart};
use lingua::util::media::MediaBlock;
use lingua::{ProviderFormat, TransformError};

use crate::catalog::ModelSpec;
use crate::error::{Error, Result};

use super::body_model::rewrite_body_model_if_required;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemoteMediaPolicy {
    pub(crate) inline_images: bool,
    pub(crate) inline_files: bool,
    pub(crate) max_bytes: usize,
}

impl RemoteMediaPolicy {
    pub(crate) const GOOGLE: Self = Self {
        inline_images: true,
        inline_files: true,
        max_bytes: 20 * 1024 * 1024,
    };

    pub(crate) const BEDROCK: Self = Self {
        inline_images: true,
        inline_files: false,
        max_bytes: 5 * 1024 * 1024,
    };

    pub(crate) fn for_format(format: ProviderFormat) -> Option<Self> {
        match format {
            ProviderFormat::Google => Some(Self::GOOGLE),
            ProviderFormat::BedrockAnthropic | ProviderFormat::Converse => Some(Self::BEDROCK),
            ProviderFormat::Anthropic
            | ProviderFormat::ChatCompletions
            | ProviderFormat::Mistral
            | ProviderFormat::Responses
            | ProviderFormat::VertexAnthropic
            | ProviderFormat::Unknown => None,
        }
    }
}

type FetchMediaFuture<'a> = Pin<Box<dyn Future<Output = Result<MediaBlock>> + Send + 'a>>;

#[derive(Debug)]
pub(crate) struct PreparedRemoteMediaRequest {
    pub(crate) bytes: Bytes,
    pub(crate) requires_json_response: bool,
}

pub(crate) async fn prepare_request_with_remote_media(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    policy: RemoteMediaPolicy,
) -> Result<PreparedRemoteMediaRequest> {
    prepare_request_with_remote_media_and_fetch(body, spec, format, policy, |url| {
        Box::pin(fetch_remote_media_as_base64(url, policy.max_bytes))
    })
    .await
}

async fn fetch_remote_media_as_base64(url: &str, max_bytes: usize) -> Result<MediaBlock> {
    lingua::util::media::convert_media_to_base64(url, None, Some(max_bytes))
        .await
        .map_err(|e| Error::InvalidRequest(format!("failed to fetch media URL {url}: {e}")))
}

pub(crate) async fn prepare_request_with_remote_media_and_fetch<F>(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    policy: RemoteMediaPolicy,
    fetch: F,
) -> Result<PreparedRemoteMediaRequest>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    let parsed = lingua::parse_json_body(body)?;
    let payload = parsed.value;
    let body = parsed.bytes;
    let source_adapter = adapters()
        .iter()
        .map(|adapter| adapter.as_ref())
        .find(|adapter| adapter.detect_request(&payload))
        .ok_or(TransformError::UnableToDetectRequestFormat)?;
    let requires_json_response = source_adapter
        .request_requires_json_response(&payload)
        .map_err(Error::from)?;

    if source_adapter.format() == format {
        return Ok(PreparedRemoteMediaRequest {
            bytes: rewrite_body_model_if_required(body, format, &spec.model),
            requires_json_response,
        });
    }

    let mut request = source_adapter.request_to_universal(payload)?;
    inline_remote_media_with_fetch(&mut request, policy, fetch).await?;
    request.model = Some(spec.model.clone());

    let target_adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    target_adapter.apply_defaults(&mut request);
    let prepared = target_adapter.request_from_universal(&request)?;
    let bytes = lingua::serde_json::to_vec(&prepared)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)?;

    Ok(PreparedRemoteMediaRequest {
        bytes,
        requires_json_response,
    })
}

pub(crate) async fn inline_remote_media_with_fetch<F>(
    request: &mut lingua::UniversalRequest,
    policy: RemoteMediaPolicy,
    mut fetch: F,
) -> Result<()>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    for message in &mut request.messages {
        let content = match message {
            Message::System { content }
            | Message::Developer { content }
            | Message::User { content } => content,
            Message::Assistant { .. } | Message::Tool { .. } | Message::AdditionalTools { .. } => {
                continue;
            }
        };
        let UserContent::Array(parts) = content else {
            continue;
        };

        for part in parts {
            match part {
                UserContentPart::Image {
                    image, media_type, ..
                } if policy.inline_images => {
                    let Some(url) = image.as_str().map(str::to_string) else {
                        continue;
                    };
                    if !is_remote_media_url(&url) {
                        continue;
                    }
                    let media_block = fetch(&url).await?;
                    *image = lingua::serde_json::Value::String(media_block.data);
                    *media_type = Some(media_block.media_type);
                }
                UserContentPart::File {
                    data, media_type, ..
                } if policy.inline_files => {
                    let Some(url) = data.as_str().map(str::to_string) else {
                        continue;
                    };
                    if !is_remote_media_url(&url) {
                        continue;
                    }
                    let media_block = fetch(&url).await?;
                    *data = lingua::serde_json::Value::String(media_block.data);
                    *media_type = media_block.media_type;
                }
                UserContentPart::Image { .. }
                | UserContentPart::File { .. }
                | UserContentPart::Text(_) => {}
            }
        }
    }

    Ok(())
}

fn is_remote_media_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ModelFlavor;
    use lingua::serde_json::json;

    fn google_spec(model: &str) -> ModelSpec {
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

    #[test]
    fn policies_select_supported_target_formats() {
        assert_eq!(
            RemoteMediaPolicy::for_format(ProviderFormat::Google),
            Some(RemoteMediaPolicy::GOOGLE)
        );
        assert_eq!(
            RemoteMediaPolicy::for_format(ProviderFormat::BedrockAnthropic),
            Some(RemoteMediaPolicy::BEDROCK)
        );
        assert_eq!(
            RemoteMediaPolicy::for_format(ProviderFormat::Converse),
            Some(RemoteMediaPolicy::BEDROCK)
        );
        assert_eq!(
            RemoteMediaPolicy::for_format(ProviderFormat::Anthropic),
            None
        );
    }

    #[tokio::test]
    async fn google_policy_inlines_remote_responses_file() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gemini-3.1-pro-preview",
                "input": [{
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "Read this PDF."},
                        {
                            "type": "input_file",
                            "filename": "sample.pdf",
                            "file_url": "https://example.com/sample.pdf"
                        }
                    ]
                }]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &google_spec("gemini-3.1-pro-preview"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            |url| {
                assert_eq!(url, "https://example.com/sample.pdf");
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "application/pdf".into(),
                        data: "cGRm".into(),
                    })
                })
            },
        )
        .await
        .expect("prepare request");

        let request: lingua::providers::google::GenerateContentRequest =
            lingua::serde_json::from_slice(&prepared.bytes).expect("google request");
        let contents = request.contents.as_ref().expect("contents");
        let parts = contents[0].parts.as_ref().expect("parts");

        assert_eq!(parts[1].file_data, None);
        let inline_data = parts[1].inline_data.as_ref().expect("inline data");
        assert_eq!(inline_data.data.as_deref(), Some("cGRm"));
        assert_eq!(inline_data.mime_type.as_deref(), Some("application/pdf"));
    }

    #[tokio::test]
    async fn google_policy_does_not_fetch_data_or_gcs_urls() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gemini-3.1-pro-preview",
                "input": [{
                    "role": "user",
                    "content": [
                        {
                            "type": "input_file",
                            "filename": "inline.pdf",
                            "file_data": "data:application/pdf;base64,cGRm"
                        },
                        {
                            "type": "input_file",
                            "filename": "stored.pdf",
                            "file_url": "gs://bucket/stored.pdf"
                        }
                    ]
                }]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &google_spec("gemini-3.1-pro-preview"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            |_url| Box::pin(async { panic!("should not fetch a non-HTTP media URL") }),
        )
        .await
        .expect("prepare request");

        let request: lingua::providers::google::GenerateContentRequest =
            lingua::serde_json::from_slice(&prepared.bytes).expect("google request");
        let contents = request.contents.as_ref().expect("contents");
        let parts = contents[0].parts.as_ref().expect("parts");

        assert!(parts[0]
            .inline_data
            .as_ref()
            .is_some_and(|inline_data| inline_data.data.is_some()));
        assert_eq!(
            parts[1]
                .file_data
                .as_ref()
                .and_then(|file_data| file_data.file_uri.as_deref()),
            Some("gs://bucket/stored.pdf")
        );
    }
}
