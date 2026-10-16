/*!
Bedrock Converse format conversions.

This module provides TryFromLLM trait implementations for converting between
AWS Bedrock's Converse API format and Lingua's universal message format.
*/

use crate::error::ConvertError;
use crate::import_parse::{
    try_convert_non_empty, try_parse, try_parse_vec_or_single, try_parsers_in_order, MessageParser,
};
use crate::providers::bedrock::request::{
    BedrockContentBlock, BedrockConversationRole, BedrockDocumentBlock, BedrockDocumentFormat,
    BedrockDocumentSource, BedrockImageBlock, BedrockImageFormat, BedrockImageSource,
    BedrockMessage, BedrockReasoningContentBlock, BedrockReasoningTextBlock,
    BedrockToolResultBlock, BedrockToolResultContent, BedrockToolUseBlock, ConverseRequest,
};
use crate::providers::bedrock::response::{
    BedrockOutputContentBlock, BedrockOutputMessage, BedrockOutputReasoningContentBlock,
    BedrockOutputReasoningTextBlock, ConverseResponse,
};
use crate::serde_json::{self, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolCallArguments,
    ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};
use crate::util::media::{is_remote_media_uri, parse_base64_data_url};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::collections::HashSet;

// ============================================================================
// Bedrock Message -> Universal Message
// ============================================================================

impl TryFromLLM<BedrockMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: BedrockMessage) -> Result<Self, Self::Error> {
        match msg.role {
            BedrockConversationRole::User => {
                let mut has_tool_results = false;
                let mut tool_results = Vec::new();
                let mut text_parts = Vec::new();

                for block in msg.content {
                    match block {
                        BedrockContentBlock::Text { text } => {
                            text_parts.push(text);
                        }
                        BedrockContentBlock::Document { .. } => {
                            return Err(ConvertError::UnsupportedMapping {
                                from: "Bedrock document block".to_string(),
                                to: "Lingua message",
                            });
                        }
                        BedrockContentBlock::ToolResult { tool_result } => {
                            has_tool_results = true;
                            let output = tool_result
                                .content
                                .into_iter()
                                .next()
                                .map(|c| match c {
                                    BedrockToolResultContent::Text { text } => Value::String(text),
                                    BedrockToolResultContent::Json { json } => json,
                                    BedrockToolResultContent::Image { .. } => {
                                        Value::String("[image]".to_string())
                                    }
                                })
                                .unwrap_or(Value::Null);

                            tool_results.push(ToolContentPart::ToolResult(ToolResultContentPart {
                                tool_call_id: tool_result.tool_use_id,
                                tool_name: String::new(),
                                output,
                                custom_tool_call: None,
                                caller: None,
                                provider_options: None,
                            }));
                        }
                        BedrockContentBlock::Image { .. } => {
                            // Skip images for now
                        }
                        BedrockContentBlock::ReasoningContent { .. } => {
                            // Reasoning content is only expected on assistant messages.
                        }
                        BedrockContentBlock::ToolUse { .. } => {
                            // Tool use shouldn't be in user messages
                        }
                    }
                }

                if has_tool_results {
                    Ok(Message::Tool {
                        content: tool_results,
                    })
                } else {
                    Ok(Message::User {
                        content: UserContent::String(text_parts.join("")),
                    })
                }
            }
            BedrockConversationRole::Assistant => {
                let mut content_parts = Vec::new();

                for block in msg.content {
                    match block {
                        BedrockContentBlock::Text { text } => {
                            content_parts.push(AssistantContentPart::Text(TextContentPart {
                                text,
                                encrypted_content: None,
                                cache_control: None,
                                provider_options: None,
                            }));
                        }
                        BedrockContentBlock::ToolUse { tool_use } => {
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_use.tool_use_id,
                                tool_name: tool_use.name,
                                arguments: ToolCallArguments::from(
                                    serde_json::to_string(&tool_use.input).unwrap_or_default(),
                                ),
                                encrypted_content: None,
                                provider_options: None,
                                status: None,
                                caller: None,
                                provider_executed: None,
                            });
                        }
                        BedrockContentBlock::ReasoningContent { reasoning_content } => {
                            if let Some((text, encrypted_content)) =
                                reasoning_content_to_universal(reasoning_content)
                            {
                                content_parts.push(AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                });
                            }
                        }
                        _ => {}
                    }
                }

                if content_parts.is_empty() {
                    content_parts.push(AssistantContentPart::Text(TextContentPart {
                        text: String::new(),
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }));
                }

                Ok(Message::Assistant {
                    content: AssistantContent::Array(content_parts),
                    id: None,
                })
            }
        }
    }
}

// ============================================================================
// Universal Message -> Bedrock Message
// ============================================================================

