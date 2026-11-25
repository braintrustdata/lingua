pub(crate) use super::convert::TraceToolCalls;
use super::generated::{
    AudioTranscription, Blob, BlockReason, CodeExecutionResult, Content, ExecutableCode, FileData,
    FinishReason, FunctionCall, FunctionResponse, Level, MediaProcessing, Part, Scheduling,
    SpeechMetadata, ToolCall, ToolResponse, UsageMetadata,
};
use crate::error::ConvertError;
use crate::import_parse::{non_empty_messages, try_parse, try_parse_vec_or_single};
use crate::import_value::{ImportValue, InputKind};
use crate::serde_json;
use crate::universal::message::{AssistantContent, Message, UserContent, UserContentPart};
use crate::universal::trajectory::AttachmentReference;
use serde::{Deserialize, de::IgnoredAny};
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
    #[serde(alias = "audio_metadata")]
    audio_metadata: Option<IgnoredAny>,
    #[serde(alias = "audio_transcription")]
    audio_transcription: Option<AudioTranscription>,
    #[serde(alias = "code_execution_result")]
    code_execution_result: Option<CodeExecutionResult>,
    #[serde(alias = "document_metadata")]
    document_metadata: Option<IgnoredAny>,
    #[serde(alias = "executable_code")]
    executable_code: Option<ExecutableCode>,
    #[serde(alias = "file_data")]
    file_data: Option<TraceFileData>,
    #[serde(alias = "function_call")]
    function_call: Option<TraceFunctionCall>,
    #[serde(alias = "function_response")]
    function_response: Option<TraceFunctionResponse>,
    #[serde(alias = "inline_data")]
    inline_data: Option<TraceBlob>,
    #[serde(alias = "media_processing")]
    media_processing: Option<MediaProcessing>,
    #[serde(alias = "media_resolution")]
    media_resolution: Option<TraceMediaResolution>,
    #[serde(alias = "part_metadata")]
    part_metadata: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(alias = "speech_metadata")]
    speech_metadata: Option<SpeechMetadata>,
    #[serde(alias = "spreadsheet_metadata")]
    spreadsheet_metadata: Option<IgnoredAny>,
    text: Option<String>,
    thought: Option<bool>,
    #[serde(alias = "thought_signature")]
    thought_signature: Option<String>,
    #[serde(alias = "tool_call")]
    tool_call: Option<ToolCall>,
    #[serde(alias = "tool_response")]
    tool_response: Option<ToolResponse>,
    #[serde(alias = "video_metadata")]
    video_metadata: Option<TraceVideoMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceMediaResolution {
    level: Option<Level>,
    #[serde(alias = "num_tokens")]
    num_tokens: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceVideoMetadata {
    #[serde(alias = "end_offset")]
    end_offset: Option<String>,
    fps: Option<f64>,
    #[serde(alias = "start_offset")]
    start_offset: Option<String>,
}

impl TraceVideoMetadata {
    fn has_preprocessing(&self) -> bool {
        self.end_offset.is_some() || self.fps.is_some() || self.start_offset.is_some()
    }
}

#[derive(Deserialize)]
struct TraceFunctionCall {
    #[serde(alias = "willContinue")]
    will_continue: Option<bool>,
    #[serde(alias = "partialArgs")]
    partial_args: Option<Vec<IgnoredAny>>,
    #[serde(flatten)]
    call: FunctionCall,
}

impl TraceFunctionCall {
    fn into_call(self) -> Result<FunctionCall, ConvertError> {
        let unsupported_field = if self.will_continue == Some(true) {
            "willContinue"
        } else if self.partial_args.is_some_and(|args| !args.is_empty()) {
            "partialArgs"
        } else {
            return Ok(self.call);
        };
        Err(ConvertError::UnsupportedMapping {
            from: format!("Google functionCall.{unsupported_field}"),
            to: "Lingua universal message",
        })
    }
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
        let unsupported_field = if self.will_continue == Some(true) {
            "willContinue"
        } else if self
            .response
            .scheduling
            .as_ref()
            .is_some_and(|scheduling| *scheduling != Scheduling::SchedulingUnspecified)
        {
            "scheduling"
        } else if self
            .response
            .parts
            .as_ref()
            .is_some_and(|parts| !parts.is_empty())
        {
            "parts"
        } else {
            return Ok(self.response);
        };
        Err(ConvertError::UnsupportedMapping {
            from: format!("Google functionResponse.{unsupported_field}"),
            to: "Lingua universal message",
        })
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
    fn validate_for_import(&self) -> Result<(), ConvertError> {
        let unsupported_field = if self.executable_code.is_some() {
            "executableCode"
        } else if self.code_execution_result.is_some() {
            "codeExecutionResult"
        } else if self.tool_response.is_some() {
            "toolResponse"
        } else if self.media_resolution.as_ref().is_some_and(|resolution| {
            resolution.num_tokens.is_some()
                || resolution
                    .level
                    .as_ref()
                    .is_some_and(|level| *level != Level::MediaResolutionUnspecified)
        }) {
            "mediaResolution"
        } else if self
            .part_metadata
            .as_ref()
            .is_some_and(|metadata| !metadata.is_empty())
        {
            "partMetadata"
        } else if self
            .video_metadata
            .as_ref()
            .is_some_and(TraceVideoMetadata::has_preprocessing)
        {
            "videoMetadata preprocessing"
        } else if self.document_metadata.is_some() {
            "documentMetadata preprocessing"
        } else if self.spreadsheet_metadata.is_some() {
            "spreadsheetMetadata preprocessing"
        } else if self.audio_metadata.is_some() {
            "audioMetadata preprocessing"
        } else if self.thought_signature.is_some()
            && (self.inline_data.is_some() || self.file_data.is_some() || self.image_url.is_some())
        {
            "thoughtSignature on media"
        } else {
            return Ok(());
        };
        Err(ConvertError::UnsupportedMapping {
            from: format!("Google part {unsupported_field}"),
            to: "Lingua universal message",
        })
    }

    fn into_parts(
        self,
        shorthand_file: Option<TraceFileData>,
    ) -> Result<TraceMessagePart, ConvertError> {
        self.validate_for_import()?;
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
            file_data: self.file_data.or(shorthand_file).map(FileData::from),
            function_call: self
                .function_call
                .map(TraceFunctionCall::into_call)
                .transpose()?,
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
            speech_metadata: self.speech_metadata,
            text: self.text,
            thought: self.thought,
            thought_signature: self.thought_signature,
            tool_call: self.tool_call,
            ..Default::default()
        };
        if part == Part::default() && attachment.is_none() {
            return Err(ConvertError::ContentConversionFailed {
                reason: "unrecognized Google content part".to_string(),
            });
        }
        Ok((part, attachment))
    }
}

