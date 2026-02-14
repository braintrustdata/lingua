use std::convert::TryFrom;

use crate::capabilities::ProviderFormat;
use crate::error::ConvertError;
use crate::providers::anthropic::generated;
use crate::providers::anthropic::generated::{
    CustomTool, JsonOutputFormat, JsonOutputFormatType, Tool, ToolChoice, ToolChoiceType,
};
use crate::serde_json;
use crate::serde_json::{json, Value};
use crate::universal::request::{
    JsonSchemaConfig, ResponseFormatConfig, ResponseFormatType, ToolChoiceConfig, ToolChoiceMode,
};
use crate::universal::tools::{BuiltinToolProvider, UniversalTool, UniversalToolType};
use crate::universal::{
    convert::TryFromLLM, message::ProviderOptions, AssistantContent, AssistantContentPart, Message,
    TextContentPart, ToolCallArguments, ToolContentPart, ToolResultContentPart, UserContent,
    UserContentPart,
};
use crate::util::media::parse_base64_data_url;

impl TryFromLLM<generated::InputMessage> for Message {
    type Error = ConvertError;

    fn try_from(input_msg: generated::InputMessage) -> Result<Self, Self::Error> {
        // Check if this is a user message that contains only tool results
        // If so, convert it to a Tool message instead
        if let generated::MessageRole::User = input_msg.role {
            if let generated::MessageContent::InputContentBlockArray(blocks) = &input_msg.content {
                // Check if all blocks are tool results
                let all_tool_results = blocks.iter().all(|block| {
                    matches!(
                        block.input_content_block_type,
                        generated::InputContentBlockType::ToolResult
                    )
                });

                let has_tool_results = blocks.iter().any(|block| {
                    matches!(
                        block.input_content_block_type,
                        generated::InputContentBlockType::ToolResult
                    )
                });

                // If we have tool results and no other content, convert to Tool message
                if has_tool_results && all_tool_results {
                    // Take ownership of the content for conversion
                    if let generated::MessageContent::InputContentBlockArray(owned_blocks) =
                        input_msg.content
                    {
                        let mut tool_content_parts = Vec::new();

                        for block in owned_blocks {
                            if matches!(
                                block.input_content_block_type,
                                generated::InputContentBlockType::ToolResult
                            ) {
                                if let (Some(tool_use_id), Some(content)) =
                                    (block.tool_use_id, block.content)
                                {
                                    let output = match content {
                                        generated::Content::String(s) => {
                                            serde_json::Value::String(s)
                                        }
                                        generated::Content::BlockArray(blocks) => {
                                            serde_json::to_value(blocks).map_err(|e| {
                                                ConvertError::JsonSerializationFailed {
                                                    field: "BlockArray".to_string(),
                                                    error: e.to_string(),
                                                }
                                            })?
                                        }
                                        generated::Content::RequestWebSearchToolResultError(
                                            err,
                                        ) => serde_json::to_value(err).map_err(|e| {
                                            ConvertError::JsonSerializationFailed {
                                                field: "RequestWebSearchToolResultError"
                                                    .to_string(),
                                                error: e.to_string(),
                                            }
                                        })?,
                                    };

                                    tool_content_parts.push(ToolContentPart::ToolResult(
                                        ToolResultContentPart {
                                            tool_call_id: tool_use_id,
                                            tool_name: String::new(), // Anthropic doesn't provide tool name in results
                                            output,
                                            provider_options: None,
                                        },
                                    ));
                                }
                            }
                        }

                        return Ok(Message::Tool {
                            content: tool_content_parts,
                        });
                    }
                }
            }
        }

        match input_msg.role {
            generated::MessageRole::User => {
                let content = match input_msg.content {
                    generated::MessageContent::String(text) => UserContent::String(text),
                    generated::MessageContent::InputContentBlockArray(blocks) => {
                        let mut content_parts = Vec::new();

                        for block in blocks {
                            match block.input_content_block_type {
                                generated::InputContentBlockType::Text => {
                                    if let Some(text) = block.text {
                                        content_parts.push(UserContentPart::Text(
                                            TextContentPart {
                                                text,
                                                provider_options: None,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::Image => {
                                    if let Some(source) = block.source {
                                        // Convert Anthropic image source to universal format
                                        match source {
                                            generated::Source::SourceSource(purple_source) => {
                                                if let Some(data) = purple_source.data {
                                                    let media_type = purple_source.media_type.map(|mt| match mt {
                                                        generated::FluffyMediaType::ImageJpeg => "image/jpeg".to_string(),
                                                        generated::FluffyMediaType::ImagePng => "image/png".to_string(),
                                                        generated::FluffyMediaType::ImageGif => "image/gif".to_string(),
                                                        generated::FluffyMediaType::ImageWebp => "image/webp".to_string(),
                                                        generated::FluffyMediaType::ApplicationPdf => "application/pdf".to_string(),
                                                        generated::FluffyMediaType::TextPlain => "text/plain".to_string(),
                                                    });
                                                    content_parts.push(UserContentPart::Image {
                                                        image: serde_json::Value::String(data),
                                                        media_type,
                                                        provider_options: None,
                                                    });
                                                }
                                            }
                                            _ => {
                                                // Skip other source types for now
                                                continue;
                                            }
                                        }
                                    }
                                }
                                generated::InputContentBlockType::ToolResult => {
                                    // Skip tool results for now - should be handled properly
                                    continue;
                                }
                                generated::InputContentBlockType::Document => {
                                    // Map document to File with provider_options for title/context
                                    if let Some(source) = &block.source {
                                        let mut opts = serde_json::Map::new();
                                        // Store document-specific fields in provider_options
                                        opts.insert(
                                            "anthropic_type".into(),
                                            serde_json::Value::String("document".to_string()),
                                        );
                                        if let Some(title) = &block.title {
                                            opts.insert(
                                                "title".into(),
                                                serde_json::Value::String(title.clone()),
                                            );
                                        }
                                        if let Some(context) = &block.context {
                                            opts.insert(
                                                "context".into(),
                                                serde_json::Value::String(context.clone()),
                                            );
                                        }

                                        // Extract data and media_type from source
                                        match source {
                                            generated::Source::SourceSource(s) => {
                                                let media_type = s.media_type.as_ref().map(|mt| {
                                                    match mt {
                                                        generated::FluffyMediaType::ImageJpeg => {
                                                            "image/jpeg".to_string()
                                                        }
                                                        generated::FluffyMediaType::ImagePng => {
                                                            "image/png".to_string()
                                                        }
                                                        generated::FluffyMediaType::ImageGif => {
                                                            "image/gif".to_string()
                                                        }
                                                        generated::FluffyMediaType::ImageWebp => {
                                                            "image/webp".to_string()
                                                        }
                                                        generated::FluffyMediaType::ApplicationPdf => {
                                                            "application/pdf".to_string()
                                                        }
                                                        generated::FluffyMediaType::TextPlain => {
                                                            "text/plain".to_string()
                                                        }
                                                    }
                                                });
                                                content_parts.push(UserContentPart::File {
                                                    data: s
                                                        .data
                                                        .clone()
                                                        .map(serde_json::Value::String)
                                                        .unwrap_or(serde_json::Value::Null),
                                                    filename: None,
                                                    media_type: media_type.unwrap_or_else(|| {
                                                        "text/plain".to_string()
                                                    }),
                                                    provider_options: Some(ProviderOptions {
                                                        options: opts,
                                                    }),
                                                });
                                            }
                                            _ => {
                                                // Skip other source types
                                                continue;
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            UserContent::String(String::new())
                        } else {
                            UserContent::Array(content_parts)
                        }
                    }
                };

                Ok(Message::User { content })
            }
            generated::MessageRole::Assistant => {
                let content = match input_msg.content {
                    generated::MessageContent::String(text) => AssistantContent::String(text),
                    generated::MessageContent::InputContentBlockArray(blocks) => {
                        let mut content_parts = Vec::new();

                        for block in blocks {
                            match block.input_content_block_type {
                                generated::InputContentBlockType::Text => {
                                    if let Some(text) = block.text {
                                        // Preserve citations in provider_options for roundtrip
                                        let provider_options =
                                            block.citations.as_ref().map(|citations| {
                                                let mut opts = serde_json::Map::new();
                                                if let Ok(v) = serde_json::to_value(citations) {
                                                    opts.insert("citations".into(), v);
                                                }
                                                ProviderOptions { options: opts }
                                            });

                                        content_parts.push(AssistantContentPart::Text(
                                            TextContentPart {
                                                text,
                                                provider_options,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::Thinking => {
                                    if let Some(thinking) = block.thinking {
                                        content_parts.push(AssistantContentPart::Reasoning {
                                            text: thinking,
                                            // Preserve the signature in encrypted_content for roundtrip
                                            encrypted_content: block.signature.clone(),
                                        });
                                    }
                                }
                                generated::InputContentBlockType::ToolUse => {
                                    if let (Some(id), Some(name)) = (&block.id, &block.name) {
                                        // The input field type is wrong in generated code, use serde_json for now
                                        let input = if let Some(input_map) = &block.input {
                                            // Convert HashMap to JSON value
                                            serde_json::to_value(input_map)
                                                .unwrap_or(serde_json::Value::Null)
                                        } else {
                                            serde_json::Value::Null
                                        };

                                        content_parts.push(AssistantContentPart::ToolCall {
                                            tool_call_id: id.clone(),
                                            tool_name: name.clone(),
                                            arguments: serde_json::to_string(&input)
                                                .unwrap_or_else(|_| "{}".to_string())
                                                .into(),
                                            encrypted_content: None,
                                            provider_options: None,
                                            provider_executed: None,
                                        });
                                    }
                                }
                                generated::InputContentBlockType::ServerToolUse => {
                                    // Server-executed tool use (web search, etc.)
                                    if let (Some(id), Some(name)) = (&block.id, &block.name) {
                                        let input = if let Some(input_map) = &block.input {
                                            serde_json::to_value(input_map)
                                                .unwrap_or(serde_json::Value::Null)
                                        } else {
                                            serde_json::Value::Null
                                        };

                                        content_parts.push(AssistantContentPart::ToolCall {
                                            tool_call_id: id.clone(),
                                            tool_name: name.clone(),
                                            arguments: serde_json::to_string(&input)
                                                .unwrap_or_else(|_| "{}".to_string())
                                                .into(),
                                            encrypted_content: None,
                                            provider_options: None,
                                            provider_executed: Some(true), // Mark as server-executed
                                        });
                                    }
                                }
                                generated::InputContentBlockType::WebSearchToolResult => {
                                    // Web search tool result - convert to ToolResult with marker
                                    if let Some(id) = &block.tool_use_id {
                                        let mut output = serde_json::Map::new();
                                        output.insert(
                                            "anthropic_type".into(),
                                            serde_json::Value::String(
                                                "web_search_tool_result".to_string(),
                                            ),
                                        );
                                        if let Some(content) = &block.content {
                                            if let Ok(v) = serde_json::to_value(content) {
                                                output.insert("content".into(), v);
                                            }
                                        }

                                        content_parts.push(AssistantContentPart::ToolResult {
                                            tool_call_id: id.clone(),
                                            tool_name: "web_search".to_string(), // Server-executed web search tool
                                            output: serde_json::Value::Object(output),
                                            provider_options: None,
                                        });
                                    }
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            AssistantContent::Array(vec![AssistantContentPart::Text(
                                TextContentPart {
                                    text: String::new(),
                                    provider_options: None,
                                },
                            )])
                        } else {
                            AssistantContent::Array(content_parts)
                        }
                    }
                };

                Ok(Message::Assistant { content, id: None })
            }
        }
    }
}

// Vec conversion is handled by the blanket implementation in universal/convert.rs

impl TryFromLLM<Message> for generated::InputMessage {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::User { content } => {
                let anthropic_content = match content {
                    UserContent::String(text) => generated::MessageContent::String(text),
                    UserContent::Array(parts) => {
                        let blocks = parts
                            .into_iter()
                            .filter_map(|part| match part {
                                UserContentPart::Text(text_part) => {
                                    // Regular text content
                                    Some(generated::InputContentBlock {
                                        cache_control: None,
                                        citations: None,
                                        text: Some(text_part.text),
                                        input_content_block_type:
                                            generated::InputContentBlockType::Text,
                                        source: None,
                                        context: None,
                                        title: None,
                                        content: None,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: None,
                                    })
                                }
                                UserContentPart::Image {
                                    image, media_type, ..
                                } => {
                                    // Convert universal image back to Anthropic format
                                    let data = match image {
                                        serde_json::Value::String(s) => Some(s),
                                        _ => None,
                                    };

                                    if let Some(image_data) = data {
                                        // Check if this is a URL - use URL source type (no media_type required)
                                        let is_url = image_data.starts_with("http://")
                                            || image_data.starts_with("https://");

                                        // Handle text content types - decode to text block instead of document
                                        // Anthropic's Document block only accepts PDFs, not text files
                                        if !is_url {
                                            if let Some(mt) = &media_type {
                                                if mt.starts_with("text/") {
                                                    // Parse data URL and decode base64 to text
                                                    if let Some(media_block) = parse_base64_data_url(&image_data) {
                                                        use base64::Engine;
                                                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&media_block.data) {
                                                            if let Ok(text) = String::from_utf8(bytes) {
                                                                return Some(generated::InputContentBlock {
                                                                    cache_control: None,
                                                                    citations: None,
                                                                    text: Some(text),
                                                                    input_content_block_type: generated::InputContentBlockType::Text,
                                                                    source: None,
                                                                    context: None,
                                                                    title: None,
                                                                    content: None,
                                                                    signature: None,
                                                                    thinking: None,
                                                                    data: None,
                                                                    id: None,
                                                                    input: None,
                                                                    name: None,
                                                                    is_error: None,
                                                                    tool_use_id: None,
                                                                });
                                                            }
                                                        }
                                                    }
                                                    // Skip if can't decode text
                                                    return None;
                                                }
                                            }
                                        }

                                        let (source_type, source_url, source_data, anthropic_media_type) = if is_url {
                                            (
                                                generated::FluffyType::Url,
                                                Some(image_data),
                                                None,
                                                None,
                                            )
                                        } else {
                                            // Base64 data - parse media_type (images and PDFs only)
                                            let anthropic_media_type =
                                                media_type.as_ref().and_then(|mt| match mt.as_str() {
                                                    "image/jpeg" => {
                                                        Some(generated::FluffyMediaType::ImageJpeg)
                                                    }
                                                    "image/png" => {
                                                        Some(generated::FluffyMediaType::ImagePng)
                                                    }
                                                    "image/gif" => {
                                                        Some(generated::FluffyMediaType::ImageGif)
                                                    }
                                                    "image/webp" => {
                                                        Some(generated::FluffyMediaType::ImageWebp)
                                                    }
                                                    "application/pdf" => {
                                                        Some(generated::FluffyMediaType::ApplicationPdf)
                                                    }
                                                    // Text types are handled above, shouldn't reach here
                                                    _ => None,
                                                });
                                            (
                                                generated::FluffyType::Base64,
                                                None,
                                                Some(image_data),
                                                anthropic_media_type,
                                            )
                                        };

                                        // Block type: only PDF uses Document, everything else is Image
                                        let block_type = match anthropic_media_type {
                                            Some(generated::FluffyMediaType::ApplicationPdf) => {
                                                generated::InputContentBlockType::Document
                                            }
                                            _ => generated::InputContentBlockType::Image,
                                        };

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type: block_type,
                                            source: Some(generated::Source::SourceSource(
                                                generated::SourceSource {
                                                    data: source_data,
                                                    media_type: anthropic_media_type,
                                                    source_type,
                                                    url: source_url,
                                                    content: None,
                                                },
                                            )),
                                            context: None,
                                            title: None,
                                            content: None,
                                            signature: None,
                                            thinking: None,
                                            data: None,
                                            id: None,
                                            input: None,
                                            name: None,
                                            is_error: None,
                                            tool_use_id: None,
                                        })
                                    } else {
                                        None
                                    }
                                }
                                UserContentPart::File {
                                    data,
                                    media_type,
                                    provider_options,
                                    ..
                                } => {
                                    // Check if this was originally a Document block
                                    let is_document = provider_options
                                        .as_ref()
                                        .and_then(|opts| opts.options.get("anthropic_type"))
                                        .and_then(|v| v.as_str())
                                        == Some("document");

                                    if is_document {
                                        // Restore as Document block
                                        let title = provider_options
                                            .as_ref()
                                            .and_then(|opts| opts.options.get("title"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());

                                        let context = provider_options
                                            .as_ref()
                                            .and_then(|opts| opts.options.get("context"))
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());

                                        let anthropic_media_type = match media_type.as_str() {
                                            "image/jpeg" => Some(generated::FluffyMediaType::ImageJpeg),
                                            "image/png" => Some(generated::FluffyMediaType::ImagePng),
                                            "image/gif" => Some(generated::FluffyMediaType::ImageGif),
                                            "image/webp" => Some(generated::FluffyMediaType::ImageWebp),
                                            "application/pdf" => {
                                                Some(generated::FluffyMediaType::ApplicationPdf)
                                            }
                                            "text/plain" => Some(generated::FluffyMediaType::TextPlain),
                                            _ => Some(generated::FluffyMediaType::TextPlain),
                                        };

                                        let data_str = match data {
                                            serde_json::Value::String(s) => Some(s),
                                            _ => None,
                                        };

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type:
                                                generated::InputContentBlockType::Document,
                                            source: Some(generated::Source::SourceSource(
                                                generated::SourceSource {
                                                    data: data_str,
                                                    media_type: anthropic_media_type,
                                                    source_type: generated::FluffyType::Text,
                                                    url: None,
                                                    content: None,
                                                },
                                            )),
                                            context,
                                            title,
                                            content: None,
                                            signature: None,
                                            thinking: None,
                                            data: None,
                                            id: None,
                                            input: None,
                                            name: None,
                                            is_error: None,
                                            tool_use_id: None,
                                        })
                                    } else {
                                        // Regular file - skip for now
                                        None
                                    }
                                }
                            })
                            .collect();
                        generated::MessageContent::InputContentBlockArray(blocks)
                    }
                };

                Ok(generated::InputMessage {
                    content: anthropic_content,
                    role: generated::MessageRole::User,
                })
            }
            Message::Assistant { content, .. } => {
                let content = match content {
                    AssistantContent::String(text) => generated::MessageContent::String(text),
                    AssistantContent::Array(parts) => {
                        let blocks = parts
                            .into_iter()
                            .filter_map(|part| match part {
                            AssistantContentPart::Text(text_part) => {
                                // Restore citations from provider_options
                                let citations = text_part.provider_options
                                    .as_ref()
                                    .and_then(|opts| opts.options.get("citations"))
                                    .and_then(|v| serde_json::from_value::<generated::Citations>(v.clone()).ok());

                                Some(generated::InputContentBlock {
                                    cache_control: None,
                                    citations,
                                    text: Some(text_part.text),
                                    input_content_block_type:
                                        generated::InputContentBlockType::Text,
                                    source: None,
                                    context: None,
                                    title: None,
                                    content: None,
                                    signature: None,
                                    thinking: None,
                                    data: None,
                                    id: None,
                                    input: None,
                                    name: None,
                                    is_error: None,
                                    tool_use_id: None,
                                })
                            }
                            AssistantContentPart::Reasoning {
                                text,
                                encrypted_content,
                            } => {
                                Some(generated::InputContentBlock {
                                    cache_control: None,
                                    citations: None,
                                    text: None,
                                    input_content_block_type:
                                        generated::InputContentBlockType::Thinking,
                                    source: None,
                                    context: None,
                                    title: None,
                                    content: None,
                                    // Restore signature from encrypted_content
                                    signature: encrypted_content,
                                    thinking: Some(text),
                                    data: None,
                                    id: None,
                                    input: None,
                                    name: None,
                                    is_error: None,
                                    tool_use_id: None,
                                })
                            }
                            AssistantContentPart::ToolCall {
                                tool_call_id,
                                tool_name,
                                arguments,
                                provider_executed,
                                ..
                            } => {
                                // Convert ToolCallArguments to serde_json::Map
                                let input_map = match &arguments {
                                    ToolCallArguments::Valid(map) => Some(map.clone()),
                                    ToolCallArguments::Invalid(_) => None,
                                };

                                // Use ServerToolUse for provider-executed tools
                                let block_type = if provider_executed == Some(true) {
                                    generated::InputContentBlockType::ServerToolUse
                                } else {
                                    generated::InputContentBlockType::ToolUse
                                };

                                Some(generated::InputContentBlock {
                                    cache_control: None,
                                    citations: None,
                                    text: None,
                                    input_content_block_type: block_type,
                                    source: None,
                                    context: None,
                                    title: None,
                                    content: None,
                                    signature: None,
                                    thinking: None,
                                    data: None,
                                    id: Some(tool_call_id.clone()),
                                    input: input_map,
                                    name: Some(tool_name.clone()),
                                    is_error: None,
                                    tool_use_id: None,
                                })
                            }
                            AssistantContentPart::ToolResult {
                                tool_call_id,
                                output,
                                ..
                            } => {
                                // Check if this was a web_search_tool_result
                                let is_web_search_result = output.as_object()
                                    .and_then(|obj| obj.get("anthropic_type"))
                                    .and_then(|v| v.as_str())
                                    == Some("web_search_tool_result");

                                if is_web_search_result {
                                    // Restore WebSearchToolResult block
                                    let content = output.as_object()
                                        .and_then(|obj| obj.get("content"))
                                        .and_then(|v| serde_json::from_value::<generated::Content>(v.clone()).ok());

                                    Some(generated::InputContentBlock {
                                        cache_control: None,
                                        citations: None,
                                        text: None,
                                        input_content_block_type:
                                            generated::InputContentBlockType::WebSearchToolResult,
                                        source: None,
                                        context: None,
                                        title: None,
                                        content,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        is_error: None,
                                        tool_use_id: Some(tool_call_id.clone()),
                                    })
                                } else {
                                    None // Skip other tool results in assistant messages
                                }
                            }
                            _ => None, // Skip other types for now
                        })
                        .collect();
                        generated::MessageContent::InputContentBlockArray(blocks)
                    }
                };

                Ok(generated::InputMessage {
                    content,
                    role: generated::MessageRole::Assistant,
                })
            }
            Message::Tool { content } => {
                // Convert tool results back to user message with tool_result blocks
                let mut blocks = Vec::new();
                for part in content {
                    match part {
                        ToolContentPart::ToolResult(tool_result) => {
                            let content = match &tool_result.output {
                                serde_json::Value::String(s) => {
                                    Some(generated::Content::String(s.clone()))
                                }
                                other => Some(generated::Content::String(
                                    serde_json::to_string(other)
                                        .map_err(|e| ConvertError::JsonSerializationFailed {
                                            field: "tool_result_output".to_string(),
                                            error: e.to_string(),
                                        })?,
                                )),
                            };

                            blocks.push(generated::InputContentBlock {
                                cache_control: None,
                                citations: None,
                                text: None,
                                input_content_block_type:
                                    generated::InputContentBlockType::ToolResult,
                                source: None,
                                context: None,
                                title: None,
                                content,
                                signature: None,
                                thinking: None,
                                data: None,
                                id: None,
                                input: None,
                                name: None,
                                is_error: None,
                                tool_use_id: Some(tool_result.tool_call_id),
                            });
                        }
                    }
                }

                Ok(generated::InputMessage {
                    content: generated::MessageContent::InputContentBlockArray(blocks),
                    role: generated::MessageRole::User,
                })
            }
            Message::System { .. } => Err(ConvertError::UnsupportedInputType {
                type_info: "System messages are not supported in Anthropic InputMessage (use system parameter instead)".to_string(),
            }),
        }
    }
}

// Convert from Anthropic response ContentBlock to Universal Message
impl TryFromLLM<Vec<generated::ContentBlock>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(content_blocks: Vec<generated::ContentBlock>) -> Result<Self, Self::Error> {
        let mut content_parts = Vec::new();

        for block in content_blocks {
            match block.content_block_type {
                generated::ContentBlockType::Text => {
                    if let Some(text) = block.text {
                        // Preserve citations in provider_options for roundtrip
                        let provider_options = block.citations.as_ref().map(|citations| {
                            let mut opts = serde_json::Map::new();
                            if let Ok(v) = serde_json::to_value(citations) {
                                opts.insert("citations".into(), v);
                            }
                            ProviderOptions { options: opts }
                        });
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text,
                            provider_options,
                        }));
                    }
                }
                generated::ContentBlockType::Thinking => {
                    if let Some(thinking) = block.thinking {
                        content_parts.push(AssistantContentPart::Reasoning {
                            text: thinking,
                            // Preserve signature in encrypted_content for roundtrip
                            encrypted_content: block.signature.clone(),
                        });
                    }
                }
                generated::ContentBlockType::ToolUse => {
                    if let (Some(id), Some(name)) = (block.id, block.name) {
                        // Convert HashMap to JSON value for response processing too
                        let input = if let Some(input_map) = block.input {
                            serde_json::to_value(input_map).unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        };

                        content_parts.push(AssistantContentPart::ToolCall {
                            tool_call_id: id,
                            tool_name: name,
                            arguments: serde_json::to_string(&input)
                                .unwrap_or_else(|_| "{}".to_string())
                                .into(),
                            encrypted_content: None,
                            provider_options: None,
                            provider_executed: None,
                        });
                    }
                }
                generated::ContentBlockType::ServerToolUse => {
                    // Server-executed tool (similar to ToolUse but provider_executed=true)
                    if let (Some(id), Some(name)) = (block.id, block.name) {
                        let input = if let Some(input_map) = block.input {
                            serde_json::to_value(input_map).unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        };

                        content_parts.push(AssistantContentPart::ToolCall {
                            tool_call_id: id,
                            tool_name: name,
                            arguments: serde_json::to_string(&input)
                                .unwrap_or_else(|_| "{}".to_string())
                                .into(),
                            encrypted_content: None,
                            provider_options: None,
                            provider_executed: Some(true), // Mark as server-executed
                        });
                    }
                }
                generated::ContentBlockType::WebSearchToolResult => {
                    // Web search tool result - convert to ToolResult with full data
                    if let Some(id) = block.tool_use_id {
                        // Store the entire block data for roundtrip
                        let mut output = serde_json::Map::new();
                        output.insert(
                            "anthropic_type".into(),
                            serde_json::Value::String("web_search_tool_result".to_string()),
                        );
                        if let Some(content) = &block.content {
                            if let Ok(v) = serde_json::to_value(content) {
                                output.insert("content".into(), v);
                            }
                        }

                        content_parts.push(AssistantContentPart::ToolResult {
                            tool_call_id: id,
                            tool_name: "web_search".to_string(),
                            output: serde_json::Value::Object(output),
                            provider_options: None,
                        });
                    }
                }
                _ => {
                    // Skip other types (RedactedThinking, etc.)
                    continue;
                }
            }
        }

        if content_parts.is_empty() {
            content_parts.push(AssistantContentPart::Text(TextContentPart {
                text: String::new(),
                provider_options: None,
            }));
        }

        Ok(vec![Message::Assistant {
            content: AssistantContent::Array(content_parts),
            id: None,
        }])
    }
}

// Convert from Universal Message to Anthropic response ContentBlock
impl TryFromLLM<Vec<Message>> for Vec<generated::ContentBlock> {
    type Error = ConvertError;

    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        let mut content_blocks = Vec::new();

        for message in messages {
            match message {
                Message::Assistant { content, .. } => match content {
                    AssistantContent::String(text) => {
                        content_blocks.push(generated::ContentBlock {
                            citations: None,
                            text: Some(text),
                            content_block_type: generated::ContentBlockType::Text,
                            signature: None,
                            thinking: None,
                            data: None,
                            id: None,
                            input: None,
                            name: None,
                            content: None,
                            tool_use_id: None,
                        });
                    }
                    AssistantContent::Array(parts) => {
                        for part in parts {
                            match part {
                                AssistantContentPart::Text(text_part) => {
                                    // Restore citations from provider_options if present
                                    let citations = text_part
                                        .provider_options
                                        .as_ref()
                                        .and_then(|opts| opts.options.get("citations"))
                                        .and_then(|v| {
                                            serde_json::from_value::<
                                                Vec<generated::ResponseLocationCitation>,
                                            >(v.clone())
                                            .ok()
                                        });
                                    content_blocks.push(generated::ContentBlock {
                                        citations,
                                        text: Some(text_part.text),
                                        content_block_type: generated::ContentBlockType::Text,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        content: None,
                                        tool_use_id: None,
                                    });
                                }
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content,
                                } => {
                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: generated::ContentBlockType::Thinking,
                                        // Restore signature from encrypted_content
                                        signature: encrypted_content,
                                        thinking: Some(text),
                                        data: None,
                                        id: None,
                                        input: None,
                                        name: None,
                                        content: None,
                                        tool_use_id: None,
                                    });
                                }
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    provider_executed,
                                    ..
                                } => {
                                    // Convert ToolCallArguments to serde_json::Map for response generation
                                    let input_map = match &arguments {
                                        ToolCallArguments::Valid(map) => Some(map.clone()),
                                        ToolCallArguments::Invalid(_) => None,
                                    };

                                    // Use ServerToolUse if provider_executed is true
                                    let block_type = if provider_executed == Some(true) {
                                        generated::ContentBlockType::ServerToolUse
                                    } else {
                                        generated::ContentBlockType::ToolUse
                                    };

                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: block_type,
                                        signature: None,
                                        thinking: None,
                                        data: None,
                                        id: Some(tool_call_id.clone()),
                                        input: input_map,
                                        name: Some(tool_name.clone()),
                                        content: None,
                                        tool_use_id: None,
                                    });
                                }
                                AssistantContentPart::ToolResult {
                                    tool_call_id,
                                    output,
                                    ..
                                } => {
                                    // Check if this is a web_search_tool_result
                                    let is_web_search_result =
                                        output.get("anthropic_type").and_then(|v| v.as_str())
                                            == Some("web_search_tool_result");

                                    if is_web_search_result {
                                        // Restore as WebSearchToolResult
                                        let content = output.get("content").and_then(|v| {
                                            serde_json::from_value::<
                                                    generated::ContentBlockContent,
                                                >(
                                                    v.clone()
                                                )
                                                .ok()
                                        });

                                        content_blocks.push(generated::ContentBlock {
                                            citations: None,
                                            text: None,
                                            content_block_type:
                                                generated::ContentBlockType::WebSearchToolResult,
                                            signature: None,
                                            thinking: None,
                                            data: None,
                                            id: None,
                                            input: None,
                                            name: None,
                                            content,
                                            tool_use_id: Some(tool_call_id.clone()),
                                        });
                                    }
                                    // Skip other tool results - they shouldn't appear in response content
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }
                    }
                },
                _ => {
                    // Skip non-assistant messages
                    continue;
                }
            }
        }

        Ok(content_blocks)
    }
}

impl From<&ToolChoice> for ToolChoiceConfig {
    fn from(tc: &ToolChoice) -> Self {
        let mode = Some(match tc.tool_choice_type {
            ToolChoiceType::Auto => ToolChoiceMode::Auto,
            ToolChoiceType::None => ToolChoiceMode::None,
            ToolChoiceType::Any => ToolChoiceMode::Required,
            ToolChoiceType::Tool => ToolChoiceMode::Tool,
        });
        ToolChoiceConfig {
            mode,
            tool_name: tc.name.clone(),
            disable_parallel: tc.disable_parallel_tool_use,
        }
    }
}

impl TryFrom<&ToolChoiceConfig> for ToolChoice {
    type Error = ();

