use crate::error::ConvertError;
use crate::providers::anthropic::generated;
use crate::serde_json;
use crate::universal::{
    convert::TryFromLLM, AssistantContent, AssistantContentPart, Message, TextContentPart,
    ToolCallArguments, ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};

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
                                                let media_type = purple_source.media_type.map(|mt| match mt {
                                                    generated::FluffyMediaType::ImageJpeg => "image/jpeg".to_string(),
                                                    generated::FluffyMediaType::ImagePng => "image/png".to_string(),
                                                    generated::FluffyMediaType::ImageGif => "image/gif".to_string(),
                                                    generated::FluffyMediaType::ImageWebp => "image/webp".to_string(),
                                                    generated::FluffyMediaType::ApplicationPdf => "application/pdf".to_string(),
                                                    generated::FluffyMediaType::TextPlain => "text/plain".to_string(),
                                                });

                                                // Handle both base64 data and URL sources
                                                let image_value =
                                                    if let Some(data) = purple_source.data {
                                                        serde_json::Value::String(data)
                                                    } else if let Some(url) = purple_source.url {
                                                        serde_json::Value::String(url)
                                                    } else {
                                                        continue;
                                                    };

                                                content_parts.push(UserContentPart::Image {
                                                    image: image_value,
                                                    media_type,
                                                    provider_options: None,
                                                });
                                            }
                                            _ => {
                                                // Skip other source types for now
                                                continue;
                                            }
                                        }
                                    }
                                }
                                generated::InputContentBlockType::Document => {
                                    if let Some(source) = block.source {
                                        match source {
                                            generated::Source::SourceSource(purple_source) => {
                                                let media_type = purple_source.media_type.map(|mt| match mt {
                                                    generated::FluffyMediaType::ImageJpeg => "image/jpeg".to_string(),
                                                    generated::FluffyMediaType::ImagePng => "image/png".to_string(),
                                                    generated::FluffyMediaType::ImageGif => "image/gif".to_string(),
                                                    generated::FluffyMediaType::ImageWebp => "image/webp".to_string(),
                                                    generated::FluffyMediaType::ApplicationPdf => "application/pdf".to_string(),
                                                    generated::FluffyMediaType::TextPlain => "text/plain".to_string(),
                                                });

                                                // Handle both base64 data and URL sources for documents
                                                let file_value =
                                                    if let Some(data) = purple_source.data {
                                                        serde_json::Value::String(data)
                                                    } else if let Some(url) = purple_source.url {
                                                        serde_json::Value::String(url)
                                                    } else {
                                                        continue;
                                                    };

                                                // media_type is required for File, use a default if not provided
                                                let file_media_type =
                                                    media_type.unwrap_or_else(|| {
                                                        "application/octet-stream".to_string()
                                                    });

                                                content_parts.push(UserContentPart::File {
                                                    data: file_value,
                                                    filename: None,
                                                    media_type: file_media_type,
                                                    provider_options: None,
                                                });
                                            }
                                            _ => {
                                                continue;
                                            }
                                        }
                                    }
                                }
                                generated::InputContentBlockType::ToolResult => {
                                    // Skip tool results for now - should be handled properly
                                    continue;
                                }
                                _ => {
                                    // Skip other types for now
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            UserContent::String(String::new())
                        } else if content_parts.len() == 1 {
                            // Single text part can be simplified to string, but keep arrays for multimodal
                            match &content_parts[0] {
                                UserContentPart::Text(text_part) => {
                                    UserContent::String(text_part.text.clone())
                                }
                                _ => UserContent::Array(content_parts),
                            }
                        } else {
                            // Multiple parts or multimodal content must remain as array
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
                                        content_parts.push(AssistantContentPart::Text(
                                            TextContentPart {
                                                text,
                                                provider_options: None,
                                            },
                                        ));
                                    }
                                }
                                generated::InputContentBlockType::Thinking => {
                                    if let Some(thinking) = block.thinking {
                                        content_parts.push(AssistantContentPart::Reasoning {
                                            text: thinking,
                                            encrypted_content: None,
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
                                            provider_options: None,
                                            provider_executed: None,
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
                                    let image_str = match image {
                                        serde_json::Value::String(s) => Some(s),
                                        _ => None,
                                    };

                                    if let Some(image_data) = image_str {
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
                                                "text/plain" => {
                                                    Some(generated::FluffyMediaType::TextPlain)
                                                }
                                                _ => None,
                                            });

                                        // Detect if this is a URL or base64 data
                                        let is_url = image_data.starts_with("http://")
                                            || image_data.starts_with("https://");
                                        let (source_type, data, url) = if is_url {
                                            (generated::FluffyType::Url, None, Some(image_data))
                                        } else {
                                            (generated::FluffyType::Base64, Some(image_data), None)
                                        };

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type:
                                                generated::InputContentBlockType::Image,
                                            source: Some(generated::Source::SourceSource(
                                                generated::SourceSource {
                                                    data,
                                                    media_type: anthropic_media_type,
                                                    source_type,
                                                    url,
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
                                    data, media_type, ..
                                } => {
                                    // Convert universal file/document back to Anthropic format
                                    let file_str = match data {
                                        serde_json::Value::String(s) => Some(s),
                                        _ => None,
                                    };

                                    if let Some(file_data) = file_str {
                                        let anthropic_media_type = match media_type.as_str() {
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
                                            "text/plain" => {
                                                Some(generated::FluffyMediaType::TextPlain)
                                            }
                                            _ => None,
                                        };

                                        // Detect if this is a URL or base64 data
                                        let is_url = file_data.starts_with("http://")
                                            || file_data.starts_with("https://");
                                        let (source_type, data, url) = if is_url {
                                            (generated::FluffyType::Url, None, Some(file_data))
                                        } else {
                                            (generated::FluffyType::Base64, Some(file_data), None)
                                        };

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type:
                                                generated::InputContentBlockType::Document,
                                            source: Some(generated::Source::SourceSource(
                                                generated::SourceSource {
                                                    data,
                                                    media_type: anthropic_media_type,
                                                    source_type,
                                                    url,
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
                let anthropic_content = match content {
                    AssistantContent::String(text) => generated::MessageContent::String(text),
                    AssistantContent::Array(parts) => {
                        let blocks: Vec<generated::InputContentBlock> = parts
                            .into_iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::Text(text_part) => {
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
                                AssistantContentPart::Reasoning { text, .. } => {
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
                                        signature: None,
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
                                    ..
                                } => {
                                    // Convert ToolCallArguments to serde_json::Map
                                    let input_map = match &arguments {
                                        ToolCallArguments::Valid(map) => Some(map.clone()),
                                        ToolCallArguments::Invalid(_) => None,
                                    };

                                    Some(generated::InputContentBlock {
                                        cache_control: None,
                                        citations: None,
                                        text: None,
                                        input_content_block_type:
                                            generated::InputContentBlockType::ToolUse,
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
                                _ => None, // Skip other types for now
                            })
                            .collect();
                        generated::MessageContent::InputContentBlockArray(blocks)
                    }
                };

                Ok(generated::InputMessage {
                    content: anthropic_content,
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
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text,
                            provider_options: None,
                        }));
                    }
                }
                generated::ContentBlockType::Thinking => {
                    if let Some(thinking) = block.thinking {
                        content_parts.push(AssistantContentPart::Reasoning {
                            text: thinking,
                            encrypted_content: None,
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
                            provider_options: None,
                            provider_executed: None,
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
                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
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
                                AssistantContentPart::Reasoning { text, .. } => {
                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: generated::ContentBlockType::Thinking,
                                        signature: None,
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
                                    ..
                                } => {
                                    // Convert ToolCallArguments to serde_json::Map for response generation
                                    let input_map = match &arguments {
                                        ToolCallArguments::Valid(map) => Some(map.clone()),
                                        ToolCallArguments::Invalid(_) => None,
                                    };

                                    content_blocks.push(generated::ContentBlock {
                                        citations: None,
                                        text: None,
                                        content_block_type: generated::ContentBlockType::ToolUse,
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

        if content_blocks.is_empty() {
            content_blocks.push(generated::ContentBlock {
                citations: None,
                text: Some(String::new()),
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

        Ok(content_blocks)
    }
}
