use crate::import_parse::{non_empty_messages, try_parse_vec_or_single};
use crate::import_value::ImportValue;
use crate::providers::bedrock::request::{
    BedrockContentBlock, BedrockConversationRole, BedrockImageFormat, BedrockMessage,
    BedrockSystemContentBlock,
};
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::trajectory::AttachmentReference;
use crate::universal::{Message, TextContentPart, UserContent, UserContentPart};
use serde::Deserialize;

#[derive(Deserialize)]
struct BedrockTraceMessage {
    role: BedrockConversationRole,
    content: Vec<BedrockTraceContent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BedrockTraceContent {
    Image { image: BedrockTraceImage },
    Document { document: BedrockTraceDocument },
    Native(BedrockContentBlock),
}

#[derive(Deserialize)]
struct BedrockTraceImage {
    format: BedrockImageFormat,
    source: BedrockTraceSource,
}

#[derive(Deserialize)]
struct BedrockTraceDocument {
    format: String,
    name: Option<String>,
    source: BedrockTraceSource,
}

#[derive(Deserialize)]
struct BedrockTraceSource {
    bytes: BedrockTraceBytes,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BedrockTraceBytes {
    Base64(String),
    Attachment(AttachmentReference),
}

impl BedrockTraceBytes {
    fn into_value(self) -> Option<Value> {
        match self {
            Self::Base64(bytes) => Some(Value::String(bytes)),
            Self::Attachment(attachment) => serde_json::to_value(attachment).ok(),
        }
    }
}

impl BedrockTraceMessage {
    fn into_message(self) -> Option<Message> {
        if self.content.is_empty() {
            return None;
        }
        if self
            .content
            .iter()
            .all(|block| matches!(block, BedrockTraceContent::Native(_)))
        {
            let content = self
                .content
                .into_iter()
                .map(|block| match block {
                    BedrockTraceContent::Native(block) => Some(block),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            return <Message as TryFromLLM<BedrockMessage>>::try_from(BedrockMessage {
                role: self.role,
                content,
            })
            .ok();
        }
        if self.role != BedrockConversationRole::User {
            return None;
        }
        let content = self
            .content
            .into_iter()
            .map(|block| match block {
                BedrockTraceContent::Native(BedrockContentBlock::Text { text }) => {
                    Some(UserContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }))
                }
                BedrockTraceContent::Image { image } => Some(UserContentPart::Image {
                    image: image.source.bytes.into_value()?,
                    media_type: Some(
                        match image.format {
                            BedrockImageFormat::Png => "image/png",
                            BedrockImageFormat::Jpeg => "image/jpeg",
                            BedrockImageFormat::Gif => "image/gif",
                            BedrockImageFormat::Webp => "image/webp",
                        }
                        .to_string(),
                    ),
                    provider_options: None,
                }),
                BedrockTraceContent::Document { document } => Some(UserContentPart::File {
                    data: document.source.bytes.into_value()?,
                    filename: document.name,
                    media_type: match document.format.as_str() {
                        "pdf" => "application/pdf",
                        "csv" => "text/csv",
                        "doc" => "application/msword",
                        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                        "xls" => "application/vnd.ms-excel",
                        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                        "html" => "text/html",
                        "txt" => "text/plain",
                        "md" => "text/markdown",
                        _ => return None,
                    }
                    .to_string(),
                    provider_options: None,
                }),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Message::User {
            content: UserContent::Array(content),
        })
    }
}

#[derive(Deserialize)]
struct BedrockTraceOutput {
    message: Option<BedrockTraceMessage>,
    tool_result_message: Option<BedrockTraceMessage>,
}

pub(super) fn try_parse_bedrock_trace_for_import(data: &impl ImportValue) -> Option<Vec<Message>> {
    if let Some(messages) = try_parse_vec_or_single::<BedrockTraceMessage>(data) {
        return non_empty_messages(
            messages
                .into_iter()
                .map(BedrockTraceMessage::into_message)
                .collect::<Option<Vec<_>>>()?,
        );
    }
    let output = data.parse::<BedrockTraceOutput>()?;
    non_empty_messages(
        [output.message, output.tool_result_message]
            .into_iter()
            .flatten()
            .map(BedrockTraceMessage::into_message)
            .collect::<Option<Vec<_>>>()?,
    )
}

#[derive(Deserialize)]
struct StrandsSystemMetadata {
    system_prompt: Option<String>,
    system_prompt_content: Option<Vec<BedrockSystemContentBlock>>,
}

pub(super) fn try_system_message_from_strands_metadata(
    metadata: &impl ImportValue,
) -> Option<Message> {
    let metadata = metadata.parse::<StrandsSystemMetadata>()?;
    let text = match metadata
        .system_prompt_content
        .filter(|parts| !parts.is_empty())
    {
        Some(parts) => parts.into_iter().map(|part| part.text).collect::<String>(),
        None => metadata.system_prompt?,
    };
    if text.is_empty() {
        return None;
    }
    Some(Message::System {
        content: UserContent::String(text),
    })
}