    fn try_from(config: &ToolChoiceConfig) -> Result<Self, Self::Error> {
        let needs_disable_parallel = config.disable_parallel == Some(true);
        let mode = match config.mode {
            Some(m) => m,
            None if needs_disable_parallel => ToolChoiceMode::Auto,
            None => return Err(()),
        };
        Ok(ToolChoice {
            tool_choice_type: match mode {
                ToolChoiceMode::Auto => ToolChoiceType::Auto,
                ToolChoiceMode::None => ToolChoiceType::None,
                ToolChoiceMode::Required => ToolChoiceType::Any,
                ToolChoiceMode::Tool => ToolChoiceType::Tool,
            },
            name: if mode == ToolChoiceMode::Tool {
                config.tool_name.clone()
            } else {
                None
            },
            disable_parallel_tool_use: if needs_disable_parallel {
                Some(true)
            } else {
                None
            },
        })
    }
}

// =============================================================================
// Response Format Conversions (Anthropic ↔ Universal)
// =============================================================================

impl From<&JsonOutputFormat> for ResponseFormatConfig {
    fn from(format: &JsonOutputFormat) -> Self {
        let format_type = match format.json_output_format_type {
            JsonOutputFormatType::JsonSchema => Some(ResponseFormatType::JsonSchema),
        };
        let json_schema = format_type
            .filter(|ft| *ft == ResponseFormatType::JsonSchema)
            .map(|_| JsonSchemaConfig {
                name: "response".to_string(),
                schema: Value::Object(format.schema.clone()),
                strict: None,
                description: None,
            });
        ResponseFormatConfig {
            format_type,
            json_schema,
        }
    }
}

impl TryFrom<&ResponseFormatConfig> for JsonOutputFormat {
    type Error = ();

