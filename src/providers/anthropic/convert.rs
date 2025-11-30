use crate::error::ConvertError;
use crate::providers::anthropic::generated;
use crate::serde_json;
use crate::universal::{
    convert::TryFromLLM, AssistantContent, AssistantContentPart, ClientTool, Message, ProviderTool,
    TextContentPart, Tool, ToolCallArguments, ToolContentPart, ToolResultContentPart, UserContent,
    UserContentPart,
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
                                            source: Some(generated::Source::SourceSource(
                                                generated::SourceSource {
                                                    data: Some(image_data),
                                                    media_type: anthropic_media_type,
                                                    source_type: generated::FluffyType::Base64,
                                                    url: None,
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
                        .collect(),
                };

                Ok(generated::InputMessage {
                    content: generated::MessageContent::InputContentBlockArray(blocks),
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

// ============================================================================
// Tool Conversions
// ============================================================================

/// Convert Lingua Tool to Anthropic Tool
impl TryFromLLM<Tool> for generated::Tool {
    type Error = String;

    fn try_from(tool: Tool) -> Result<Self, Self::Error> {
        match tool {
            Tool::Client(client_tool) => {
                let schema_obj: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_value(client_tool.input_schema.clone()).map_err(|e| {
                        format!("Invalid input_schema, must be a JSON object: {}", e)
                    })?;

                let properties = schema_obj
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .cloned();

                let required: Option<Vec<String>> = schema_obj
                    .get("required")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });

                // Build input_schema as JSON Value (our generated types use serde_json::Value
                // for complex nested types to avoid dependency on specific type definitions)
                let mut input_schema_obj = serde_json::Map::new();
                input_schema_obj.insert("type".to_string(), serde_json::json!("object"));
                if let Some(props) = properties {
                    input_schema_obj
                        .insert("properties".to_string(), serde_json::Value::Object(props));
                }
                if let Some(req) = required {
                    input_schema_obj.insert(
                        "required".to_string(),
                        serde_json::Value::Array(
                            req.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }

                Ok(generated::Tool::Custom(generated::CustomTool {
                    name: client_tool.name,
                    description: Some(client_tool.description),
                    input_schema: serde_json::Value::Object(input_schema_obj),
                    cache_control: None,
                }))
            }
            Tool::Provider(provider_tool) => {
                let config = provider_tool
                    .config
                    .unwrap_or_else(|| serde_json::json!({}));
                match provider_tool.tool_type.as_str() {
                    "web_search_20250305" => {
                        let max_uses = config.get("max_uses").and_then(|v| v.as_i64());
                        let allowed_domains = config
                            .get("allowed_domains")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            });
                        let blocked_domains = config
                            .get("blocked_domains")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            });
                        let user_location = config
                            .get("user_location")
                            .and_then(|v| serde_json::from_value(v.clone()).ok());

                        Ok(generated::Tool::WebSearch20250305(
                            generated::WebSearchTool20250305 {
                                name: provider_tool
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| provider_tool.tool_type.clone()),
                                max_uses,
                                allowed_domains,
                                blocked_domains,
                                cache_control: None,
                                user_location,
                            },
                        ))
                    }
                    "bash_20250124" => {
                        Ok(generated::Tool::Bash20250124(generated::BashTool20250124 {
                            name: provider_tool
                                .name
                                .clone()
                                .unwrap_or_else(|| provider_tool.tool_type.clone()),
                            cache_control: None,
                        }))
                    }
                    "text_editor_20250124" => Ok(generated::Tool::TextEditor20250124(
                        generated::TextEditor20250124 {
                            name: provider_tool
                                .name
                                .clone()
                                .unwrap_or_else(|| provider_tool.tool_type.clone()),
                            cache_control: None,
                        },
                    )),
                    "text_editor_20250429" => Ok(generated::Tool::TextEditor20250429(
                        generated::TextEditor20250429 {
                            name: provider_tool
                                .name
                                .clone()
                                .unwrap_or_else(|| provider_tool.tool_type.clone()),
                            cache_control: None,
                        },
                    )),
                    "text_editor_20250728" => {
                        let max_characters = config.get("max_characters").and_then(|v| v.as_i64());
                        Ok(generated::Tool::TextEditor20250728(
                            generated::TextEditor20250728 {
                                name: provider_tool
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| provider_tool.tool_type.clone()),
                                cache_control: None,
                                max_characters,
                            },
                        ))
                    }
                    unknown => Ok(generated::Tool::Unknown {
                        tool_type: unknown.to_string(),
                        name: provider_tool.name.unwrap_or_else(|| unknown.to_string()),
                        config: config
                            .as_object()
                            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default(),
                    }),
                }
            }
        }
    }
}

