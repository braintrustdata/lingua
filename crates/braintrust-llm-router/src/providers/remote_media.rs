use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use lingua::processing::{adapter_for_format, adapters, normalize_universal_request_for_target};
use lingua::providers::openai::generated::{
    ChatCompletionRequestMessageContent, CreateChatCompletionRequestClass, PurpleType,
};
use lingua::universal::message::{AudioFormat, Message, UserContent, UserContentPart};
use lingua::util::media::MediaBlock;
use lingua::{ProviderFormat, TransformError};
use serde::Deserialize;

use crate::catalog::ModelSpec;
use crate::error::{Error, Result};

use super::body_model::rewrite_body_model_if_required;

const MAX_REMOTE_MEDIA_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemoteMediaPolicy {
    pub(crate) inline_images: bool,
    pub(crate) inline_files: bool,
    pub(crate) inline_audio: bool,
}

impl RemoteMediaPolicy {
    pub(crate) const GOOGLE: Self = Self {
        inline_images: true,
        inline_files: true,
        inline_audio: true,
    };

    pub(crate) const BEDROCK: Self = Self {
        inline_images: true,
        inline_files: false,
        inline_audio: false,
    };

    pub(crate) const OPENAI: Self = Self {
        inline_images: false,
        inline_files: false,
        inline_audio: true,
    };

    pub(crate) fn for_format(format: ProviderFormat) -> Option<Self> {
        match format {
            ProviderFormat::Google => Some(Self::GOOGLE),
            ProviderFormat::BedrockAnthropic | ProviderFormat::Converse => Some(Self::BEDROCK),
            ProviderFormat::ChatCompletions => Some(Self::OPENAI),
            ProviderFormat::Anthropic
            | ProviderFormat::Mistral
            | ProviderFormat::Responses
            | ProviderFormat::VertexAnthropic
            | ProviderFormat::Unknown => None,
        }
    }
}

type FetchMediaFuture<'a> = Pin<Box<dyn Future<Output = Result<MediaBlock>> + Send + 'a>>;

#[derive(Deserialize)]
struct ChatAudioRequestView {
    #[serde(default)]
    messages: Vec<ChatAudioMessageView>,
}