impl TryFromLLM<Message> for BedrockMessage {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let (role, content) = match message {
            Message::System { content } | Message::Developer { content } => {
                if content.has_audio() {
                    return Err(ConvertError::UnsupportedMapping {
                        from: "Lingua audio content".to_string(),
                        to: "Bedrock Converse system prompt",
                    });
                }

                let text = match content {
                    UserContent::String(s) => format!("System: {}", s),
                    UserContent::Array(parts) => {
                        let texts: Vec<String> = parts
                            .into_iter()
                            .filter_map(|p| match p {
                                UserContentPart::Text(t) => Some(t.text),
                                _ => None,
                            })
                            .collect();
                        format!("System: {}", texts.join(""))
                    }
                };
                (
                    BedrockConversationRole::User,
                    vec![BedrockContentBlock::Text { text }],
                )
            }
            Message::User { content } => {
                let blocks: Vec<BedrockContentBlock> = match content {
                    UserContent::String(s) => vec![BedrockContentBlock::Text { text: s }],
                    UserContent::Array(parts) => {
                        if parts
                            .iter()
                            .any(|part| matches!(part, UserContentPart::Audio { .. }))
                        {
                            return Err(ConvertError::UnsupportedMapping {
                                from: "Lingua audio content".to_string(),
                                to: "Bedrock Converse audio input",
                            });
                        }

                        parts
                            .into_iter()
                            .map(|p| match p {
                                UserContentPart::Text(t) => {
                                    Ok(Some(BedrockContentBlock::Text { text: t.text }))
                                }
                                UserContentPart::Image {
                                    image, media_type, ..
                                } => {
                                    if let Value::String(data) = image {
                                        let format = media_type
                                            .as_deref()
                                            .and_then(|mt| mt.strip_prefix("image/"))
                                            .map(|f| match f {
                                                "png" => BedrockImageFormat::Png,
                                                "gif" => BedrockImageFormat::Gif,
                                                "webp" => BedrockImageFormat::Webp,
                                                _ => BedrockImageFormat::Jpeg,
                                            })
                                            .unwrap_or(BedrockImageFormat::Jpeg);
                                        Ok(Some(BedrockContentBlock::Image {
                                            image: BedrockImageBlock {
                                                format,
                                                source: BedrockImageSource { bytes: data },
                                            },
                                        }))
                                    } else {
                                        Ok(None)
                                    }
                                }
                                UserContentPart::File {
                                    data,
                                    filename,
                                    media_type,
                                    ..
                                } => bedrock_document_from_file(
                                    data,
                                    filename.as_deref(),
                                    &media_type,
                                )
                                .map(Some),
                                _ => Ok(None),
                            })
                            .collect::<Result<Vec<_>, ConvertError>>()?
                            .into_iter()
                            .flatten()
                            .collect()
                    }
                };
                if blocks
                    .iter()
                    .any(|block| matches!(block, BedrockContentBlock::Document { .. }))
                    && !blocks
                        .iter()
                        .any(|block| matches!(block, BedrockContentBlock::Text { .. }))
                {
                    return Err(ConvertError::UnsupportedMapping {
                        from: "Lingua document content without a text block in the same message".to_string(),
                        to: "Bedrock Converse document input (requires a text block in the same message)",
                    });
                }
                (BedrockConversationRole::User, blocks)
            }
            Message::Assistant { content, .. } => {
                let blocks = match content {
                    AssistantContent::String(s) => vec![BedrockContentBlock::Text { text: s }],
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => {
                                Some(BedrockContentBlock::Text { text: t.text })
                            }
                            AssistantContentPart::Reasoning {
                                text,
                                encrypted_content,
                            } => Some(BedrockContentBlock::ReasoningContent {
                                reasoning_content: reasoning_content_from_universal(
                                    text,
                                    encrypted_content,
                                ),
                            }),
                            AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                ..
                            } => {
                                let input: Value = match arguments {
                                    ToolCallArguments::Valid(map) => serde_json::to_value(map)
                                        .unwrap_or(Value::Object(Default::default())),
                                    ToolCallArguments::Invalid(s)
                                    | ToolCallArguments::Custom(s) => serde_json::from_str(&s)
                                        .unwrap_or(Value::Object(Default::default())),
                                };
                                Some(BedrockContentBlock::ToolUse {
                                    tool_use: BedrockToolUseBlock {
                                        tool_use_id: tool_call_id,
                                        name: tool_name,
                                        input,
                                    },
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                };
                (BedrockConversationRole::Assistant, blocks)
            }
            Message::Tool { content } => {
                let blocks = tool_result_blocks_from_content(content)?;
                (BedrockConversationRole::User, blocks)
            }
            Message::AdditionalTools { .. } => {
                return Err(ConvertError::UnsupportedMapping {
                    from: "Message::AdditionalTools".to_string(),
                    to: "Bedrock Message",
                });
            }
        };

        Ok(BedrockMessage { role, content })
    }
}

fn tool_result_blocks_from_content(
    content: Vec<ToolContentPart>,
) -> Result<Vec<BedrockContentBlock>, ConvertError> {
    content
        .into_iter()
        .filter_map(|part| match part {
            ToolContentPart::ToolResult(result) => Some(result),
            ToolContentPart::ToolDiscoveryResult(_) => None,
        })
        .map(|result| {
            let content_text = match result.output {
                Value::String(s) => s,
                other => serde_json::to_string(&other).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: "tool_result.output".to_string(),
                        error: e.to_string(),
                    }
                })?,
            };
            Ok(BedrockContentBlock::ToolResult {
                tool_result: BedrockToolResultBlock {
                    tool_use_id: result.tool_call_id,
                    content: vec![BedrockToolResultContent::Text { text: content_text }],
                    status: None,
                },
            })
        })
        .collect()
}

fn is_tool_result_message(message: &BedrockMessage) -> bool {
    message.role == BedrockConversationRole::User
        && !message.content.is_empty()
        && message
            .content
            .iter()
            .all(|block| matches!(block, BedrockContentBlock::ToolResult { .. }))
}