#[derive(Deserialize)]
struct TraceContent {
    parts: Option<Vec<TracePart>>,
    role: Option<String>,
}

impl TraceContent {
    fn is_direct_content(&self) -> bool {
        match self.role.as_deref() {
            Some("model") => true,
            Some("user" | "tool") => self.parts.is_some(),
            _ => false,
        }
    }

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
            .map(|part| part.into_parts(None))
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
        let mut file = self.file;
        file.file_uri = file.file_uri.or(self.uri);
        self.part
            .into_parts(file.file_uri.is_some().then_some(file))
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

    fn push_content_parts(
        &mut self,
        role: &str,
        parts: Vec<TraceMessagePart>,
    ) -> Result<(), ConvertError> {
        if role == "user" && parts.iter().any(|(part, _)| part.function_call.is_some()) {
            return Err(ConvertError::UnsupportedMapping {
                from: "Google user content with functionCall".to_string(),
                to: "Lingua universal message",
            });
        }
        let has_tool_result = parts
            .iter()
            .any(|(part, _)| part.function_response.is_some());
        if role == "tool"
            && has_tool_result
            && parts
                .iter()
                .any(|(part, _)| part.function_response.is_none())
        {
            return Err(ConvertError::UnsupportedMapping {
                from: "Google tool role with functionResponse and non-result parts".to_string(),
                to: "Lingua tool message",
            });
        }
        if role == "user" && has_tool_result {
            let mut parts = parts.into_iter().peekable();
            while let Some(first) = parts.next() {
                let is_result = first.0.function_response.is_some();
                let rest = std::iter::from_fn(|| {
                    parts.next_if(|(part, _)| part.function_response.is_some() == is_result)
                });
                self.messages
                    .push(Content::message_from_parts_with_trace_tools(
                        role,
                        std::iter::once(first).chain(rest),
                        Some(&mut self.trace_tools),
                    )?);
            }
        } else {
            self.messages
                .push(Content::message_from_parts_with_trace_tools(
                    role,
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
                    request.flush()?;
                    let (role, parts) = object.content.into_role_and_parts(Some("user"))?;
                    request.push_content_parts(&role, parts)?;
                    continue;
                }
                object.part.into_parts()?
            }
            RequestEntry::Parts(parts) => {
                request.flush()?;
                if !parts.is_empty() {
                    let parts = parts
                        .into_iter()
                        .map(RequestPart::into_parts)
                        .collect::<Result<_, _>>()?;
                    request.push_content_parts("user", parts)?;
                }
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
                });
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
    #[serde(alias = "continuation_token", alias = "ContinuationToken")]
    continuation_token: Option<String>,
    #[serde(alias = "Config")]
    config: Option<TraceConfig>,
    #[serde(alias = "system_instruction")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceResponse {
    #[serde(alias = "Candidates")]
    candidates: Option<Vec<TraceCandidate>>,
    #[serde(alias = "automatic_function_calling_history")]
    automatic_function_calling_history: Option<Vec<IgnoredAny>>,
    #[serde(alias = "prompt_feedback", alias = "PromptFeedback")]
    prompt_feedback: Option<TracePromptFeedback>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TracePromptFeedback {
    #[serde(alias = "block_reason", alias = "BlockReason")]
    block_reason: Option<BlockReason>,
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum TraceInteractionStatus {
    Idle,
    InProgress,
    #[serde(other)]
    Unknown,
}

// ADK logs a single result at the top level instead of a GenerateContent candidates array.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TraceCandidate {
    #[serde(alias = "Content")]
    content: Option<TraceContent>,
    #[serde(alias = "finish_reason", alias = "FinishReason")]
    finish_reason: Option<FinishReason>,
    #[serde(alias = "continuation_token", alias = "ContinuationToken")]
    continuation_token: Option<String>,
    #[serde(alias = "usage_metadata", alias = "UsageMetadata")]
    usage_metadata: Option<UsageMetadata>,
    #[serde(alias = "error_code", alias = "ErrorCode")]
    error_code: Option<String>,
    #[serde(alias = "Partial")]
    partial: Option<bool>,
    #[serde(alias = "Interrupted")]
    interrupted: Option<bool>,
    #[serde(alias = "turn_complete", alias = "TurnComplete")]
    turn_complete: Option<bool>,
    #[serde(alias = "turn_complete_reason", alias = "TurnCompleteReason")]
    turn_complete_reason: Option<String>,
    #[serde(alias = "interaction_status", alias = "InteractionStatus")]
    interaction_status: Option<TraceInteractionStatus>,
}

impl TraceCandidate {
    fn has_roleless_candidate_content(&self) -> bool {
        (self.finish_reason.is_some()
            || self.turn_complete == Some(true)
            || self.partial == Some(false))
            && self
                .content
                .as_ref()
                .is_some_and(|content| content.role.is_none() && content.parts.is_some())
    }

    fn has_adk_lifecycle(&self) -> bool {
        (self.partial == Some(true)
            || self.interrupted == Some(true)
            || self.turn_complete.is_some()
            || self.turn_complete_reason.is_some()
            || self.interaction_status.is_some())
            && self
                .content
                .as_ref()
                .is_none_or(|content| matches!(content.role.as_deref(), None | Some("model")))
    }

    fn has_content(&self) -> bool {
        self.content
            .as_ref()
            .is_some_and(|content| content.role.is_some() || content.parts.is_some())
    }

    fn has_terminal_finish_reason(&self) -> bool {
        !matches!(
            self.finish_reason,
            None | Some(FinishReason::FinishReasonUnspecified | FinishReason::Continuation)
        )
    }

    fn is_finished(&self, in_candidates: bool) -> bool {
        let terminal = self.has_terminal_finish_reason();
        let turn_complete = self.turn_complete == Some(true);
        ((terminal || turn_complete || self.partial == Some(false))
            && self.has_roleless_candidate_content())
            || (terminal && (in_candidates || self.usage_metadata.is_some() || turn_complete))
    }

    fn is_adk_error(&self) -> bool {
        self.error_code
            .as_ref()
            .is_some_and(|code| !code.is_empty())
    }

    fn validate_candidate_state(
        &self,
        in_candidates: bool,
        finished: bool,
    ) -> Result<(), ConvertError> {
        let field = if self.continuation_token.is_some()
            && (in_candidates || finished || self.content.is_some())
        {
            "continuationToken"
        } else if self.finish_reason == Some(FinishReason::Continuation) {
            "finishReason CONTINUATION"
        } else if self.finish_reason == Some(FinishReason::FinishReasonUnspecified)
            && !(self.turn_complete == Some(true) && self.has_content())
            && !self.is_adk_error()
        {
            "finishReason FINISH_REASON_UNSPECIFIED"
        } else if in_candidates
            && self.content.is_some()
            && self.finish_reason.is_none()
            && self.turn_complete != Some(true)
            && self.partial != Some(false)
        {
            "finishReason (missing from unfinished generation)"
        } else {
            return Ok(());
        };
        Err(ConvertError::UnsupportedMapping {
            from: format!("Google candidate.{field}"),
            to: "Lingua universal message",
        })
    }

    fn validate_lifecycle(&self, finished: bool) -> Result<(), ConvertError> {
        if let Some(reason) = self.turn_complete_reason.as_deref()
            && reason != "TURN_COMPLETE_REASON_UNSPECIFIED"
        {
            return Err(ConvertError::UnsupportedMapping {
                from: format!("ADK LlmResponse.turnCompleteReason {reason}"),
                to: "Lingua universal message",
            });
        }

        let field = if self.partial == Some(true) {
            "partial"
        } else if self.interrupted == Some(true) {
            "interrupted"
        } else if matches!(
            self.interaction_status,
            Some(TraceInteractionStatus::InProgress)
        ) {
            "interactionStatus IN_PROGRESS"
        } else if matches!(
            self.interaction_status,
            Some(TraceInteractionStatus::Unknown)
        ) {
            "interactionStatus (unknown value)"
        } else if !self.is_adk_error()
            && ((self.turn_complete == Some(false) && !self.has_terminal_finish_reason())
                || (self.turn_complete == Some(true) && !self.has_content() && !finished))
        {
            "turnComplete"
        } else {
            return Ok(());
        };
        Err(ConvertError::UnsupportedMapping {
            from: format!("ADK LlmResponse.{field}"),
            to: "Lingua universal message",
        })
    }
}

fn try_parse_content(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    let contents = try_parse_vec_or_single::<TraceContent>(data)?;
    if contents.is_empty() || !contents.iter().all(TraceContent::is_direct_content) {
        return None;
    }
    let mut parsed = RequestMessages {
        trace_tools: trace_tools.clone(),
        ..Default::default()
    };
    for content in contents {
        let result = content
            .into_role_and_parts(None)
            .and_then(|(role, parts)| parsed.push_content_parts(&role, parts));
        if let Err(error) = result {
            return Some(Err(error));
        }
    }
    *trace_tools = parsed.trace_tools;
    Some(Ok(parsed.messages))
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
    if request.continuation_token.is_some() {
        return Err(ConvertError::UnsupportedMapping {
            from: "Google request.continuationToken".to_string(),
            to: "Lingua universal message",
        });
    }
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
    let mut pending_tools = None;
    for candidate in candidates {
        let finished = candidate.is_finished(in_candidates);
        candidate.validate_candidate_state(in_candidates, finished)?;
        candidate.validate_lifecycle(finished)?;
        if let Some(content) = candidate.content {
            let envelope_role = (in_candidates || finished).then_some("model");
            let tools = pending_tools.get_or_insert_with(|| trace_tools.clone());
            messages.push(content.into_message(envelope_role, Some(tools))?);
        } else if finished || candidate.is_adk_error() {
            messages.push(Message::Assistant {
                content: AssistantContent::String(String::new()),
                id: None,
            });
        }
    }
    if let Some(tools) = pending_tools {
        *trace_tools = tools;
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
    if response
        .automatic_function_calling_history
        .is_some_and(|history| !history.is_empty())
    {
        return Some(Err(ConvertError::UnsupportedMapping {
            from: "Google response.automaticFunctionCallingHistory".to_string(),
            to: "Lingua universal message",
        }));
    }
    let candidates = response.candidates.unwrap_or_default();
    let prompt_was_blocked = response
        .prompt_feedback
        .and_then(|feedback| feedback.block_reason)
        .is_some_and(|reason| reason != BlockReason::BlockReasonUnspecified);
    if candidates.is_empty() && prompt_was_blocked {
        return Some(Ok(vec![Message::Assistant {
            content: AssistantContent::Array(vec![]),
            id: None,
        }]));
    }
    messages_from_candidates(candidates, true, trace_tools).transpose()
}

fn try_parse_candidates(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    let candidates = try_parse_vec_or_single::<TraceCandidate>(data)?;
    if candidates.iter().any(|candidate| {
        candidate.content.as_ref().is_some_and(|content| {
            !content.is_direct_content()
                && !candidate.has_roleless_candidate_content()
                && !candidate.has_adk_lifecycle()
        })
    }) {
        return None;
    }
    messages_from_candidates(candidates, false, trace_tools).transpose()
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
        && !candidate.has_roleless_candidate_content()
        && !candidate.is_adk_error()
        && !candidate.has_adk_lifecycle()
    {
        return None;
    }
    messages_from_candidates(vec![candidate], false, trace_tools).transpose()
}

pub(crate) fn try_parse_google_for_import(
    data: &impl ImportValue,
    trace_tools: &mut TraceToolCalls,
) -> Option<Result<Vec<Message>, ConvertError>> {
    try_parse_content(data, trace_tools)
        .or_else(|| try_parse_request(data, trace_tools))
        .or_else(|| try_parse_response(data, trace_tools))
        .or_else(|| try_parse_candidates(data, trace_tools))
}
