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
                                _ => {
                                    // Skip other types for now (tool_use, tool_result, etc.)
                                    continue;
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            UserContent::String(String::new())
                        } else if content_parts.len() == 1 {
                            // Single text part can be simplified to string
                            match &content_parts[0] {
                                UserContentPart::Text(text_part) => {
                                    UserContent::String(text_part.text.clone())
                                }
                                _ => UserContent::Array(content_parts),
                            }
                        } else {
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
                                _ => {
                                    // Skip other types for now (tool_use, tool_result, etc.)
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
                                _ => None, // Skip non-text parts for now
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
                _ => {
                    // Skip other types for now (tool_use, tool_result, etc.)
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
