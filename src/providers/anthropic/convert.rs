use crate::providers::anthropic::generated;
use crate::universal::{
    convert::TryFromLLM, AssistantContent, AssistantContentPart, Message, TextContentPart,
    UserContent, UserContentPart,
};

// Vec conversion is handled by the blanket implementation in universal/convert.rs

impl TryFromLLM<generated::InputMessage> for Message {
    type Error = String;

    fn try_from(input_msg: generated::InputMessage) -> Result<Self, Self::Error> {
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
                                            generated::InputContentBlockSource::PurpleSource(
                                                purple_source,
                                            ) => {
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
                                    // TODO: Handle tool results - these should become separate Message::Tool entries
                                    // For now, skip to avoid type errors
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
                    generated::MessageContent::String(text) => {
                        AssistantContent::Array(vec![AssistantContentPart::Text(TextContentPart {
                            text,
                            provider_options: None,
                        })])
                    }
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
                                            input,
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
    type Error = String;

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

                                        Some(generated::InputContentBlock {
                                            cache_control: None,
                                            citations: None,
                                            text: None,
                                            input_content_block_type:
                                                generated::InputContentBlockType::Image,
                                            source: Some(
                                                generated::InputContentBlockSource::PurpleSource(
                                                    generated::PurpleSource {
                                                        data: Some(image_data),
                                                        media_type: anthropic_media_type,
                                                        source_type: generated::FluffyType::Base64,
                                                        url: None,
                                                        content: None,
                                                    },
                                                ),
                                            ),
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
                                // TODO: Handle tool results - they should come from Message::Tool, not UserContentPart
                                _ => None, // Skip other parts for now
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
                let blocks = match content {
                    AssistantContent::String(text) => {
                        vec![generated::InputContentBlock {
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
                        }]
                    }
                    AssistantContent::Array(parts) => parts
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
                                input,
                                ..
                            } => {
                                // Convert JSON value back to HashMap - this is a workaround for type issues
                                let input_map = serde_json::from_value::<
                                    std::collections::HashMap<
                                        String,
                                        Option<generated::WebSearchToolResultErrorCode>,
                                    >,
                                >(input.clone())
                                .ok();

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
                        .collect(),
                };

                Ok(generated::InputMessage {
                    content: generated::MessageContent::InputContentBlockArray(blocks),
                    role: generated::MessageRole::Assistant,
                })
            }
            _ => Err("Unsupported message type for Anthropic conversion".to_string()),
        }
    }
}

// Convert from Anthropic response ContentBlock to Universal Message
impl TryFromLLM<&Vec<generated::ContentBlock>> for Vec<Message> {
    type Error = String;

    fn try_from(content_blocks: &Vec<generated::ContentBlock>) -> Result<Self, Self::Error> {
        let mut content_parts = Vec::new();

        for block in content_blocks {
            match block.content_block_type {
                generated::ContentBlockType::Text => {
                    if let Some(text) = &block.text {
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text: text.clone(),
                            provider_options: None,
                        }));
                    }
                }
                generated::ContentBlockType::Thinking => {
                    if let Some(thinking) = &block.thinking {
                        content_parts.push(AssistantContentPart::Reasoning {
                            text: thinking.clone(),
                            encrypted_content: None,
                        });
                    }
                }
                generated::ContentBlockType::ToolUse => {
                    if let (Some(id), Some(name)) = (&block.id, &block.name) {
                        // Convert HashMap to JSON value for response processing too
                        let input = if let Some(input_map) = &block.input {
                            serde_json::to_value(input_map).unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        };

                        content_parts.push(AssistantContentPart::ToolCall {
                            tool_call_id: id.clone(),
                            tool_name: name.clone(),
                            input,
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
    type Error = String;

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
                                    input,
                                    ..
                                } => {
                                    // Convert JSON value to HashMap for response generation
                                    let input_map =
                                        serde_json::from_value::<
                                            std::collections::HashMap<
                                                String,
                                                Option<generated::WebSearchToolResultErrorCode>,
                                            >,
                                        >(input.clone())
                                        .ok();

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