#[derive(Deserialize)]
struct ChatAudioMessageView {
    content: Option<ChatAudioContentView>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChatAudioContentView {
    Parts(Vec<ChatAudioPartView>),
    Text(String),
}

#[derive(Deserialize)]
struct ChatAudioPartView {
    input_audio: Option<AudioDataView>,
}

#[derive(Deserialize)]
struct GoogleAudioRequestView {
    #[serde(default)]
    contents: Vec<GoogleAudioContentView>,
}

#[derive(Deserialize)]
struct GoogleAudioContentView {
    #[serde(default)]
    parts: Vec<GoogleAudioPartView>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAudioPartView {
    inline_data: Option<AudioDataView>,
}

#[derive(Deserialize)]
struct AudioDataView {
    data: String,
}

#[derive(Debug)]
pub(crate) struct PreparedRemoteMediaRequest {
    pub(crate) bytes: Bytes,
    #[cfg(test)]
    pub(crate) detected_format: Option<ProviderFormat>,
    #[cfg(test)]
    pub(crate) requires_json_response: bool,
    #[cfg(test)]
    pub(crate) lingua_passthrough: bool,
}

pub(crate) async fn prepare_request_with_remote_media(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    policy: RemoteMediaPolicy,
    rewrite_body_model: bool,
) -> Result<PreparedRemoteMediaRequest> {
    prepare_request_with_remote_media_and_fetch_with_model_rewrite(
        body,
        spec,
        format,
        policy,
        rewrite_body_model,
        |url| Box::pin(fetch_remote_media_as_base64(url)),
    )
    .await
}

async fn fetch_remote_media_as_base64(url: &str) -> Result<MediaBlock> {
    lingua::util::media::convert_media_to_base64(url, None, Some(MAX_REMOTE_MEDIA_BYTES))
        .await
        .map_err(|e| Error::InvalidRequest(format!("failed to fetch media URL {url}: {e}")))
}

#[cfg(test)]
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
    prepare_request_with_remote_media_and_fetch_with_model_rewrite(
        body, spec, format, policy, true, fetch,
    )
    .await
}

async fn prepare_request_with_remote_media_and_fetch_with_model_rewrite<F>(
    body: Bytes,
    spec: &ModelSpec,
    format: ProviderFormat,
    policy: RemoteMediaPolicy,
    rewrite_body_model: bool,
    mut fetch: F,
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
    #[cfg(test)]
    let requires_json_response = source_adapter
        .request_requires_json_response(&payload)
        .map_err(Error::from)?;
    let has_remote_audio = request_has_remote_audio(&body, source_adapter.format())?;

    if source_adapter.format() == format && !has_remote_audio {
        return Ok(PreparedRemoteMediaRequest {
            bytes: if rewrite_body_model {
                rewrite_body_model_if_required(body, format, &spec.model)
            } else {
                body
            },
            #[cfg(test)]
            detected_format: None,
            #[cfg(test)]
            requires_json_response,
            #[cfg(test)]
            lingua_passthrough: true,
        });
    }

    if source_adapter.format() == ProviderFormat::ChatCompletions && has_remote_audio {
        let prepared =
            inline_remote_chat_audio_with_fetch(body, spec, rewrite_body_model, &mut fetch).await?;
        if format == ProviderFormat::ChatCompletions {
            return Ok(prepared);
        }
        let parsed = lingua::parse_json_body(prepared.bytes)?;
        let payload = parsed.value;
        let source_adapter = adapters()
            .iter()
            .map(|adapter| adapter.as_ref())
            .find(|adapter| adapter.detect_request(&payload))
            .ok_or(TransformError::UnableToDetectRequestFormat)?;
        let mut request = source_adapter.request_to_universal(payload)?;
        normalize_universal_request_for_target(&mut request, format);
        inline_remote_media_with_fetch(&mut request, policy, fetch).await?;
        let target_adapter =
            adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
        target_adapter.apply_defaults(&mut request);
        let bytes = lingua::serde_json::to_vec(&target_adapter.request_from_universal(&request)?)
            .map(Bytes::from)
            .map_err(Error::LinguaJson)?;
        return Ok(PreparedRemoteMediaRequest {
            bytes,
            #[cfg(test)]
            detected_format: Some(source_adapter.format()),
            #[cfg(test)]
            requires_json_response,
            #[cfg(test)]
            lingua_passthrough: false,
        });
    }

    let mut request = source_adapter.request_to_universal(payload)?;

    if rewrite_body_model {
        request.model = Some(spec.model.clone());
    }
    normalize_universal_request_for_target(&mut request, format);
    inline_remote_media_with_fetch(&mut request, policy, fetch).await?;

    let target_adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    target_adapter.apply_defaults(&mut request);
    let prepared = target_adapter.request_from_universal(&request)?;
    let bytes = lingua::serde_json::to_vec(&prepared)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)?;

    Ok(PreparedRemoteMediaRequest {
        bytes,
        #[cfg(test)]
        detected_format: Some(source_adapter.format()),
        #[cfg(test)]
        requires_json_response,
        #[cfg(test)]
        lingua_passthrough: false,
    })
}

pub(crate) fn request_has_remote_audio_in_payload(body: &[u8]) -> Result<bool> {
    let parsed = lingua::parse_json_body(Bytes::copy_from_slice(body))?;
    let source_adapter = adapters()
        .iter()
        .map(|adapter| adapter.as_ref())
        .find(|adapter| adapter.detect_request(&parsed.value))
        .ok_or(TransformError::UnableToDetectRequestFormat)?;
    request_has_remote_audio(body, source_adapter.format())
}

fn request_has_remote_audio(body: &[u8], format: ProviderFormat) -> Result<bool> {
    match format {
        ProviderFormat::ChatCompletions => {
            let request: ChatAudioRequestView = lingua::serde_json::from_slice(body)?;
            Ok(request
                .messages
                .into_iter()
                .any(|message| match message.content {
                    Some(ChatAudioContentView::Parts(parts)) => parts
                        .iter()
                        .filter_map(|part| part.input_audio.as_ref())
                        .any(|audio| is_remote_media_url(&audio.data)),
                    Some(ChatAudioContentView::Text(text)) => {
                        let _ = text;
                        false
                    }
                    None => false,
                }))
        }
        ProviderFormat::Google => {
            let request: GoogleAudioRequestView = lingua::serde_json::from_slice(body)?;
            Ok(request.contents.into_iter().any(|content| {
                content
                    .parts
                    .into_iter()
                    .filter_map(|part| part.inline_data)
                    .any(|audio| is_remote_media_url(&audio.data))
            }))
        }
        ProviderFormat::BedrockAnthropic | ProviderFormat::Converse => Ok(false),
        _ => Ok(false),
    }
}