fn is_discovery_only_assistant_message(message: &Message) -> bool {
    match message {
        Message::Assistant { content, .. } => match content {
            AssistantContent::Array(parts) => {
                !parts.is_empty()
                    && parts
                        .iter()
                        .all(|part| matches!(part, AssistantContentPart::ToolDiscoveryCall { .. }))
            }
            AssistantContent::String(_) => false,
        },
        _ => false,
    }
}

fn reasoning_content_to_universal(
    reasoning_content: BedrockReasoningContentBlock,
) -> Option<(String, Option<String>)> {
    match reasoning_content {
        BedrockReasoningContentBlock::ReasoningText { reasoning_text } => {
            Some((reasoning_text.text, reasoning_text.signature))
        }
        BedrockReasoningContentBlock::RedactedContent { redacted_content } => {
            Some((String::new(), Some(redacted_content)))
        }
    }
}

fn output_reasoning_content_to_universal(
    reasoning_content: BedrockOutputReasoningContentBlock,
) -> Option<(String, Option<String>)> {
    match reasoning_content {
        BedrockOutputReasoningContentBlock::ReasoningText { reasoning_text } => {
            Some((reasoning_text.text, reasoning_text.signature))
        }
        BedrockOutputReasoningContentBlock::RedactedContent { redacted_content } => {
            Some((String::new(), Some(redacted_content)))
        }
    }
}

fn reasoning_content_from_universal(
    text: String,
    encrypted_content: Option<String>,
) -> BedrockReasoningContentBlock {
    match (text.is_empty(), encrypted_content) {
        (true, Some(redacted_content)) => {
            BedrockReasoningContentBlock::RedactedContent { redacted_content }
        }
        (_, signature) => BedrockReasoningContentBlock::ReasoningText {
            reasoning_text: BedrockReasoningTextBlock { text, signature },
        },
    }
}

fn output_reasoning_content_from_universal(
    text: String,
    encrypted_content: Option<String>,
) -> BedrockOutputReasoningContentBlock {
    match (text.is_empty(), encrypted_content) {
        (true, Some(redacted_content)) => {
            BedrockOutputReasoningContentBlock::RedactedContent { redacted_content }
        }
        (_, signature) => BedrockOutputReasoningContentBlock::ReasoningText {
            reasoning_text: BedrockOutputReasoningTextBlock { text, signature },
        },
    }
}

// ============================================================================
// Convenience functions using trait implementations
// ============================================================================

/// Convert Bedrock ConverseRequest to universal messages.
pub fn bedrock_to_universal(request: &ConverseRequest) -> Result<Vec<Message>, ConvertError> {
    <Vec<Message> as TryFromLLM<Vec<BedrockMessage>>>::try_from(request.messages.clone())
}

/// Convert universal messages to Bedrock messages.
pub fn universal_to_bedrock_messages(
    messages: &[Message],
) -> Result<Vec<BedrockMessage>, ConvertError> {
    let mut bedrock_messages = Vec::new();

    for message in messages.iter().cloned() {
        if is_discovery_only_assistant_message(&message) {
            continue;
        }
        match message {
            Message::Tool { content } => {
                let blocks = tool_result_blocks_from_content(content)?;
                if blocks.is_empty() {
                    continue;
                }
                if let Some(last_message) = bedrock_messages.last_mut() {
                    if is_tool_result_message(last_message) {
                        last_message.content.extend(blocks);
                        continue;
                    }
                }

                bedrock_messages.push(BedrockMessage {
                    role: BedrockConversationRole::User,
                    content: blocks,
                });
            }
            other => {
                bedrock_messages.push(<BedrockMessage as TryFromLLM<Message>>::try_from(other)?);
            }
        }
    }

    dedupe_document_names(&mut bedrock_messages);
    Ok(bedrock_messages)
}

const BEDROCK_DOCUMENT_NAME_MAX_LEN: usize = 200;

const BEDROCK_DOCUMENT_FORMATS: [BedrockDocumentFormat; 9] = [
    BedrockDocumentFormat::Pdf,
    BedrockDocumentFormat::Doc,
    BedrockDocumentFormat::Docx,
    BedrockDocumentFormat::Xls,
    BedrockDocumentFormat::Xlsx,
    BedrockDocumentFormat::Txt,
    BedrockDocumentFormat::Csv,
    BedrockDocumentFormat::Html,
    BedrockDocumentFormat::Md,
];

fn bedrock_document_media_type(format: BedrockDocumentFormat) -> &'static str {
    match format {
        BedrockDocumentFormat::Pdf => "application/pdf",
        BedrockDocumentFormat::Doc => "application/msword",
        BedrockDocumentFormat::Docx => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        BedrockDocumentFormat::Xls => "application/vnd.ms-excel",
        BedrockDocumentFormat::Xlsx => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        BedrockDocumentFormat::Txt => "text/plain",
        BedrockDocumentFormat::Csv => "text/csv",
        BedrockDocumentFormat::Html => "text/html",
        BedrockDocumentFormat::Md => "text/markdown",
    }
}

fn is_text_document(format: BedrockDocumentFormat) -> bool {
    matches!(
        format,
        BedrockDocumentFormat::Txt
            | BedrockDocumentFormat::Csv
            | BedrockDocumentFormat::Html
            | BedrockDocumentFormat::Md
    )
}

fn unsupported_file_mapping() -> ConvertError {
    ConvertError::UnsupportedMapping {
        from: "Lingua file content".to_string(),
        to: "Bedrock Converse file input",
    }
}