    fn try_from(config: &ResponseFormatConfig) -> Result<Self, Self::Error> {
        match config.format_type.ok_or(())? {
            ResponseFormatType::Text => Err(()),
            ResponseFormatType::JsonObject => Ok(JsonOutputFormat {
                schema: serde_json::from_value(json!({
                    "type": "object",
                    "additionalProperties": false
                }))
                .expect("static JSON object is always a valid Map"),
                json_output_format_type: JsonOutputFormatType::JsonSchema,
            }),
            ResponseFormatType::JsonSchema => {
                let js = config.json_schema.as_ref().ok_or(())?;
                match &js.schema {
                    Value::Object(m) => Ok(JsonOutputFormat {
                        schema: m.clone(),
                        json_output_format_type: JsonOutputFormatType::JsonSchema,
                    }),
                    _ => Err(()),
                }
            }
        }
    }
}

impl TryFrom<&UniversalTool> for CustomTool {
    type Error = ConvertError;

    fn try_from(tool: &UniversalTool) -> Result<Self, Self::Error> {
        match &tool.tool_type {
            UniversalToolType::Function => Ok(CustomTool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.parameters.clone().unwrap_or_else(|| json!({})),
                strict: tool.strict,
                cache_control: None,
                eager_input_streaming: None,
            }),
            UniversalToolType::Custom { .. } => Err(ConvertError::UnsupportedToolType {
                tool_name: tool.name.clone(),
                tool_type: "custom".to_string(),
                target_provider: ProviderFormat::Anthropic,
            }),
            UniversalToolType::Builtin { builtin_type, .. } => {
                Err(ConvertError::UnsupportedToolType {
                    tool_name: tool.name.clone(),
                    tool_type: builtin_type.clone(),
                    target_provider: ProviderFormat::Anthropic,
                })
            }
        }
    }
}