async fn inline_remote_chat_audio_with_fetch<F>(
    body: Bytes,
    spec: &ModelSpec,
    rewrite_body_model: bool,
    fetch: &mut F,
) -> Result<PreparedRemoteMediaRequest>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    let mut request: CreateChatCompletionRequestClass = lingua::serde_json::from_slice(&body)?;
    for message in &mut request.messages {
        let Some(
            ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(
                parts,
            ),
        ) = message.content.as_mut()
        else {
            continue;
        };
        for part in parts {
            if part.content_part_type != PurpleType::InputAudio {
                continue;
            }
            let Some(audio) = part.input_audio.as_mut() else {
                continue;
            };
            if !is_remote_media_url(&audio.data) {
                continue;
            }
            let media_block = fetch(&audio.data).await?;
            let format = match audio.format {
                lingua::providers::openai::generated::InputAudioFormat::Mp3 => AudioFormat::Mp3,
                lingua::providers::openai::generated::InputAudioFormat::Wav => AudioFormat::Wav,
            };
            if !audio_format_matches_media_type(&format, &media_block.media_type) {
                return Err(Error::InvalidRequest(format!(
                    "remote audio MIME type {} does not match input_audio format {format:?}",
                    media_block.media_type
                )));
            }
            audio.data = media_block.data;
        }
    }
    if rewrite_body_model {
        request.model = spec.model.clone();
    }
    let bytes = lingua::serde_json::to_vec(&request)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)?;
    Ok(PreparedRemoteMediaRequest {
        bytes,
        #[cfg(test)]
        detected_format: None,
        #[cfg(test)]
        requires_json_response: false,
        #[cfg(test)]
        lingua_passthrough: false,
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
                UserContentPart::Audio { data, format } if policy.inline_audio => {
                    let url = data.clone();
                    if !is_remote_media_url(&url) {
                        continue;
                    }
                    let media_block = fetch(&url).await?;
                    if !audio_format_matches_media_type(format, &media_block.media_type) {
                        return Err(Error::InvalidRequest(format!(
                            "remote audio MIME type {} does not match input_audio format {format:?}",
                            media_block.media_type
                        )));
                    }
                    *data = media_block.data;
                }
                UserContentPart::Image { .. }
                | UserContentPart::File { .. }
                | UserContentPart::Audio { .. }
                | UserContentPart::Text(_) => {}
            }
        }
    }

    Ok(())
}

fn audio_format_matches_media_type(format: &AudioFormat, media_type: &str) -> bool {
    let media_type = media_type
        .split(';')
        .next()
        .unwrap_or(media_type)
        .trim()
        .to_ascii_lowercase();
    match format {
        AudioFormat::Mp3 => matches!(media_type.as_str(), "audio/mpeg" | "audio/mp3"),
        AudioFormat::Wav => matches!(media_type.as_str(), "audio/wav" | "audio/x-wav"),
    }
}