/// Text documents carry raw text in universal `File.data`; binary documents carry base64.
fn bedrock_document_from_file(
    data: Value,
    filename: Option<&str>,
    media_type: &str,
) -> Result<BedrockContentBlock, ConvertError> {
    let Value::String(data) = data else {
        return Err(unsupported_file_mapping());
    };

    let format = BEDROCK_DOCUMENT_FORMATS
        .into_iter()
        .find(|format| bedrock_document_media_type(*format) == media_type)
        .ok_or_else(unsupported_file_mapping)?;
    let data = if !is_text_document(format) && data.starts_with("data:") {
        let block = parse_base64_data_url(&data).ok_or_else(unsupported_file_mapping)?;
        if block.media_type != media_type {
            return Err(unsupported_file_mapping());
        }
        block.data
    } else {
        data
    };
    let is_remote = if is_text_document(format) {
        is_remote_media_uri(&data)
    } else {
        url::Url::parse(&data).is_ok()
    };
    if is_remote {
        return Err(unsupported_file_mapping());
    }
    let bytes = if is_text_document(format) {
        BASE64_STANDARD.encode(data.as_bytes())
    } else {
        data
    };

    Ok(BedrockContentBlock::Document {
        document: BedrockDocumentBlock {
            format,
            name: bedrock_document_name(filename),
            source: BedrockDocumentSource { bytes },
        },
    })
}

/// Bedrock document names allow only ASCII alphanumerics, single whitespace,
/// hyphens, parentheses, and square brackets, up to 200 characters.
fn bedrock_document_name(filename: Option<&str>) -> String {
    let mut name = String::new();
    for c in filename.unwrap_or_default().chars() {
        let c = if c.is_ascii_alphanumeric() || matches!(c, '-' | '(' | ')' | '[' | ']') {
            c
        } else if c.is_whitespace() {
            ' '
        } else {
            '-'
        };
        if c == ' ' && (name.is_empty() || name.ends_with(' ')) {
            continue;
        }
        name.push(c);
    }
    let name: String = name.chars().take(BEDROCK_DOCUMENT_NAME_MAX_LEN).collect();
    let name = name.trim_end();
    if name.is_empty() {
        "document".to_string()
    } else {
        name.to_string()
    }
}

/// Bedrock rejects requests that contain duplicate document names.
fn dedupe_document_names(messages: &mut [BedrockMessage]) {
    let mut seen = HashSet::new();
    for block in messages.iter_mut().flat_map(|m| m.content.iter_mut()) {
        let BedrockContentBlock::Document { document } = block else {
            continue;
        };
        let base = document.name.clone();
        let mut index = 1;
        while !seen.insert(document.name.clone()) {
            index += 1;
            let suffix = format!(" ({index})");
            let stem: String = base
                .chars()
                .take(BEDROCK_DOCUMENT_NAME_MAX_LEN - suffix.len())
                .collect();
            document.name = format!("{}{suffix}", stem.trim_end());
        }
    }
}

/// Convert universal messages to Bedrock Converse format as JSON Value.
///
/// This serializes the converted BedrockMessage structs to JSON for use
/// in contexts where a Value is needed (e.g., when building full requests).
pub fn universal_to_bedrock(messages: &[Message]) -> Result<Value, ConvertError> {
    let bedrock_messages = universal_to_bedrock_messages(messages)?;
    serde_json::to_value(bedrock_messages).map_err(|e| ConvertError::JsonSerializationFailed {
        field: "messages".to_string(),
        error: e.to_string(),
    })
}

fn try_messages_from_bedrock_messages(messages: Vec<BedrockMessage>) -> Option<Vec<Message>> {
    try_convert_non_empty(messages)
}

fn try_message_from_bedrock_output_message(message: BedrockOutputMessage) -> Option<Vec<Message>> {
    if message.role != "assistant" {
        return None;
    }

    let message = <Message as TryFromLLM<BedrockOutputMessage>>::try_from(message).ok()?;
    Some(vec![message])
}

fn try_messages_from_bedrock_output_messages(
    output_messages: Vec<BedrockOutputMessage>,
) -> Option<Vec<Message>> {
    if output_messages
        .iter()
        .any(|message| message.role != "assistant")
    {
        return None;
    }

    try_convert_non_empty(output_messages)
}

fn try_parse_bedrock_message_for_import(data: &Value) -> Option<Vec<Message>> {
    let messages = try_parse_vec_or_single::<BedrockMessage>(data)?;
    try_messages_from_bedrock_messages(messages)
}

fn try_parse_bedrock_request_for_import(data: &Value) -> Option<Vec<Message>> {
    let request = try_parse::<ConverseRequest>(data)?;
    try_messages_from_bedrock_messages(request.messages)
}

fn try_parse_bedrock_output_message_for_import(data: &Value) -> Option<Vec<Message>> {
    let output_messages = try_parse_vec_or_single::<BedrockOutputMessage>(data)?;
    if output_messages.len() == 1 {
        return try_message_from_bedrock_output_message(output_messages.into_iter().next()?);
    }
    try_messages_from_bedrock_output_messages(output_messages)
}

fn try_parse_bedrock_response_for_import(data: &Value) -> Option<Vec<Message>> {
    let response = try_parse::<ConverseResponse>(data)?;
    try_message_from_bedrock_output_message(response.output.message)
}