impl From<&Tool> for UniversalTool {
    fn from(tool: &Tool) -> Self {
        match tool {
            Tool::Custom(ct) => Self::function(
                &ct.name,
                ct.description.clone(),
                Some(ct.input_schema.clone()),
                ct.strict,
            ),
            other => {
                let config = serde_json::to_value(other).ok();
                let type_str = config
                    .as_ref()
                    .and_then(|v| v.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let name = config
                    .as_ref()
                    .and_then(|v| v.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or(&type_str)
                    .to_string();
                Self::builtin(name, BuiltinToolProvider::Anthropic, type_str, config)
            }
        }
    }
}

impl TryFrom<&UniversalTool> for Tool {
    type Error = ConvertError;

    fn try_from(tool: &UniversalTool) -> Result<Self, Self::Error> {
        match &tool.tool_type {
            UniversalToolType::Function => Ok(Tool::Custom(CustomTool::try_from(tool)?)),
            UniversalToolType::Custom { .. } => Err(ConvertError::UnsupportedToolType {
                tool_name: tool.name.clone(),
                tool_type: "custom".to_string(),
                target_provider: ProviderFormat::Anthropic,
            }),
            UniversalToolType::Builtin {
                provider,
                builtin_type,
                config,
            } => {
                if !matches!(provider, BuiltinToolProvider::Anthropic) {
                    return Err(ConvertError::UnsupportedToolType {
                        tool_name: tool.name.clone(),
                        tool_type: builtin_type.clone(),
                        target_provider: ProviderFormat::Anthropic,
                    });
                }
                let config_val =
                    config
                        .clone()
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: format!("config for Anthropic builtin tool '{}'", tool.name),
                        })?;
                serde_json::from_value::<Tool>(config_val).map_err(|e| {
                    ConvertError::JsonSerializationFailed {
                        field: format!("builtin tool '{}'", tool.name),
                        error: e.to_string(),
                    }
                })
            }
        }
    }
}

