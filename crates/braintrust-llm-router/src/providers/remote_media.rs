use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use lingua::processing::{adapter_for_format, adapters, normalize_universal_request_for_target};
use lingua::providers::google::generated::Part;
use lingua::providers::openai::generated::{
    ContentInputItemContentList, HilariousType, InputAudio, InputAudioFormat, PurpleContentPart,
    PurpleType,
};
use lingua::universal::message::{AudioFormat, Message, UserContent, UserContentPart};
use lingua::util::media::MediaBlock;
use lingua::{ProviderFormat, TransformError};

use crate::catalog::ModelSpec;
use crate::error::{Error, Result};

use super::body_model::rewrite_body_model_if_required;
use super::json_selection::{
    select,
    Segment::{Each, Key},
};

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

// An internal fetch operation, not a provider wire format. Paths are obtained while
// deserializing generated content parts; only the selected data string is replaced.
struct RemoteAudio {
    pointer: String,
    url: String,
    mime_type: String,
}

fn openai_audio(pointer: String, audio: InputAudio) -> Option<RemoteAudio> {
    is_remote_media_url(&audio.data).then(|| RemoteAudio {
        pointer: format!("{pointer}/input_audio/data"),
        url: audio.data,
        mime_type: match audio.format {
            InputAudioFormat::Mp3 => "audio/mpeg",
            InputAudioFormat::Wav => "audio/wav",
        }
        .into(),
    })
}

fn remote_audio(body: &[u8], format: ProviderFormat) -> Result<Vec<RemoteAudio>> {
    let mut audio = Vec::new();
    match format {
        ProviderFormat::ChatCompletions => {
            for (pointer, part) in
                select::<PurpleContentPart>(body, &[Key("messages"), Each, Key("content"), Each])?
            {
                if part.content_part_type == PurpleType::InputAudio {
                    if let Some(remote) = part
                        .input_audio
                        .and_then(|input| openai_audio(pointer, input))
                    {
                        audio.push(remote);
                    }
                }
            }
        }
        ProviderFormat::Responses => {
            for (pointer, part) in select::<ContentInputItemContentList>(
                body,
                &[Key("input"), Each, Key("content"), Each],
            )? {
                if part.input_content_type == HilariousType::InputAudio {
                    if let Some(remote) = part
                        .input_audio
                        .and_then(|input| openai_audio(pointer, input))
                    {
                        audio.push(remote);
                    }
                }
            }
        }
        ProviderFormat::Google => {
            for (pointer, part) in
                select::<Part>(body, &[Key("contents"), Each, Key("parts"), Each])?
            {
                if let Some(blob) = part.inline_data {
                    if let Some(url) = blob.data.filter(|data| is_remote_media_url(data)) {
                        let mime_type = blob.mime_type.ok_or_else(|| {
                            Error::InvalidRequest("remote inlineData requires mimeType".into())
                        })?;
                        audio.push(RemoteAudio {
                            pointer: format!("{pointer}/inlineData/data"),
                            url,
                            mime_type,
                        });
                    }
                }
            }
        }
        _ => {}
    }
    Ok(audio)
}

pub(crate) fn reject_remote_responses_audio(body: &[u8]) -> Result<()> {
    if !remote_audio(body, ProviderFormat::Responses)?.is_empty() {
        return Err(Error::InvalidRequest(
            "Responses input_audio.data requires base64 audio; remote audio URLs are not supported"
                .into(),
        ));
    }
    Ok(())
}

async fn inline_native_audio<F>(
    body: Bytes,
    audio: Vec<RemoteAudio>,
    fetch: &mut F,
) -> Result<Bytes>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    let mut payload: lingua::serde_json::Value = lingua::serde_json::from_slice(&body)?;
    for remote in audio {
        let media = fetch(&remote.url).await?;
        if normalized_media_type(&remote.mime_type) != normalized_media_type(&media.media_type) {
            return Err(Error::InvalidRequest(format!(
                "remote audio MIME type {} does not match declared MIME type {}",
                media.media_type, remote.mime_type
            )));
        }
        // Mechanical write to a path already validated by a generated type. Never
        // reserialize that type: it does not retain every provider extension.
        let data = payload.pointer_mut(&remote.pointer).ok_or_else(|| {
            Error::InvalidRequest("selected audio data is missing from the original payload".into())
        })?;
        *data = lingua::serde_json::Value::String(media.data);
    }
    Ok(Bytes::from(lingua::serde_json::to_vec(&payload)?))
}

#[derive(Debug)]
pub(crate) struct PreparedRemoteMediaRequest {
    pub(crate) bytes: Bytes,
    pub(crate) detected_format: Option<ProviderFormat>,
    pub(crate) requires_json_response: bool,
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
    let requires_json_response = source_adapter
        .request_requires_json_response(&payload)
        .map_err(Error::from)?;
    let audio = remote_audio(&body, source_adapter.format())?;
    let has_remote_audio = !audio.is_empty();