pub(crate) fn try_parse_bedrock_for_import(data: &Value) -> Option<Vec<Message>> {
    const PARSERS: &[MessageParser] = &[
        try_parse_bedrock_message_for_import,
        try_parse_bedrock_request_for_import,
        try_parse_bedrock_output_message_for_import,
        try_parse_bedrock_response_for_import,
    ];
    try_parsers_in_order(data, PARSERS)
}

// ============================================================================
// BedrockOutputMessage (Response) -> Universal Message
// ============================================================================

impl TryFromLLM<BedrockOutputMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: BedrockOutputMessage) -> Result<Self, Self::Error> {
        // Output messages are always from the assistant
        let mut content_parts = Vec::new();

        for block in msg.content {
            match block {
                BedrockOutputContentBlock::Text { text } => {
                    content_parts.push(AssistantContentPart::Text(TextContentPart {
                        text,
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }));
                }
                BedrockOutputContentBlock::ToolUse { tool_use } => {
                    content_parts.push(AssistantContentPart::ToolCall {
                        tool_call_id: tool_use.tool_use_id,
                        tool_name: tool_use.name,
                        arguments: ToolCallArguments::from(
                            serde_json::to_string(&tool_use.input).unwrap_or_default(),
                        ),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: None,
                    });
                }
                BedrockOutputContentBlock::ReasoningContent { reasoning_content } => {
                    if let Some((text, encrypted_content)) =
                        output_reasoning_content_to_universal(reasoning_content)
                    {
                        content_parts.push(AssistantContentPart::Reasoning {
                            text,
                            encrypted_content,
                        });
                    }
                }
            }
        }

        if content_parts.is_empty() {
            content_parts.push(AssistantContentPart::Text(TextContentPart {
                text: String::new(),
                encrypted_content: None,
                cache_control: None,
                provider_options: None,
            }));
        }

        Ok(Message::Assistant {
            content: AssistantContent::Array(content_parts),
            id: None,
        })
    }
}

// ============================================================================
// Universal Message -> BedrockOutputMessage (Response)
// ============================================================================