impl TryFromLLM<Vec<Tool>> for Vec<UniversalTool> {
    type Error = ConvertError;

    fn try_from(tools: Vec<Tool>) -> Result<Self, Self::Error> {
        Ok(tools.iter().map(UniversalTool::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::convert::TryFromLLM;

    #[test]
    fn test_file_to_anthropic_document_with_provider_options() {
        // Create a File content part marked as a document (via provider_options)
        let mut opts = serde_json::Map::new();
        opts.insert(
            "anthropic_type".into(),
            serde_json::Value::String("document".to_string()),
        );
        opts.insert(
            "title".into(),
            serde_json::Value::String("Test Document".to_string()),
        );

        let file_part = UserContentPart::File {
            data: serde_json::Value::String("base64encodeddata".to_string()),
            filename: Some("test.pdf".to_string()),
            media_type: "application/pdf".to_string(),
            provider_options: Some(ProviderOptions { options: opts }),
        };

        // Create a user message with the file part
        let message = Message::User {
            content: UserContent::Array(vec![file_part]),
        };

        // Convert to Anthropic InputMessage
        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok(), "File conversion should succeed");
        let input_msg = result.unwrap();

        // Verify it's a user message with document block
        assert!(matches!(input_msg.role, generated::MessageRole::User));
        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1, "Should have exactly one content block");
            let block = &blocks[0];
            assert!(
                matches!(
                    block.input_content_block_type,
                    generated::InputContentBlockType::Document
                ),
                "Should be a Document block"
            );
            assert_eq!(block.title, Some("Test Document".to_string()));
        } else {
            panic!("Expected InputContentBlockArray");
        }
    }