/// Convert Anthropic Tool to Lingua Tool
impl TryFromLLM<generated::Tool> for Tool {
    type Error = String;

    fn try_from(tool: generated::Tool) -> Result<Self, Self::Error> {
        match tool {
            generated::Tool::Custom(custom) => {
                // input_schema is now serde_json::Value, use it directly
                Ok(Tool::Client(ClientTool {
                    name: custom.name,
                    description: custom.description.unwrap_or_default(),
                    input_schema: custom.input_schema,
                    provider_options: None,
                }))
            }
            generated::Tool::Bash20250124(bash) => Ok(Tool::Provider(ProviderTool {
                tool_type: "bash_20250124".to_string(),
                name: Some(bash.name),
                config: None,
            })),
            generated::Tool::TextEditor20250124(editor) => Ok(Tool::Provider(ProviderTool {
                tool_type: "text_editor_20250124".to_string(),
                name: Some(editor.name),
                config: None,
            })),
            generated::Tool::TextEditor20250429(editor) => Ok(Tool::Provider(ProviderTool {
                tool_type: "text_editor_20250429".to_string(),
                name: Some(editor.name),
                config: None,
            })),
            generated::Tool::TextEditor20250728(editor) => {
                let mut cfg = serde_json::Map::new();
                if let Some(max_characters) = editor.max_characters {
                    cfg.insert(
                        "max_characters".to_string(),
                        serde_json::Value::Number(max_characters.into()),
                    );
                }

                let config = if cfg.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(cfg))
                };

                Ok(Tool::Provider(ProviderTool {
                    tool_type: "text_editor_20250728".to_string(),
                    name: Some(editor.name),
                    config,
                }))
            }
            generated::Tool::WebSearch20250305(search) => {
                let mut cfg = serde_json::Map::new();

                if let Some(max_uses) = search.max_uses {
                    cfg.insert(
                        "max_uses".to_string(),
                        serde_json::Value::Number(max_uses.into()),
                    );
                }

                if let Some(allowed) = search.allowed_domains {
                    cfg.insert(
                        "allowed_domains".to_string(),
                        serde_json::Value::Array(
                            allowed.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }

                if let Some(blocked) = search.blocked_domains {
                    cfg.insert(
                        "blocked_domains".to_string(),
                        serde_json::Value::Array(
                            blocked.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                }

                if let Some(location) = search.user_location {
                    if let Ok(location_value) = serde_json::to_value(location) {
                        cfg.insert("user_location".to_string(), location_value);
                    }
                }

                let config = if cfg.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(cfg))
                };

                Ok(Tool::Provider(ProviderTool {
                    tool_type: "web_search_20250305".to_string(),
                    name: Some(search.name),
                    config,
                }))
            }
            generated::Tool::Unknown {
                tool_type,
                name,
                config,
            } => {
                let mut cfg = serde_json::Map::new();
                for (k, v) in config {
                    cfg.insert(k, v);
                }

                let config = if cfg.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(cfg))
                };

                Ok(Tool::Provider(ProviderTool {
                    tool_type,
                    name: Some(name),
                    config,
                }))
            }
        }
    }
}