impl TryFromLLM<Message> for BedrockOutputMessage {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::Assistant { content, .. } => {
                let blocks = match content {
                    AssistantContent::String(s) => {
                        vec![BedrockOutputContentBlock::Text { text: s }]
                    }
                    AssistantContent::Array(parts) => parts
                        .into_iter()
                        .filter_map(|p| match p {
                            AssistantContentPart::Text(t) => {
                                Some(BedrockOutputContentBlock::Text { text: t.text })
                            }
                            AssistantContentPart::Reasoning {
                                text,
                                encrypted_content,
                            } => Some(BedrockOutputContentBlock::ReasoningContent {
                                reasoning_content: output_reasoning_content_from_universal(
                                    text,
                                    encrypted_content,
                                ),
                            }),
                            AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                ..
                            } => {
                                use crate::providers::bedrock::response::BedrockOutputToolUse;
                                let input: Value = match arguments {
                                    ToolCallArguments::Valid(map) => serde_json::to_value(map)
                                        .unwrap_or(Value::Object(Default::default())),
                                    ToolCallArguments::Invalid(s)
                                    | ToolCallArguments::Custom(s) => serde_json::from_str(&s)
                                        .unwrap_or(Value::Object(Default::default())),
                                };
                                Some(BedrockOutputContentBlock::ToolUse {
                                    tool_use: BedrockOutputToolUse {
                                        tool_use_id: tool_call_id,
                                        name: tool_name,
                                        input,
                                    },
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                };
                Ok(BedrockOutputMessage {
                    role: "assistant".to_string(),
                    content: blocks,
                })
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: "Only Assistant messages can be converted to BedrockOutputMessage"
                    .to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;
    use crate::universal::message::{
        AudioFormat, ToolDiscoveryResultContentPart, ToolDiscoveryResultItem,
    };

    #[test]
    fn test_bedrock_message_to_universal_user() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::User,
            content: vec![BedrockContentBlock::Text {
                text: "Hello".to_string(),
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
        match message {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_bedrock_message_to_universal_assistant() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::Assistant,
            content: vec![BedrockContentBlock::Text {
                text: "Hi there!".to_string(),
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::Text(t) => assert_eq!(t.text, "Hi there!"),
                        _ => panic!("Expected text part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_bedrock_message_to_universal_tool_use() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::Assistant,
            content: vec![BedrockContentBlock::ToolUse {
                tool_use: BedrockToolUseBlock {
                    tool_use_id: "tool_123".to_string(),
                    name: "get_weather".to_string(),
                    input: json!({"location": "SF"}),
                },
            }],
        };

        let message = <Message as TryFromLLM<BedrockMessage>>::try_from(msg).unwrap();
        match message {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_call_id,
                            tool_name,
                            ..
                        } => {
                            assert_eq!(tool_call_id, "tool_123");
                            assert_eq!(tool_name, "get_weather");
                        }
                        _ => panic!("Expected tool call part"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_message_to_bedrock_user() {
        let message = Message::User {
            content: UserContent::String("Hello".to_string()),
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::User);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_message_to_bedrock_rejects_file_content() {
        let message = Message::User {
            content: UserContent::Array(vec![UserContentPart::File {
                data: Value::String("https://example.com/sample.mp3".to_string()),
                filename: Some("sample.mp3".to_string()),
                media_type: "audio/mp3".to_string(),
                provider_options: None,
            }]),
        };

        let error = <BedrockMessage as TryFromLLM<Message>>::try_from(message)
            .expect_err("Bedrock must not silently drop file content");
        assert!(matches!(error, ConvertError::UnsupportedMapping { .. }));
    }

    fn file_part(data: Value, filename: Option<&str>, media_type: &str) -> UserContentPart {
        UserContentPart::File {
            data,
            filename: filename.map(str::to_string),
            media_type: media_type.to_string(),
            provider_options: None,
        }
    }

    fn file_to_bedrock(part: UserContentPart) -> Result<BedrockMessage, ConvertError> {
        <BedrockMessage as TryFromLLM<Message>>::try_from(Message::User {
            content: UserContent::Array(vec![
                UserContentPart::Text(TextContentPart {
                    text: "Read the document".into(),
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                }),
                part,
            ]),
        })
    }

    fn document(format: BedrockDocumentFormat, name: &str, bytes: String) -> BedrockContentBlock {
        BedrockContentBlock::Document {
            document: BedrockDocumentBlock {
                format,
                name: name.to_string(),
                source: BedrockDocumentSource { bytes },
            },
        }
    }

    #[test]
    fn test_message_to_bedrock_file_becomes_document() {
        let docx = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        for (data, filename, media_type, expected) in [
            (
                "JVBERi0xLjQK",
                "receipt.pdf",
                "application/pdf",
                document(
                    BedrockDocumentFormat::Pdf,
                    "receipt-pdf",
                    "JVBERi0xLjQK".into(),
                ),
            ),
            (
                "UEsDBBQ=",
                "report.docx",
                docx,
                document(
                    BedrockDocumentFormat::Docx,
                    "report-docx",
                    "UEsDBBQ=".into(),
                ),
            ),
            (
                "Total: 45 DKK",
                "notes.txt",
                "text/plain",
                document(
                    BedrockDocumentFormat::Txt,
                    "notes-txt",
                    BASE64_STANDARD.encode("Total: 45 DKK"),
                ),
            ),
        ] {
            let msg = file_to_bedrock(file_part(data.into(), Some(filename), media_type)).unwrap();
            assert_eq!(msg.content[1..], [expected]);
        }
    }

    #[test]
    fn test_message_to_bedrock_rejects_unsupported_files() {
        for (data, media_type) in [
            (json!("https://example.com/report.pdf"), "application/pdf"),
            (json!("s3://bucket/report.pdf"), "application/pdf"),
            (json!("https://example.com/notes.txt"), "text/plain"),
            (json!("UklGRg=="), "audio/wav"),
            (json!("iVBORw0KGgo="), "image/png"),
            (json!({"bytes": [1, 2, 3]}), "application/pdf"),
        ] {
            match file_to_bedrock(file_part(data, Some("file"), media_type)) {
                Err(ConvertError::UnsupportedMapping { from, to }) => {
                    assert_eq!(from, "Lingua file content");
                    assert_eq!(to, "Bedrock Converse file input");
                }
                other => panic!("expected unsupported mapping for {media_type}, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_bedrock_document_only_message_is_rejected() {
        let message = Message::User {
            content: UserContent::Array(vec![file_part(
                json!("JVBERi0xLjQK"),
                Some("receipt.pdf"),
                "application/pdf",
            )]),
        };
        let error = <BedrockMessage as TryFromLLM<Message>>::try_from(message.clone()).unwrap_err();
        assert!(matches!(error, ConvertError::UnsupportedMapping { .. }));
        assert!(error.to_string().contains("text"));
        let messages = vec![
            Message::User {
                content: UserContent::String("Read the receipt".into()),
            },
            message,
        ];
        assert!(universal_to_bedrock_messages(&messages).is_err());
    }

    #[test]
    fn test_bedrock_binary_document_data_url() {
        let block = bedrock_document_from_file(
            json!("data:application/pdf;base64,JVBERi0xLjQK"),
            Some("receipt.pdf"),
            "application/pdf",
        )
        .unwrap();
        assert_eq!(
            block,
            document(
                BedrockDocumentFormat::Pdf,
                "receipt-pdf",
                "JVBERi0xLjQK".into()
            )
        );
    }

    #[test]
    fn test_bedrock_binary_document_rejects_invalid_data_urls() {
        for data in [
            "data:image/png;base64,iVBORw0KGgo=",
            "data:application/pdf,raw-pdf",
            "data:application/pdf;base64,",
        ] {
            assert!(matches!(
                bedrock_document_from_file(json!(data), Some("receipt.pdf"), "application/pdf"),
                Err(ConvertError::UnsupportedMapping { .. })
            ));
        }
    }

    #[test]
    fn test_bedrock_document_name_sanitization() {
        assert_eq!(bedrock_document_name(None), "document");
        assert_eq!(bedrock_document_name(Some("   ")), "document");
        assert_eq!(
            bedrock_document_name(Some("my_receipt.pdf")),
            "my-receipt-pdf"
        );
        assert_eq!(
            bedrock_document_name(Some("Q3  report\t(final) [v2].pdf")),
            "Q3 report (final) [v2]-pdf"
        );
        assert_eq!(bedrock_document_name(Some("reçu.pdf")), "re-u-pdf");
        assert_eq!(
            bedrock_document_name(Some(&"a".repeat(250))).len(),
            BEDROCK_DOCUMENT_NAME_MAX_LEN
        );
    }

    #[test]
    fn test_universal_to_bedrock_dedupes_document_names_across_messages() {
        let pdf =
            |filename: Option<&str>| file_part(json!("JVBERi0xLjQK"), filename, "application/pdf");
        let long_name = "a".repeat(BEDROCK_DOCUMENT_NAME_MAX_LEN);
        let messages = vec![
            Message::User {
                content: UserContent::Array(vec![
                    UserContentPart::Text(TextContentPart {
                        text: "Read these documents".into(),
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }),
                    pdf(None),
                    pdf(None),
                    pdf(Some(&long_name)),
                ]),
            },
            Message::User {
                content: UserContent::Array(vec![
                    UserContentPart::Text(TextContentPart {
                        text: "Read these too".into(),
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }),
                    pdf(None),
                    pdf(Some(&long_name)),
                ]),
            },
        ];

        let names: Vec<String> = universal_to_bedrock_messages(&messages)
            .unwrap()
            .into_iter()
            .flat_map(|m| m.content)
            .filter_map(|block| match block {
                BedrockContentBlock::Document { document } => Some(document.name),
                _ => None,
            })
            .collect();

        assert_eq!(
            names,
            vec![
                "document".to_string(),
                "document (2)".to_string(),
                long_name.clone(),
                "document (3)".to_string(),
                format!("{} (2)", &long_name[..BEDROCK_DOCUMENT_NAME_MAX_LEN - 4]),
            ]
        );
    }

    #[test]
    fn test_bedrock_document_to_universal_is_rejected() {
        let msg = BedrockMessage {
            role: BedrockConversationRole::User,
            content: vec![document(
                BedrockDocumentFormat::Pdf,
                "receipt",
                "JVBERi0xLjQK".into(),
            )],
        };
        assert!(matches!(
            <Message as TryFromLLM<BedrockMessage>>::try_from(msg),
            Err(ConvertError::UnsupportedMapping { .. })
        ));
    }

    #[test]
    fn test_message_to_bedrock_rejects_audio_content() {
        let message = Message::User {
            content: UserContent::Array(vec![
                UserContentPart::Text(TextContentPart {
                    text: "Transcribe this audio clip.".to_string(),
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                }),
                UserContentPart::Audio {
                    data: "UklGRg==".to_string(),
                    format: AudioFormat::Wav,
                },
            ]),
        };

        let error = <BedrockMessage as TryFromLLM<Message>>::try_from(message)
            .expect_err("Bedrock must not silently drop audio content");
        match error {
            ConvertError::UnsupportedMapping { from, to } => {
                assert_eq!(from, "Lingua audio content");
                assert_eq!(to, "Bedrock Converse audio input");
            }
            other => panic!("expected unsupported mapping error, got {other:?}"),
        }
    }

    #[test]
    fn test_system_audio_is_rejected() {
        let message = Message::System {
            content: UserContent::Array(vec![UserContentPart::Audio {
                data: "UklGRg==".to_string(),
                format: AudioFormat::Wav,
            }]),
        };

        let error = <BedrockMessage as TryFromLLM<Message>>::try_from(message)
            .expect_err("Bedrock must not silently drop system audio");
        assert!(matches!(error, ConvertError::UnsupportedMapping { .. }));
    }

    #[test]
    fn test_message_to_bedrock_assistant() {
        let message = Message::Assistant {
            content: AssistantContent::String("Hi there!".to_string()),
            id: None,
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::Assistant);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::Text { text } => assert_eq!(text, "Hi there!"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_message_to_bedrock_tool_call() {
        let message = Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "tool_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                encrypted_content: None,
                provider_options: None,
                status: None,
                caller: None,
                provider_executed: None,
            }]),
            id: None,
        };

        let msg = <BedrockMessage as TryFromLLM<Message>>::try_from(message).unwrap();
        assert_eq!(msg.role, BedrockConversationRole::Assistant);
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0] {
            BedrockContentBlock::ToolUse { tool_use } => {
                assert_eq!(tool_use.tool_use_id, "tool_123");
                assert_eq!(tool_use.name, "get_weather");
            }
            _ => panic!("Expected tool use block"),
        }
    }

    #[test]
    fn test_bedrock_to_universal_simple() {
        let request = ConverseRequest {
            model_id: "test-model".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::User,
                content: vec![BedrockContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let messages = bedrock_to_universal(&request).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::User { content } => match content {
                UserContent::String(s) => assert_eq!(s, "Hello"),
                _ => panic!("Expected string content"),
            },
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_bedrock_to_universal_with_tool_use() {
        let request = ConverseRequest {
            model_id: "test-model".to_string(),
            messages: vec![BedrockMessage {
                role: BedrockConversationRole::Assistant,
                content: vec![BedrockContentBlock::ToolUse {
                    tool_use: BedrockToolUseBlock {
                        tool_use_id: "tool_123".to_string(),
                        name: "get_weather".to_string(),
                        input: json!({"location": "SF"}),
                    },
                }],
            }],
            system: None,
            inference_config: None,
            tool_config: None,
            guardrail_config: None,
            additional_model_request_fields: None,
            additional_model_response_field_paths: None,
            prompt_variables: None,
        };

        let messages = bedrock_to_universal(&request).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Assistant { content, .. } => match content {
                AssistantContent::Array(parts) => {
                    assert_eq!(parts.len(), 1);
                    match &parts[0] {
                        AssistantContentPart::ToolCall {
                            tool_call_id,
                            tool_name,
                            ..
                        } => {
                            assert_eq!(tool_call_id, "tool_123");
                            assert_eq!(tool_name, "get_weather");
                        }
                        _ => panic!("Expected tool call"),
                    }
                }
                _ => panic!("Expected array content"),
            },
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_universal_to_bedrock_simple() {
        let messages = vec![Message::User {
            content: UserContent::String("Hello".to_string()),
        }];

        let result = universal_to_bedrock(&messages).unwrap();
        let expected = json!([{
            "role": "user",
            "content": [{"text": "Hello"}]
        }]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_universal_to_bedrock_with_tool_call() {
        let messages = vec![Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                tool_call_id: "tool_123".to_string(),
                tool_name: "get_weather".to_string(),
                arguments: ToolCallArguments::from(r#"{"location":"SF"}"#.to_string()),
                encrypted_content: None,
                provider_options: None,
                status: None,
                caller: None,
                provider_executed: None,
            }]),
            id: None,
        }];

        let result = universal_to_bedrock(&messages).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["role"], "assistant");
        assert!(arr[0]["content"][0].get("toolUse").is_some());
    }

    #[test]
    fn test_universal_to_bedrock_drops_discovery_only_history() {
        let messages = vec![
            Message::User {
                content: UserContent::String("Find the available tools.".to_string()),
            },
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolDiscoveryCall {
                    tool_call_id: "call_tool_search_123".to_string(),
                    discovery_tool_name: "tool_search".to_string(),
                    query: Some("repo:example symbol".to_string()),
                    arguments: None,
                    status: Some("completed".to_string()),
                    execution: Some("client".to_string()),
                    provider_options: None,
                }]),
                id: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolDiscoveryResult(
                    ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![ToolDiscoveryResultItem {
                            tool_name: "search_code".to_string(),
                            tool: None,
                            provider_options: None,
                        }],
                        status: Some("completed".to_string()),
                        execution: Some("client".to_string()),
                        provider_options: None,
                    },
                )],
            },
            Message::User {
                content: UserContent::String("Use the discovered tool list.".to_string()),
            },
        ];

        let result = universal_to_bedrock_messages(&messages).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, BedrockConversationRole::User);
        assert_eq!(result[1].role, BedrockConversationRole::User);
        assert!(result.iter().all(|message| !message.content.is_empty()));
    }

    #[test]
    fn test_universal_to_bedrock_preserves_redacted_reasoning() {
        let messages = vec![Message::Assistant {
            content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                text: String::new(),
                encrypted_content: Some("redacted-by-provider".to_string()),
            }]),
            id: None,
        }];

        let result = universal_to_bedrock(&messages).unwrap();
        assert_eq!(
            result,
            json!([{
                "role": "assistant",
                "content": [{
                    "reasoningContent": {
                        "redactedContent": "redacted-by-provider"
                    }
                }]
            }])
        );
    }

    #[test]
    fn test_bedrock_output_message_redacted_reasoning_roundtrip() {
        let output = BedrockOutputMessage {
            role: "assistant".to_string(),
            content: vec![BedrockOutputContentBlock::ReasoningContent {
                reasoning_content: BedrockOutputReasoningContentBlock::RedactedContent {
                    redacted_content: "redacted-by-provider".to_string(),
                },
            }],
        };

        let message = <Message as TryFromLLM<BedrockOutputMessage>>::try_from(output).unwrap();
        let roundtrip = <BedrockOutputMessage as TryFromLLM<Message>>::try_from(message).unwrap();

        assert_eq!(
            roundtrip.content,
            vec![BedrockOutputContentBlock::ReasoningContent {
                reasoning_content: BedrockOutputReasoningContentBlock::RedactedContent {
                    redacted_content: "redacted-by-provider".to_string(),
                },
            }]
        );
    }

    #[test]
    fn test_universal_to_bedrock_coalesces_parallel_tool_results() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![
                    AssistantContentPart::ToolCall {
                        tool_call_id: "call_sf".to_string(),
                        tool_name: "get_weather".to_string(),
                        arguments: ToolCallArguments::from(
                            r#"{"location":"San Francisco, CA"}"#.to_string(),
                        ),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: None,
                    },
                    AssistantContentPart::ToolCall {
                        tool_call_id: "call_nyc".to_string(),
                        tool_name: "get_weather".to_string(),
                        arguments: ToolCallArguments::from(
                            r#"{"location":"New York, NY"}"#.to_string(),
                        ),
                        encrypted_content: None,
                        provider_options: None,
                        status: None,
                        caller: None,
                        provider_executed: None,
                    },
                ]),
                id: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "call_sf".to_string(),
                    tool_name: "get_weather".to_string(),
                    output: Value::String("65°F and sunny.".to_string()),
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })],
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "call_nyc".to_string(),
                    tool_name: "get_weather".to_string(),
                    output: Value::String("45°F and cloudy.".to_string()),
                    custom_tool_call: None,
                    caller: None,
                    provider_options: None,
                })],
            },
        ];

        let result = universal_to_bedrock_messages(&messages).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].role, BedrockConversationRole::User);
        assert_eq!(result[1].content.len(), 2);

        let tool_use_ids: Vec<String> = result[1]
            .content
            .iter()
            .map(|block| match block {
                BedrockContentBlock::ToolResult { tool_result } => tool_result.tool_use_id.clone(),
                _ => panic!("Expected tool result block"),
            })
            .collect();
        assert_eq!(tool_use_ids, vec!["call_sf", "call_nyc"]);
    }
}
