use super::generated::{
    Content, FinishReason, FunctionCall, FunctionResponse, Part, UsageMetadata,
};
use crate::import_parse::{
    non_empty_messages, try_parse, try_parse_vec_or_single, try_parsers_in_order, MessageParser,
};
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
struct TracePart {
    #[serde(alias = "imageUrl")]
    image_url: Option<TraceAttachmentUrl>,
    function_call: Option<FunctionCall>,
    function_response: Option<FunctionResponse>,
    thought_signature: Option<String>,
    #[serde(flatten)]
    rest: Part,
}

impl TracePart {
    fn into_parts(mut self) -> Option<(Part, Option<UserContentPart>)> {
        self.rest.function_call = self.rest.function_call.or(self.function_call);
        self.rest.function_response = self.rest.function_response.or(self.function_response);
        self.rest.thought_signature = self.rest.thought_signature.or(self.thought_signature);
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
        Some((self.rest, attachment))
    }
}

#[derive(Deserialize)]
struct TraceContent {
    parts: Option<Vec<TracePart>>,
    role: Option<String>,
}

impl TraceContent {
    fn into_message(self, envelope_role: Option<&str>) -> Option<Message> {
        let role = self.role.as_deref().or(envelope_role)?;
        let parts = match self.parts {
            Some(parts) => parts,
            None if self.role.as_deref() == Some("model") => Vec::new(),
            None => return None,
        };
        let parts = parts
            .into_iter()
            .map(TracePart::into_parts)
            .collect::<Option<Vec<_>>>()?;
        Content::message_from_parts(role, parts).ok()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RequestContents {
    Text(String),
    Conversation(Vec<TraceContent>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SystemInstruction {
    Text(String),
    Content(TraceContent),
}

impl SystemInstruction {
    fn into_message(self) -> Option<Message> {
        let content = match self {
            Self::Text(text) => UserContent::String(text),
            Self::Content(content) => match content.into_message(Some("user"))? {
                Message::User { content } => content,
                _ => return None,
            },
        };
        Some(Message::System { content })
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
    fn is_finished(&self, in_candidates: bool) -> bool {
        self.finish_reason.is_some() && (in_candidates || self.usage_metadata.is_some())
    }

    fn is_adk_error(&self) -> bool {
        self.error_code.as_deref() == Some("UNKNOWN_ERROR") && self.error_message.is_some()
    }
}

fn try_parse_content<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    try_parse_vec_or_single::<TraceContent>(data)?
        .into_iter()
        .map(|content| content.into_message(None))
        .collect()
}

fn try_parse_request<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    let request: TraceRequest = try_parse(data)?;
    let mut messages = match request.contents {
        RequestContents::Text(text) => vec![Message::User {
            content: UserContent::String(text),
        }],
        // The Google request schema permits an omitted role for user content.
        RequestContents::Conversation(contents) => contents
            .into_iter()
            .map(|content| content.into_message(Some("user")))
            .collect::<Option<Vec<_>>>()?,
    };
    let system_instruction = request
        .system_instruction
        .or_else(|| request.config?.system_instruction);
    if let Some(instruction) = system_instruction {
        messages.insert(0, instruction.into_message()?);
    }
    Some(messages)
}

fn messages_from_candidates(
    candidates: Vec<TraceCandidate>,
    in_candidates: bool,
) -> Option<Vec<Message>> {
    let mut messages = Vec::new();
    for candidate in candidates {
        let finished = candidate.is_finished(in_candidates);
        if let Some(content) = candidate.content {
            let envelope_role = (in_candidates || finished).then_some("model");
            messages.push(content.into_message(envelope_role)?);
        } else if finished || candidate.is_adk_error() {
            messages.push(Message::Assistant {
                content: AssistantContent::String(String::new()),
                id: None,
            });
        }
    }
    Some(messages)
}

fn try_parse_response<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    let response: TraceResponse = try_parse(data)?;
    messages_from_candidates(response.candidates, true)
}

fn try_parse_candidates<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    messages_from_candidates(try_parse_vec_or_single(data)?, false)
}

pub(crate) fn try_parse_adk_for_import(data: &impl ImportValue) -> Option<Vec<Message>> {
    if data.kind() != InputKind::Object {
        return None;
    }
    let candidate: TraceCandidate = try_parse(data)?;
    let has_google_content = candidate
        .content
        .as_ref()
        .is_some_and(|content| content.role.as_deref() == Some("model") && content.parts.is_some());
    if candidate.usage_metadata.is_none() && !has_google_content && !candidate.is_adk_error() {
        return None;
    }
    non_empty_messages(messages_from_candidates(vec![candidate], false)?)
}

pub(crate) fn try_parse_google_for_import<V: ImportValue>(data: &V) -> Option<Vec<Message>> {
    let parsers: &[MessageParser<V>] = &[
        try_parse_content,
        try_parse_request,
        try_parse_response,
        try_parse_candidates,
    ];
    try_parsers_in_order(data, parsers)
}