    #[test]
    fn test_regular_file_without_anthropic_marker_is_skipped() {
        // Create a regular File content part (no anthropic_type marker)
        let file_part = UserContentPart::File {
            data: serde_json::Value::String("base64encodeddata".to_string()),
            filename: Some("test.pdf".to_string()),
            media_type: "application/pdf".to_string(),
            provider_options: None, // No anthropic_type marker
        };

        let message = Message::User {
            content: UserContent::Array(vec![file_part]),
        };

        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok());
        let input_msg = result.unwrap();

        // Regular files without anthropic_type marker are currently skipped
        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            // The file was skipped, so blocks should be empty
            assert!(
                blocks.is_empty(),
                "Regular files without anthropic_type should be skipped (current behavior)"
            );
        }
    }

    #[test]
    fn test_image_url_to_anthropic() {
        let image_part = UserContentPart::Image {
            image: serde_json::Value::String("https://example.com/image.jpg".to_string()),
            media_type: Some("image/jpeg".to_string()),
            provider_options: None,
        };

        let message = Message::User {
            content: UserContent::Array(vec![image_part]),
        };

        let result: Result<generated::InputMessage, _> =
            <generated::InputMessage as TryFromLLM<Message>>::try_from(message);

        assert!(result.is_ok());
        let input_msg = result.unwrap();

        if let generated::MessageContent::InputContentBlockArray(blocks) = input_msg.content {
            assert_eq!(blocks.len(), 1);
            let block = &blocks[0];
            assert!(matches!(
                block.input_content_block_type,
                generated::InputContentBlockType::Image
            ));
            // Verify URL source type is used
            if let Some(generated::Source::SourceSource(source)) = &block.source {
                assert!(matches!(source.source_type, generated::FluffyType::Url));
                assert_eq!(
                    source.url,
                    Some("https://example.com/image.jpg".to_string())
                );
            } else {
                panic!("Expected SourceSource");
            }
        } else {
            panic!("Expected InputContentBlockArray");
        }
    }
}
