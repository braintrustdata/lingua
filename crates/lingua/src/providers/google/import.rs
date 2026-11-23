pub(crate) use super::convert::TraceToolCalls;
use super::generated::{
    AudioTranscription, Blob, CodeExecutionResult, Content, ExecutableCode, FileData, FinishReason,
    FunctionCall, FunctionResponse, MediaProcessing, MediaResolution, Part, SpeechMetadata,
    ToolCall, ToolResponse, UsageMetadata, VideoMetadata,
};
use crate::error::ConvertError;
use crate::import_parse::{non_empty_messages, try_parse, try_parse_vec_or_single};
use crate::import_value::{ImportValue, InputKind};
use crate::serde_json;
use crate::universal::message::{AssistantContent, Message, UserContent, UserContentPart};
use crate::universal::trajectory::AttachmentReference;
use serde::Deserialize;

#[derive(Deserialize)]
struct TraceAttachmentUrl {
    url: AttachmentReference,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TracePart {
    #[serde(alias = "image_url")]
    image_url: Option<TraceAttachmentUrl>,
    #[serde(alias = "audio_transcription")]
    audio_transcription: Option<AudioTranscription>,
    #[serde(alias = "code_execution_result")]
    code_execution_result: Option<CodeExecutionResult>,
    #[serde(alias = "executable_code")]
    executable_code: Option<ExecutableCode>,
    #[serde(alias = "file_data")]
    file_data: Option<TraceFileData>,
    #[serde(alias = "function_call")]
    function_call: Option<FunctionCall>,
    #[serde(alias = "function_response")]
    function_response: Option<FunctionResponse>,
    #[serde(alias = "inline_data")]
    inline_data: Option<TraceBlob>,
    #[serde(alias = "media_processing")]
    media_processing: Option<MediaProcessing>,
    #[serde(alias = "media_resolution")]
    media_resolution: Option<MediaResolution>,
    #[serde(alias = "part_metadata")]
    part_metadata: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(alias = "speech_metadata")]
    speech_metadata: Option<SpeechMetadata>,
    text: Option<String>,
    thought: Option<bool>,
    #[serde(alias = "thought_signature")]
    thought_signature: Option<String>,
    #[serde(alias = "tool_call")]
    tool_call: Option<ToolCall>,
    #[serde(alias = "tool_response")]
    tool_response: Option<ToolResponse>,
    #[serde(alias = "video_metadata")]
    video_metadata: Option<VideoMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceBlob {
    data: Option<String>,
    #[serde(alias = "display_name")]
    display_name: Option<String>,
    #[serde(alias = "mime_type")]
    mime_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceFileData {
    #[serde(alias = "display_name")]
    display_name: Option<String>,
    #[serde(alias = "file_uri")]
    file_uri: Option<String>,
    #[serde(alias = "mime_type")]
    mime_type: Option<String>,
}

impl From<TraceFileData> for FileData {
    fn from(file: TraceFileData) -> Self {
        Self {
            display_name: file.display_name,
            file_uri: file.file_uri,
            mime_type: file.mime_type,
        }
    }
}

impl TracePart {
    fn into_parts(self) -> Option<(Part, Option<UserContentPart>)> {
        let attachment = match self.image_url {
            Some(TraceAttachmentUrl { url }) => {
                let media_type = match &url {
                    AttachmentReference::BraintrustAttachment { content_type, .. }
                    | AttachmentReference::ExternalAttachment { content_type, .. } => {
                        content_type.clone()
                    }
                };
                Some(UserContentPart::Image {
                    image: serde_json::to_value(url).ok()?,
                    media_type: Some(media_type),
                    provider_options: None,
                })
            }
            None => None,
        };
        let part = Part {
            audio_transcription: self.audio_transcription,
            code_execution_result: self.code_execution_result,
            executable_code: self.executable_code,
            file_data: self.file_data.map(FileData::from),
            function_call: self.function_call,
            function_response: self.function_response,
            inline_data: self.inline_data.map(|blob| Blob {
                data: blob.data,
                display_name: blob.display_name,
                mime_type: blob.mime_type,
            }),
            media_processing: self.media_processing,
            media_resolution: self.media_resolution,
            part_metadata: self.part_metadata,
            speech_metadata: self.speech_metadata,
            text: self.text,
            thought: self.thought,
            thought_signature: self.thought_signature,
            tool_call: self.tool_call,
            tool_response: self.tool_response,
            video_metadata: self.video_metadata,
        };
        Some((part, attachment))
    }
}

#[derive(Deserialize)]
struct TraceContent {
    parts: Option<Vec<TracePart>>,
    role: Option<String>,
}

impl TraceContent {
    fn into_message(
        self,
        envelope_role: Option<&str>,
        trace_tools: Option<&mut TraceToolCalls>,
    ) -> Result<Message, ConvertError> {
        let role = self.role.as_deref().or(envelope_role).ok_or_else(|| {
            ConvertError::MissingRequiredField {
                field: "Google content role".to_string(),
            }
        })?;
        let parts = match self.parts {
            Some(parts) => parts,
            None if role == "model" => Vec::new(),
            None => {
                return Err(ConvertError::MissingRequiredField {
                    field: "Google content parts".to_string(),
                });
            }
        };
        let parts = parts
            .into_iter()
            .map(TracePart::into_parts)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| ConvertError::ContentConversionFailed {
                reason: "invalid Google content part".to_string(),
            })?;
        Content::message_from_parts_with_trace_tools(role, parts, trace_tools)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequestContents {
    Text(String),
    Entry(Box<RequestObject>),
    Entries(Vec<RequestEntry>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequestEntry {
    Text(String),
    Object(Box<RequestObject>),
    Parts(Vec<RequestPart>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequestPart {
    Text(String),
    Object(Box<RequestPartObject>),
}

impl RequestPart {
    fn into_parts(self) -> Option<(Part, Option<UserContentPart>)> {
        match self {
            Self::Text(text) => Some((
                Part {
                    text: Some(text),
                    ..Default::default()
                },
                None,
            )),
            Self::Object(part) => part.into_parts(),
        }
    }
}

#[derive(Deserialize)]
struct RequestPartObject {
    #[serde(flatten)]
    part: TracePart,
    #[serde(flatten)]
    file: TraceFileData,
    uri: Option<String>,
}

impl RequestPartObject {
    fn into_parts(self) -> Option<(Part, Option<UserContentPart>)> {
        let (mut part, attachment) = self.part.into_parts()?;
        if part == Part::default() && attachment.is_none() {
            let mut file = self.file;
            file.file_uri = Some(file.file_uri.or(self.uri)?);
            part.file_data = Some(file.into());
        }
        Some((part, attachment))
    }
}

#[derive(Deserialize)]
struct RequestObject {
    #[serde(flatten)]
    content: TraceContent,
    #[serde(flatten)]
    part: RequestPartObject,
}

impl RequestObject {
    fn is_content(&self) -> bool {
        self.content.parts.is_some() || self.content.role.is_some()
    }
}

#[derive(PartialEq)]
enum RequestPartKind {
    Model,
    ToolResult,
    User,
}

fn request_part_kind(part: &Part) -> RequestPartKind {
    if part.function_call.is_some() {
        RequestPartKind::Model
    } else if part.function_response.is_some() {
        RequestPartKind::ToolResult
    } else {
        RequestPartKind::User
    }
}

#[derive(Default)]
struct RequestMessages {
    messages: Vec<Message>,
    pending: Vec<(Part, Option<UserContentPart>)>,
    trace_tools: TraceToolCalls,
}

impl RequestMessages {
    fn flush(&mut self) -> Option<()> {
        if let Some((part, _)) = self.pending.first() {
            let role = match request_part_kind(part) {
                RequestPartKind::Model => "model",
                RequestPartKind::ToolResult | RequestPartKind::User => "user",
            };
            self.messages.push(
                Content::message_from_parts_with_trace_tools(
                    role,
                    self.pending.drain(..),
                    Some(&mut self.trace_tools),
                )
                .ok()?,
            );
        }
        Some(())
    }

    fn push_part(&mut self, part: (Part, Option<UserContentPart>)) -> Option<()> {
        if self
            .pending
            .first()
            .is_some_and(|(previous, _)| request_part_kind(previous) != request_part_kind(&part.0))
        {
            self.flush()?;
        }
        self.pending.push(part);
        Some(())
    }

    fn push_content(&mut self, content: TraceContent) -> Option<()> {
        self.flush()?;
        self.messages.push(
            content
                .into_message(Some("user"), Some(&mut self.trace_tools))
                .ok()?,
        );
        Some(())
    }
}

fn messages_from_request_entries(
    entries: impl IntoIterator<Item = RequestEntry>,
) -> Option<RequestMessages> {
    let mut request = RequestMessages::default();
    for entry in entries {
        let part = match entry {
            RequestEntry::Text(text) => RequestPart::Text(text).into_parts()?,
            RequestEntry::Object(object) => {
                if object.is_content() {
                    request.push_content(object.content)?;
                    continue;
                }
                object.part.into_parts()?
            }
            RequestEntry::Parts(parts) => {
                request.flush()?;
                for part in parts {
                    request.push_part(part.into_parts()?)?;
                }
                request.flush()?;
                continue;
            }
        };
        request.push_part(part)?;
    }
    request.flush()?;
    Some(request)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SystemInstruction {
    Text(String),
    Object(Box<RequestObject>),
    Parts(Vec<RequestPart>),
}

impl SystemInstruction {
    fn into_message(self) -> Option<Message> {
        let message = match self {
            Self::Text(text) => {
                return Some(Message::System {
                    content: UserContent::String(text),
                })
            }
            Self::Object(object) if object.is_content() => {
                object.content.into_message(Some("user"), None).ok()?
            }
            Self::Object(object) => {
                let part = object.part.into_parts()?;
                Content::message_from_parts("user", [part]).ok()?
            }
            Self::Parts(parts) => {
                let parts = parts
                    .into_iter()
                    .map(RequestPart::into_parts)
                    .collect::<Option<Vec<_>>>()?;
                Content::message_from_parts("user", parts).ok()?
            }
        };
        match message {
            Message::User { content } => Some(Message::System { content }),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceConfig {
    #[serde(alias = "system_instruction")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceRequest {
    #[serde(alias = "Contents")]
    contents: RequestContents,
    #[serde(alias = "Config")]
    config: Option<TraceConfig>,
    #[serde(alias = "system_instruction")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Deserialize)]
struct TraceResponse {
    candidates: Vec<TraceCandidate>,
}

// ADK logs a single result at the top level instead of a GenerateContent candidates array.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceCandidate {
    #[serde(alias = "Content")]
    content: Option<TraceContent>,
    #[serde(alias = "finish_reason", alias = "FinishReason")]
    finish_reason: Option<FinishReason>,
    #[serde(alias = "usage_metadata", alias = "UsageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    #[serde(alias = "error_code", alias = "ErrorCode")]
    error_code: Option<String>,
    #[serde(alias = "error_message", alias = "ErrorMessage")]
    error_message: Option<String>,
}

impl TraceCandidate {
    fn has_roleless_finished_content(&self) -> bool {
        self.finish_reason.is_some()
            && self
                .content
                .as_ref()
                .is_some_and(|content| content.role.is_none() && content.parts.is_some())
    }

    fn is_finished(&self, in_candidates: bool) -> bool {
        self.has_roleless_finished_content()
            || (self.finish_reason.is_some() && (in_candidates || self.usage_metadata.is_some()))
    }

    fn is_adk_error(&self) -> bool {
        self.error_code.as_deref() == Some("UNKNOWN_ERROR") && self.error_message.is_some()
    }
}

fn try_parse_content<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    try_parse_vec_or_single::<TraceContent>(data)?
        .into_iter()
        .map(|content| content.into_message(None, None).ok())
        .collect()
}

fn try_parse_request(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Vec<Message>> {
    let request: TraceRequest = try_parse(data)?;
    let mut parsed = match request.contents {
        RequestContents::Text(text) => RequestMessages {
            messages: vec![Message::User {
                content: UserContent::String(text),
            }],
            ..Default::default()
        },
        RequestContents::Entry(object) => {
            messages_from_request_entries([RequestEntry::Object(object)])?
        }
        RequestContents::Entries(entries) => messages_from_request_entries(entries)?,
    };
    let system_instruction = request
        .system_instruction
        .or_else(|| request.config?.system_instruction);
    if let Some(instruction) = system_instruction {
        parsed.messages.insert(0, instruction.into_message()?);
    }
    let messages = non_empty_messages(parsed.messages)?;
    *trace_tools = parsed.trace_tools;
    Some(messages)
}

fn messages_from_candidates(
    candidates: Vec<TraceCandidate>,
    in_candidates: bool,
    trace_tools: &mut TraceToolCalls,
) -> Result<Vec<Message>, ConvertError> {
    let mut messages = Vec::new();
    for candidate in candidates {
        let finished = candidate.is_finished(in_candidates);
        if let Some(content) = candidate.content {
            let envelope_role = (in_candidates || finished).then_some("model");
            messages.push(content.into_message(envelope_role, Some(trace_tools))?);
        } else if finished || candidate.is_adk_error() {
            messages.push(Message::Assistant {
                content: AssistantContent::String(String::new()),
                id: None,
            });
        }
    }
    Ok(messages)
}

fn try_parse_response(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Vec<Message>> {
    let response: TraceResponse = try_parse(data)?;
    non_empty_messages(messages_from_candidates(response.candidates, true, trace_tools).ok()?)
}

fn try_parse_candidates(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Vec<Message>> {
    non_empty_messages(
        messages_from_candidates(try_parse_vec_or_single(data)?, false, trace_tools).ok()?,
    )
}

pub(crate) fn try_parse_adk_for_import(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let candidate: TraceCandidate = try_parse(data)?;
    let has_google_content = candidate.content.as_ref().is_some_and(|content| {
        content.role.as_deref() == Some("model")
            && (content.parts.is_some() || candidate.finish_reason.is_some())
    });
    if candidate.usage_metadata.is_none()
        && !has_google_content
        && !candidate.has_roleless_finished_content()
        && !candidate.is_adk_error()
    {
        return None;
    }
    match messages_from_candidates(vec![candidate], false, trace_tools) {
        Ok(messages) => non_empty_messages(messages).map(Ok),
        Err(error) => Some(Err(error)),
    }
}

pub(crate) fn try_parse_google_for_import(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Vec<Message>> {
    try_parse_content(data)
        .and_then(non_empty_messages)
        .or_else(|| try_parse_request(data, trace_tools))
        .or_else(|| try_parse_response(data, trace_tools))
        .or_else(|| try_parse_candidates(data, trace_tools))
}