    if has_remote_audio && !policy.inline_audio {
        return Err(Error::InvalidRequest(format!(
            "remote audio input is unsupported for {format:?}"
        )));
    }

    if source_adapter.format() == format && !has_remote_audio {
        return Ok(PreparedRemoteMediaRequest {
            bytes: if rewrite_body_model {
                rewrite_body_model_if_required(body, format, &spec.model)
            } else {
                body
            },
            detected_format: None,
            requires_json_response,
            lingua_passthrough: true,
        });
    }

    let payload = if has_remote_audio
        && matches!(
            source_adapter.format(),
            ProviderFormat::ChatCompletions | ProviderFormat::Google
        ) {
        let bytes = inline_native_audio(body, audio, &mut fetch).await?;
        if source_adapter.format() == format {
            return Ok(PreparedRemoteMediaRequest {
                bytes: if rewrite_body_model {
                    rewrite_body_model_if_required(bytes, format, &spec.model)
                } else {
                    bytes
                },
                detected_format: None,
                requires_json_response,
                lingua_passthrough: false,
            });
        }
        lingua::parse_json_body(bytes)?.value
    } else {
        payload
    };

    let mut request = source_adapter.request_to_universal(payload)?;
    if rewrite_body_model {
        request.model = Some(spec.model.clone());
    }
    let bytes = prepare_universal_remote_media_request(request, format, policy, &mut fetch).await?;

    Ok(PreparedRemoteMediaRequest {
        bytes,
        detected_format: Some(source_adapter.format()),
        requires_json_response,
        lingua_passthrough: false,
    })
}

async fn prepare_universal_remote_media_request<F>(
    mut request: lingua::UniversalRequest,
    format: ProviderFormat,
    policy: RemoteMediaPolicy,
    fetch: &mut F,
) -> Result<Bytes>
where
    F: for<'a> FnMut(&'a str) -> FetchMediaFuture<'a>,
{
    normalize_universal_request_for_target(&mut request, format);
    inline_remote_media_with_fetch(&mut request, policy, fetch).await?;
    let target_adapter =
        adapter_for_format(format).ok_or(TransformError::UnsupportedTargetFormat(format))?;
    target_adapter.apply_defaults(&mut request);
    lingua::serde_json::to_vec(&target_adapter.request_from_universal(&request)?)
        .map(Bytes::from)
        .map_err(Error::LinguaJson)
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
    let expected = match format {
        AudioFormat::Mp3 => "audio/mpeg",
        AudioFormat::Wav => "audio/wav",
    };
    normalized_media_type(media_type) == expected
}