fn is_remote_media_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ModelFlavor;
    use lingua::serde_json::json;

    fn spec(model: &str, format: ProviderFormat) -> ModelSpec {
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

    fn google_spec(model: &str) -> ModelSpec {
        spec(model, ProviderFormat::Google)
    }

    fn openai_spec(model: &str) -> ModelSpec {
        spec(model, ProviderFormat::ChatCompletions)
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
            RemoteMediaPolicy::for_format(ProviderFormat::ChatCompletions),
            Some(RemoteMediaPolicy::OPENAI)
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

        assert_eq!(prepared.detected_format, Some(ProviderFormat::Responses));
        assert!(!prepared.lingua_passthrough);

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
    async fn google_policy_inlines_remote_chat_audio() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gpt-audio-1.5",
                "messages": [{
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Judge this call."},
                        {
                            "type": "input_audio",
                            "input_audio": {
                                "data": "https://example.com/call.wav",
                                "format": "wav"
                            }
                        }
                    ]
                }]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &google_spec("gemini-3.5-flash"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            |url| {
                assert_eq!(url, "https://example.com/call.wav");
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "audio/wav".into(),
                        data: "cmlm".into(),
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
        let inline_data = parts[1].inline_data.as_ref().expect("inline data");

        assert_eq!(inline_data.mime_type.as_deref(), Some("audio/wav"));
        assert_eq!(inline_data.data.as_deref(), Some("cmlm"));
    }

    #[tokio::test]
    async fn openai_policy_inlines_remote_chat_audio() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gpt-audio-1.5",
                "messages": [{
                    "role": "user",
                    "name": "recording-owner",
                    "content": [{
                        "type": "input_audio",
                        "input_audio": {
                            "data": "https://example.com/call.wav",
                            "format": "wav"
                        }
                    }]
                }]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &openai_spec("gpt-audio-1.5"),
            ProviderFormat::ChatCompletions,
            RemoteMediaPolicy::OPENAI,
            |url| {
                assert_eq!(url, "https://example.com/call.wav");
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "audio/wav".into(),
                        data: "cmlm".into(),
                    })
                })
            },
        )
        .await
        .expect("prepare request");

        let request: lingua::serde_json::Value =
            lingua::serde_json::from_slice(&prepared.bytes).expect("OpenAI request");
        assert_eq!(
            request["messages"][0]["content"][0]["input_audio"]["data"],
            "cmlm"
        );
        assert_eq!(request["messages"][0]["name"], "recording-owner");
    }

    #[tokio::test]
    async fn remote_chat_audio_is_inlined_before_responses_upgrade() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gpt-5.4-mini",
                "messages": [{
                    "role": "user",
                    "content": [{
                        "type": "input_audio",
                        "input_audio": {
                            "data": "https://example.com/call.wav",
                            "format": "wav"
                        }
                    }]
                }],
                "reasoning_effort": "medium",
                "tools": [{
                    "type": "function",
                    "function": {"name": "get_weather", "parameters": {"type": "object"}}
                }]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &openai_spec("gpt-5.4-mini"),
            ProviderFormat::ChatCompletions,
            RemoteMediaPolicy::OPENAI,
            |url| {
                assert_eq!(url, "https://example.com/call.wav");
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "audio/wav".into(),
                        data: "cmlm".into(),
                    })
                })
            },
        )
        .await
        .expect("audio is inlined before transform");

        let transformed = lingua::transform_request(
            prepared.bytes,
            ProviderFormat::ChatCompletions,
            Some("gpt-5.4-mini"),
        )
        .expect("request upgrades to Responses");
        let lingua::TransformResult::Transformed {
            bytes,
            actual_target_format,
            ..
        } = transformed.result
        else {
            panic!("reasoning plus tools must transform to Responses");
        };
        assert_eq!(actual_target_format, ProviderFormat::Responses);
        let response: lingua::serde_json::Value =
            lingua::serde_json::from_slice(&bytes).expect("Responses request JSON");
        assert_eq!(
            response["input"][0]["content"][0]["input_audio"]["data"],
            "cmlm"
        );
    }

    #[tokio::test]
    async fn openai_policy_passthrough_does_not_convert_native_refusal_without_remote_audio() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-4o","messages":[{"role":"assistant","content":[{"type":"refusal","refusal":"I can't help with that."}]}]}"#,
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body.clone(),
            &openai_spec("gpt-4o"),
            ProviderFormat::ChatCompletions,
            RemoteMediaPolicy::OPENAI,
            |_url| panic!("a request without remote audio must not fetch media"),
        )
        .await
        .expect("native request passes through without universal conversion");

        assert_eq!(prepared.bytes, body);
        assert!(prepared.lingua_passthrough);
    }

    #[tokio::test]
    async fn remote_audio_rejects_mismatched_content_type() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "gpt-audio-1.5",
                "messages": [{
                    "role": "user",
                    "content": [{
                        "type": "input_audio",
                        "input_audio": {
                            "data": "https://example.com/call.wav",
                            "format": "wav"
                        }
                    }]
                }]
            }))
            .expect("json"),
        );

        let error = prepare_request_with_remote_media_and_fetch(
            body,
            &google_spec("gemini-3.5-flash"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            |_url| {
                Box::pin(async {
                    Ok(MediaBlock {
                        media_type: "audio/mpeg".into(),
                        data: "cmlm".into(),
                    })
                })
            },
        )
        .await
        .expect_err("mismatched audio content type should fail");

        assert!(error
            .to_string()
            .contains("does not match input_audio format"));
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

    #[tokio::test]
    async fn google_policy_strips_claude_code_attribution() {
        let body = Bytes::from(
            lingua::serde_json::to_vec(&json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1024,
                "system": [{
                    "type": "text",
                    "text": "x-anthropic-billing-header: cc_version=1.2.3; cch=changing-hash;"
                }],
                "messages": [{"role": "user", "content": "Hello."}]
            }))
            .expect("json"),
        );

        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &google_spec("gemini-3.1-pro-preview"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            |_url| Box::pin(async { panic!("request has no remote media") }),
        )
        .await
        .expect("prepare request");

        let text = String::from_utf8(prepared.bytes.to_vec()).expect("UTF-8 JSON");
        assert!(!text.contains("x-anthropic-billing-header"));
        assert!(!text.contains("changing-hash"));
    }
}
