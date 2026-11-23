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
use std::borrow::Cow;

type TraceMessagePart = (Part, Option<UserContentPart>);

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
    function_response: Option<TraceFunctionResponse>,
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
struct TraceFunctionResponse {
    #[serde(alias = "willContinue")]
    will_continue: Option<bool>,
    #[serde(flatten)]
    response: FunctionResponse,
}

impl TraceFunctionResponse {
    fn into_response(self) -> Result<FunctionResponse, ConvertError> {
        if self.will_continue == Some(true) {
            return Err(ConvertError::UnsupportedMapping {
                from: "Google functionResponse.willContinue".to_string(),
                to: "Lingua universal message",
            });
        }
        if self
            .response
            .parts
            .as_ref()
            .is_some_and(|parts| !parts.is_empty())
        {
            return Err(ConvertError::UnsupportedMapping {
                from: "Google functionResponse.parts".to_string(),
                to: "Lingua universal message",
            });
        }
        Ok(self.response)
    }
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
    fn into_parts(self) -> Result<TraceMessagePart, ConvertError> {
        let attachment = match self.image_url {
            Some(TraceAttachmentUrl { url }) => {
                let media_type = match &url {
                    AttachmentReference::BraintrustAttachment { content_type, .. }
                    | AttachmentReference::ExternalAttachment { content_type, .. } => {
                        content_type.clone()
                    }
                };
                Some(UserContentPart::Image {
                    image: serde_json::to_value(url).map_err(|error| {
                        ConvertError::JsonSerializationFailed {
                            field: "Google attachment URL".to_string(),
                            error: error.to_string(),
                        }
                    })?,
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
            function_response: self
                .function_response
                .map(TraceFunctionResponse::into_response)
                .transpose()?,
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
        Ok((part, attachment))
    }
}

#[derive(Deserialize)]
struct TraceContent {
    parts: Option<Vec<TracePart>>,
    role: Option<String>,
}

impl TraceContent {
    fn into_role_and_parts(
        self,
        envelope_role: Option<&'static str>,
    ) -> Result<(Cow<'static, str>, Vec<TraceMessagePart>), ConvertError> {
        let role = self
            .role
            .map(Cow::Owned)
            .or(envelope_role.map(Cow::Borrowed))
            .ok_or_else(|| ConvertError::MissingRequiredField {
                field: "Google content role".to_string(),
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
            .collect::<Result<_, _>>()?;
        Ok((role, parts))
    }

    fn into_message(
        self,
        envelope_role: Option<&'static str>,
        trace_tools: Option<&mut TraceToolCalls>,
    ) -> Result<Message, ConvertError> {
        let (role, parts) = self.into_role_and_parts(envelope_role)?;
        Content::message_from_parts_with_trace_tools(&role, parts, trace_tools)
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
    fn into_parts(self) -> Result<TraceMessagePart, ConvertError> {
        match self {
            Self::Text(text) => Ok((
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
    fn into_parts(self) -> Result<TraceMessagePart, ConvertError> {
        let (mut part, attachment) = self.part.into_parts()?;
        let mut file = self.file;
        file.file_uri = file.file_uri.or(self.uri);
        if part.file_data.is_none() && file.file_uri.is_some() {
            part.file_data = Some(file.into());
        }
        if part == Part::default() && attachment.is_none() {
            return Err(ConvertError::ContentConversionFailed {
                reason: "unrecognized Google request part".to_string(),
            });
        }
        Ok((part, attachment))
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
    pending: Vec<TraceMessagePart>,
    trace_tools: TraceToolCalls,
}

impl RequestMessages {
    fn flush(&mut self) -> Result<(), ConvertError> {
        if let Some((part, _)) = self.pending.first() {
            let role = match request_part_kind(part) {
                RequestPartKind::Model => "model",
                RequestPartKind::ToolResult | RequestPartKind::User => "user",
            };
            self.messages
                .push(Content::message_from_parts_with_trace_tools(
                    role,
                    self.pending.drain(..),
                    Some(&mut self.trace_tools),
                )?);
        }
        Ok(())
    }

    fn push_part(&mut self, part: TraceMessagePart) -> Result<(), ConvertError> {
        if self
            .pending
            .first()
            .is_some_and(|(previous, _)| request_part_kind(previous) != request_part_kind(&part.0))
        {
            self.flush()?;
        }
        self.pending.push(part);
        Ok(())
    }

    fn push_content(&mut self, content: TraceContent) -> Result<(), ConvertError> {
        self.flush()?;
        let (role, parts) = content.into_role_and_parts(Some("user"))?;
        if role == "user"
            && parts
                .iter()
                .any(|(part, _)| part.function_response.is_some())
        {
            let mut parts = parts.into_iter().peekable();
            while let Some(first) = parts.next() {
                let is_result = first.0.function_response.is_some();
                let rest = std::iter::from_fn(|| {
                    parts.next_if(|(part, _)| part.function_response.is_some() == is_result)
                });
                self.messages
                    .push(Content::message_from_parts_with_trace_tools(
                        &role,
                        std::iter::once(first).chain(rest),
                        Some(&mut self.trace_tools),
                    )?);
            }
        } else {
            self.messages
                .push(Content::message_from_parts_with_trace_tools(
                    &role,
                    parts,
                    Some(&mut self.trace_tools),
                )?);
        }
        Ok(())
    }
}

fn messages_from_request_entries(
    entries: impl IntoIterator<Item = RequestEntry>,
) -> Result<RequestMessages, ConvertError> {
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
    Ok(request)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SystemInstruction {
    Text(String),
    Object(Box<RequestObject>),
    Parts(Vec<RequestPart>),
}

impl SystemInstruction {
    fn into_message(self) -> Result<Message, ConvertError> {
        let message = match self {
            Self::Text(text) => {
                return Ok(Message::System {
                    content: UserContent::String(text),
                })
            }
            Self::Object(object) if object.is_content() => {
                object.content.into_message(Some("user"), None)?
            }
            Self::Object(object) => {
                let part = object.part.into_parts()?;
                Content::message_from_parts("user", [part])?
            }
            Self::Parts(parts) => {
                let parts = parts
                    .into_iter()
                    .map(RequestPart::into_parts)
                    .collect::<Result<Vec<_>, _>>()?;
                Content::message_from_parts("user", parts)?
            }
        };
        match message {
            Message::User { content } => Ok(Message::System { content }),
            _ => Err(ConvertError::UnsupportedMapping {
                from: "Google system instruction with non-user content".to_string(),
                to: "Lingua system message",
            }),
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
) -> Option<Result<Vec<Message>, ConvertError>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let request: TraceRequest = try_parse(data)?;
    match convert_request(request) {
        Ok(parsed) => {
            let messages = non_empty_messages(parsed.messages)?;
            *trace_tools = parsed.trace_tools;
            Some(Ok(messages))
        }
        Err(error) => Some(Err(error)),
    }
}

fn convert_request(request: TraceRequest) -> Result<RequestMessages, ConvertError> {
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
    Ok(parsed)
}

fn messages_from_candidates(
    candidates: Vec<TraceCandidate>,
    in_candidates: bool,
    trace_tools: &mut TraceToolCalls,
) -> Result<Option<Vec<Message>>, ConvertError> {
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
    Ok(non_empty_messages(messages))
}

fn try_parse_response(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let response: TraceResponse = try_parse(data)?;
    messages_from_candidates(response.candidates, true, trace_tools).transpose()
}

fn try_parse_candidates(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Vec<Message>> {
    messages_from_candidates(try_parse_vec_or_single(data)?, false, trace_tools).ok()?
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
    messages_from_candidates(vec![candidate], false, trace_tools).transpose()
}

pub(crate) fn try_parse_google_for_import(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    try_parse_content(data)
        .and_then(non_empty_messages)
        .map(Ok)
        .or_else(|| try_parse_request(data, trace_tools))
        .or_else(|| try_parse_response(data, trace_tools))
        .or_else(|| try_parse_candidates(data, trace_tools).map(Ok))
}