fn normalized_media_type(media_type: &str) -> String {
    let media_type = media_type
        .split(';')
        .next()
        .unwrap_or(media_type)
        .trim()
        .to_ascii_lowercase();
    match media_type.as_str() {
        "audio/mp3" => "audio/mpeg".into(),
        "audio/x-wav" => "audio/wav".into(),
        _ => media_type,
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

    fn wav_fetch(
        expected_url: &'static str,
    ) -> impl for<'a> FnMut(&'a str) -> FetchMediaFuture<'a> {
        move |url| {
            assert_eq!(url, expected_url);
            Box::pin(async {
                Ok(MediaBlock {
                    media_type: "audio/wav".into(),
                    data: "cmlm".into(),
                })
            })
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
            wav_fetch("https://example.com/call.wav"),
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
                "provider": {"order": ["anthropic"]},
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
            wav_fetch("https://example.com/call.wav"),
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
        assert_eq!(request["provider"]["order"][0], "anthropic");
    }

    #[tokio::test]
    async fn bedrock_remote_audio_is_rejected_without_fetching() {
        let body = Bytes::from_static(br#"{"model":"gpt-4o","messages":[{"role":"user","content":[{"type":"input_audio","input_audio":{"data":"https://example.com/audio.wav","format":"wav"}}]}]}"#);
        for format in [ProviderFormat::BedrockAnthropic, ProviderFormat::Converse] {
            let error = prepare_request_with_remote_media_and_fetch(
                body.clone(),
                &spec("test-model", format),
                format,
                RemoteMediaPolicy::BEDROCK,
                |_| panic!("unsupported audio must not trigger a fetch"),
            )
            .await
            .expect_err("Bedrock does not support audio input");
            assert!(matches!(error, Error::InvalidRequest(_)));
            assert!(error
                .to_string()
                .contains("remote audio input is unsupported"));
        }
    }

    #[tokio::test]
    async fn native_google_audio_validates_fetched_mime_type() {
        for (declared, fetched, compatible) in [
            ("audio/wav", "audio/mpeg", false),
            ("audio/mpeg", "audio/wav", false),
            ("audio/wav", "Audio/X-Wav; charset=binary", true),
            ("audio/mp3", "audio/mpeg", true),
            ("audio/flac", "audio/flac", true),
        ] {
            let fixture = |data: &str| {
                json!({
                    "contents": [{"role": "user", "parts": [{
                        "inlineData": {"mimeType": declared, "data": data}
                    }]}]
                })
            };
            let result = prepare_request_with_remote_media_and_fetch(
                Bytes::from(
                    lingua::serde_json::to_vec(&fixture("https://example.com/audio?secret=token"))
                        .unwrap(),
                ),
                &google_spec("gemini-3.5-flash"),
                ProviderFormat::Google,
                RemoteMediaPolicy::GOOGLE,
                move |_| {
                    Box::pin(async move {
                        Ok(MediaBlock {
                            media_type: fetched.into(),
                            data: "cmlm".into(),
                        })
                    })
                },
            )
            .await;
            if compatible {
                let actual: lingua::serde_json::Value =
                    lingua::serde_json::from_slice(&result.expect("compatible MIME type").bytes)
                        .unwrap();
                assert_eq!(actual, fixture("cmlm"));
            } else {
                let error = result.expect_err("incompatible MIME type must fail");
                assert!(matches!(error, Error::InvalidRequest(_)));
                let message = error.to_string();
                assert!(message.contains(declared));
                assert!(message.contains(fetched));
                assert!(!message.contains("secret=token"));
            }
        }
    }

    #[tokio::test]
    async fn native_google_audio_inlining_preserves_all_other_fields() {
        let fixture = |data: &str| {
            json!({
                "systemInstruction": {"parts": [{"text": "Keep this instruction"}]},
                "safetySettings": [{"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_LOW_AND_ABOVE"}],
                "cachedContent": "cachedContents/example",
                "custom": {"keep": true},
                "contents": [{"role": "user", "custom": 42, "parts": [
                    {"text": "Listen"},
                    {"text": "Null", "inlineData": null},
                    {"inlineData": {"mimeType": "audio/wav", "data": data, "custom": true}, "custom": "part"}
                ]}]
            })
        };
        let prepared = prepare_request_with_remote_media_and_fetch(
            Bytes::from(
                lingua::serde_json::to_vec(&fixture("https://example.com/call.wav")).unwrap(),
            ),
            &google_spec("gemini-3.5-flash"),
            ProviderFormat::Google,
            RemoteMediaPolicy::GOOGLE,
            wav_fetch("https://example.com/call.wav"),
        )
        .await
        .expect("native Google audio prepares");
        let actual: lingua::serde_json::Value =
            lingua::serde_json::from_slice(&prepared.bytes).unwrap();
        assert_eq!(actual, fixture("cmlm"));
        assert_eq!(prepared.detected_format, None);
        assert!(!prepared.lingua_passthrough);
    }

    #[tokio::test]
    async fn native_audio_inlining_changes_only_audio_data() {
        let fixture = |data: &str| {
            json!({
                "model": "gpt-audio-1.5",
                "provider": {"order": ["custom"]},
                "messages": [
                    {"role": "assistant"},
                    {"role": "assistant", "content": null},
                    {"role": "user", "name": "speaker", "custom": true, "content": [
                        {"type": "text", "text": "Listen"},
                        {"type": "text", "text": "Null", "input_audio": null},
                        {"type": "text", "text": "Not audio", "input_audio": {
                            "data": "https://example.com/do-not-fetch.wav", "format": "wav"
                        }},
                        {"type": "input_audio", "custom": 42, "input_audio": {
                            "data": data, "format": "wav", "custom": {"keep": true}
                        }}
                    ]}
                ]
            })
        };
        let body = Bytes::from(
            lingua::serde_json::to_vec(&fixture("https://example.com/call.wav")).unwrap(),
        );
        let prepared = prepare_request_with_remote_media_and_fetch(
            body,
            &openai_spec("gpt-audio-1.5"),
            ProviderFormat::ChatCompletions,
            RemoteMediaPolicy::OPENAI,
            wav_fetch("https://example.com/call.wav"),
        )
        .await
        .expect("inline native audio");
        let actual: lingua::serde_json::Value =
            lingua::serde_json::from_slice(&prepared.bytes).unwrap();
        assert_eq!(actual, fixture("cmlm"));
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
            wav_fetch("https://example.com/call.wav"),
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

        assert!(matches!(error, Error::InvalidRequest(_)));
        assert!(error.to_string().contains("audio/mpeg"));
        assert!(error.to_string().contains("audio/wav"));
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
