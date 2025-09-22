use crate::providers::openai::generated as openai;
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart,
    ToolResultContentPart, UserContent, UserContentPart,
};
use std::fmt;

/// Errors that can occur during conversion between OpenAI and universal formats
#[derive(Debug)]
pub enum ConvertError {
    UnsupportedInputType,
    MissingRequiredField { field: String },
    InvalidRole { role: String },
    ContentConversionFailed { reason: String },
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::UnsupportedInputType => write!(f, "Unsupported input item type"),
            ConvertError::MissingRequiredField { field } => {
                write!(f, "Missing required field: {}", field)
            }
            ConvertError::InvalidRole { role } => write!(f, "Invalid role: {}", role),
            ConvertError::ContentConversionFailed { reason } => {
                write!(f, "Content conversion failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for ConvertError {}

/// Convert OpenAI InputItem collection to universal Message collection
/// This handles OpenAI-specific logic for combining or transforming multiple items
impl TryFromLLM<Vec<openai::InputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(inputs: Vec<openai::InputItem>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();
        for mut input in inputs {
            match input.input_item_type {
                Some(openai::InputItemType::Reasoning) => {
                    let mut summaries = vec![];
                    let mut first = true;
                    for summary in input.summary.unwrap_or_default() {
                        summaries.push(AssistantContentPart::Reasoning {
                            text: summary.text,
                            // OpenAI returns encrypted content on the message level, but may
                            // return multiple summary parts. To keep it simple, we just match this
                            // convention by putting the encrypted content on the first part.
                            encrypted_content: if first {
                                first = false;
                                input.encrypted_content.take()
                            } else {
                                None
                            },
                        });
                    }

                    if summaries.is_empty() {
                        // Handle case where there are no summary parts (empty reasoning). This way
                        // we stil get the encrypted content and make it clear that there was a
                        // reasoning step.
                        summaries.push(AssistantContentPart::Reasoning {
                            text: "".to_string(),
                            encrypted_content: input.encrypted_content.take(),
                        });
                    }

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(summaries),
                        id: input.id,
                    });
                }
                _ => {
                    let role = input
                        .role
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "role".to_string(),
                        })?;

                    let content =
                        input
                            .content
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "content".to_string(),
                            })?;

                    result.push(match role {
                        openai::InputItemRole::System | openai::InputItemRole::Developer => {
                            Message::System {
                                content: TryFromLLM::try_from(content)?,
                            }
                        }
                        openai::InputItemRole::User => Message::User {
                            content: TryFromLLM::try_from(content)?,
                        },
                        openai::InputItemRole::Assistant => Message::Assistant {
                            id: input.id,
                            content: TryFromLLM::try_from(content)?,
                        },
                    });
                }
            };
        }

        Ok(result)
    }
}

impl TryFromLLM<openai::InputItemContent> for UserContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => UserContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                UserContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

impl TryFromLLM<openai::InputContent> for UserContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options: None,
                })
            }
            // TODO: ToolCall and ToolResult content types - not yet implemented in generated types
            openai::InputItemContentListType::InputImage => {
                // Extract image URL from the InputContent
                let image_url =
                    value
                        .image_url
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "image_url".to_string(),
                        })?;

                // Preserve detail in provider_options
                let provider_options = if let Some(detail) = &value.detail {
                    let mut options = serde_json::Map::new();
                    options.insert(
                        "detail".to_string(),
                        serde_json::to_value(detail).unwrap_or(serde_json::Value::Null),
                    );
                    Some(crate::universal::message::ProviderOptions { options })
                } else {
                    None
                };

                UserContentPart::Image {
                    image: serde_json::Value::String(image_url),
                    media_type: Some("image/jpeg".to_string()), // Default to JPEG, could be improved
                    provider_options,
                }
            }
            openai::InputItemContentListType::InputAudio => {
                // Handle audio input if needed in the future
                return Err(ConvertError::UnsupportedInputType);
            }
            openai::InputItemContentListType::InputFile => {
                // Handle file input if needed in the future
                return Err(ConvertError::UnsupportedInputType);
            }
            openai::InputItemContentListType::ReasoningText => {
                // Handle reasoning text - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options: None,
                })
            }
            openai::InputItemContentListType::Refusal => {
                // Handle refusal - treat as regular text for now
                UserContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .unwrap_or_else(|| "Content was refused".to_string()),
                    provider_options: None,
                })
            }
        })
    }
}

impl TryFromLLM<openai::InputItemContent> for AssistantContent {
    type Error = ConvertError;

    fn try_from(contents: openai::InputItemContent) -> Result<Self, Self::Error> {
        Ok(match contents {
            openai::InputItemContent::String(text) => AssistantContent::String(text),
            openai::InputItemContent::InputContentArray(parts) => {
                AssistantContent::Array(TryFromLLM::try_from(parts)?)
            }
        })
    }
}

// Add reverse conversions for the reciprocal pattern

impl TryFromLLM<UserContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: UserContent) -> Result<Self, Self::Error> {
        Ok(match content {
            UserContent::String(text) => openai::InputItemContent::String(text),
            UserContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<UserContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: UserContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            UserContentPart::Text(text_part) => openai::InputContent {
                input_content_type: openai::InputItemContentListType::InputText,
                text: Some(text_part.text),
                ..Default::default()
            },
            UserContentPart::Image {
                image,
                provider_options,
                ..
            } => {
                let image_url = match image {
                    serde_json::Value::String(url) => url,
                    _ => return Err(ConvertError::UnsupportedInputType),
                };

                // Extract detail from provider_options if present
                let detail = provider_options
                    .as_ref()
                    .and_then(|opts| opts.options.get("detail"))
                    .and_then(|detail_val| serde_json::from_value(detail_val.clone()).ok());

                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::InputImage,
                    image_url: Some(image_url),
                    detail,
                    ..Default::default()
                }
            }
            _ => return Err(ConvertError::UnsupportedInputType),
        })
    }
}

impl Default for openai::InputContent {
    fn default() -> Self {
        Self {
            text: None,
            input_content_type: openai::InputItemContentListType::InputText,
            detail: None,
            file_id: None,
            image_url: None,
            file_data: None,
            file_url: None,
            filename: None,
            input_audio: None,
            annotations: None,
            logprobs: None,
            refusal: None,
        }
    }
}

impl TryFromLLM<AssistantContent> for openai::InputItemContent {
    type Error = ConvertError;

    fn try_from(content: AssistantContent) -> Result<Self, Self::Error> {
        Ok(match content {
            AssistantContent::String(text) => openai::InputItemContent::String(text),
            AssistantContent::Array(parts) => {
                let input_parts: Result<Vec<_>, _> =
                    parts.into_iter().map(TryFromLLM::try_from).collect();
                openai::InputItemContent::InputContentArray(input_parts?)
            }
        })
    }
}

impl TryFromLLM<AssistantContentPart> for openai::InputContent {
    type Error = ConvertError;

    fn try_from(part: AssistantContentPart) -> Result<Self, Self::Error> {
        Ok(match part {
            AssistantContentPart::Text(text_part) => openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText,
                text: Some(text_part.text),
                annotations: Some(vec![]), // Add empty annotations array
                logprobs: Some(vec![]),    // Add empty logprobs array
                ..Default::default()
            },
            // TODO: ToolCall support - not yet implemented in generated types
            AssistantContentPart::ToolCall { .. } => {
                return Err(ConvertError::UnsupportedInputType);
            }
            _ => return Err(ConvertError::UnsupportedInputType),
        })
    }
}

impl TryFromLLM<openai::InputContent> for AssistantContentPart {
    type Error = ConvertError;

    fn try_from(value: openai::InputContent) -> Result<Self, Self::Error> {
        Ok(match value.input_content_type {
            openai::InputItemContentListType::InputText
            | openai::InputItemContentListType::OutputText => {
                AssistantContentPart::Text(TextContentPart {
                    text: value
                        .text
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "text".to_string(),
                        })?,
                    provider_options: None,
                })
            }
            // TODO: ToolCall content type support - not yet implemented in generated types
            _ => {
                return Err(ConvertError::UnsupportedInputType);
            }
        })
    }
}

/// Default implementation for InputItem
impl Default for openai::InputItem {
    fn default() -> Self {
        Self {
            role: None,
            content: None,
            input_item_type: None,
            status: None,
            id: None,
            queries: None,
            results: None,
            action: None,
            call_id: None,
            pending_safety_checks: None,
            acknowledged_safety_checks: None,
            output: None,
            arguments: None,
            name: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            container_id: None,
            server_label: None,
            tools: None,
            approval_request_id: None,
            approve: None,
            reason: None,
            request_id: None,
            input: None,
            error: None,
            outputs: None,
        }
    }
}

/// Convert universal Message to OpenAI InputItem (for Responses API)
impl TryFromLLM<Message> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::System),
                content: Some(TryFromLLM::try_from(content)?),
                ..Default::default()
            }),
            Message::User { content } => Ok(openai::InputItem {
                role: Some(openai::InputItemRole::User),
                content: Some(TryFromLLM::try_from(content)?),
                ..Default::default()
            }),
            Message::Assistant { content, id } => {
                match content {
                    AssistantContent::String(text) => Ok(openai::InputItem {
                        role: Some(openai::InputItemRole::Assistant),
                        content: Some(openai::InputItemContent::String(text)),
                        id,
                        input_item_type: Some(openai::InputItemType::Message),
                        status: Some(openai::FunctionCallItemStatus::Completed),
                        ..Default::default()
                    }),
                    AssistantContent::Array(parts) => {
                        let mut has_reasoning = false;
                        let mut encrypted_content = None;
                        let mut reasoning_parts: Vec<openai::SummaryText> = vec![];
                        let mut normal_parts: Vec<openai::InputContent> = vec![];

                        for part in parts {
                            match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: ec,
                                } => {
                                    has_reasoning = true;
                                    encrypted_content = ec;
                                    if !text.is_empty() {
                                        reasoning_parts.push(openai::SummaryText {
                                            text,
                                            summary_text_type: openai::SummaryType::SummaryText,
                                        });
                                    }
                                }
                                _ => {
                                    normal_parts.push(TryFromLLM::try_from(part)?);
                                }
                            }
                        }

                        if has_reasoning {
                            if !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed reasoning and normal content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Pure reasoning message - convert to reasoning InputItem
                            let reasoning_item = openai::InputItem {
                                role: None,
                                content: None,
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                // Extract encrypted_content from first reasoning part
                                encrypted_content,
                                ..Default::default()
                            };
                            Ok(reasoning_item)
                        } else {
                            // Mixed content or regular message - use proper conversion
                            Ok(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                id,
                                status: Some(openai::FunctionCallItemStatus::Completed), // Add status field
                                ..Default::default()
                            })
                        }
                    }
                }
            }
            Message::Tool { content } => {
                // Convert tool results to appropriate InputItems
                let mut result_items = Vec::new();

                for tool_part in content {
                    match tool_part {
                        ToolContentPart::ToolResult(tool_result) => {
                            // Create a tool result InputItem
                            result_items.push(openai::InputItem {
                                role: Some(openai::InputItemRole::User), // Tools appear as user messages in OpenAI
                                content: Some(openai::InputItemContent::String(format!(
                                    "Tool result: {}",
                                    serde_json::to_string(&tool_result.output).unwrap_or_default()
                                ))),
                                input_item_type: Some(openai::InputItemType::CustomToolCallOutput),
                                call_id: Some(serde_json::Value::String(
                                    tool_result.tool_call_id.clone(),
                                )),
                                name: Some(tool_result.tool_name.clone()),
                                output: None, // output field is for Refusal type, not tool output
                                ..Default::default()
                            });
                        }
                    }
                }

                // For now, return the first tool result or a placeholder
                result_items.into_iter().next().ok_or_else(|| {
                    ConvertError::ContentConversionFailed {
                        reason: "Empty tool content".to_string(),
                    }
                })
            }
        }
    }
}

/// Convert OutputItem to InputItem for unified processing
/// OutputItem is used in responses while InputItem is used in requests,
/// but they have very similar structure for message content
impl TryFromLLM<openai::OutputItem> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(output_item: openai::OutputItem) -> Result<Self, Self::Error> {
        // Convert OutputItem to InputItem by mapping the fields
        // The main differences are in content type and some field names

        let input_item_type = match output_item.output_item_type {
            openai::OutputItemType::Message => Some(openai::InputItemType::Message),
            openai::OutputItemType::Reasoning => Some(openai::InputItemType::Reasoning),
            // For other types, we might need to map them or handle specially
            _ => None, // Will be handled based on content
        };

        // Convert content from Vec<OutputMessageContent> to InputItemContent
        let content = if let Some(output_content) = output_item.content {
            if output_content.is_empty() {
                None
            } else if output_content.len() == 1 {
                // Single content item - check if we can convert to string
                if output_content[0].output_message_content_type == openai::ContentType::OutputText
                {
                    output_content
                        .into_iter()
                        .next()
                        .unwrap()
                        .text
                        .map(openai::InputItemContent::String)
                } else {
                    // Convert to InputContent array
                    let input_contents: Result<Vec<_>, _> = output_content
                        .into_iter()
                        .map(convert_output_message_content_to_input_content)
                        .collect();
                    Some(openai::InputItemContent::InputContentArray(input_contents?))
                }
            } else {
                // Multiple content items - convert to array
                let input_contents: Result<Vec<_>, _> = output_content
                    .into_iter()
                    .map(convert_output_message_content_to_input_content)
                    .collect();
                Some(openai::InputItemContent::InputContentArray(input_contents?))
            }
        } else {
            None
        };

        // Convert role from MessageRole to InputItemRole
        let role = output_item.role.map(|mr| match mr {
            openai::MessageRole::Assistant => openai::InputItemRole::Assistant,
            // MessageRole only has Assistant variant for outputs
        });

        // Convert status
        let status = output_item.status;

        // Handle reasoning summary conversion - OutputItem has summary field
        let summary = output_item.summary;

        Ok(openai::InputItem {
            role,
            content,
            input_item_type,
            status,
            id: output_item.id,
            summary,
            // Set other fields to None/default - many OutputItem fields don't have InputItem equivalents
            queries: output_item.queries,
            ..Default::default()
        })
    }
}

/// Helper function to convert OutputMessageContent to InputContent
fn convert_output_message_content_to_input_content(
    output_content: openai::OutputMessageContent,
) -> Result<openai::InputContent, ConvertError> {
    match output_content.output_message_content_type {
        openai::ContentType::OutputText => Ok(openai::InputContent {
            input_content_type: openai::InputItemContentListType::OutputText,
            text: output_content.text,
            annotations: output_content.annotations,
            logprobs: output_content.logprobs,
            refusal: output_content.refusal,
            ..Default::default()
        }),
        // TODO: Handle other content types like tool calls when they're properly supported
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText, // Default fallback
                text: output_content.text,
                annotations: output_content.annotations,
                logprobs: output_content.logprobs,
                refusal: output_content.refusal,
                ..Default::default()
            })
        }
    }
}

// ============================================================================
// Chat Completion Conversions
// ============================================================================

/// Convert ChatCompletionRequestMessage to universal Message
impl TryFromLLM<openai::ChatCompletionRequestMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: openai::ChatCompletionRequestMessage) -> Result<Self, Self::Error> {
        match msg.role {
            openai::ChatCompletionRequestMessageRole::System => {
                let content = match msg.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::System { content })
            }
            openai::ChatCompletionRequestMessageRole::User => {
                let content = match msg.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::User { content })
            }
            openai::ChatCompletionRequestMessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add text content if present
                match msg.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        if !text.is_empty() {
                            content_parts.push(AssistantContentPart::Text(TextContentPart {
                                text,
                                provider_options: None,
                            }));
                        }
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let assistant_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(|part| {
                                // Convert ChatCompletionRequestMessageContentPart to AssistantContentPart
                                match part.chat_completion_request_message_content_part_type {
                                    openai::PurpleType::Text => {
                                        if let Some(text) = part.text {
                                            Ok(AssistantContentPart::Text(TextContentPart {
                                                text,
                                                provider_options: None,
                                            }))
                                        } else {
                                            Err(ConvertError::MissingRequiredField { field: "text".to_string() })
                                        }
                                    }
                                    _ => Err(ConvertError::UnsupportedInputType),
                                }
                            })
                            .collect();
                        content_parts.extend(assistant_parts?);
                    }
                    None => {} // Handle empty assistant messages
                }

                // Add tool calls if present
                if let Some(tool_calls) = msg.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = tool_call.function {
                            let input = serde_json::from_str(&function.arguments)
                                .unwrap_or(serde_json::Value::Null);

                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id,
                                tool_name: function.name,
                                input,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = if content_parts.is_empty() {
                    AssistantContent::String(String::new())
                } else if content_parts.len() == 1 {
                    // If there's only one text part, use string format
                    match &content_parts[0] {
                        AssistantContentPart::Text(text_part) => {
                            AssistantContent::String(text_part.text.clone())
                        }
                        _ => AssistantContent::Array(content_parts),
                    }
                } else {
                    AssistantContent::Array(content_parts)
                };

                Ok(Message::Assistant { content, id: None })
            }
            openai::ChatCompletionRequestMessageRole::Developer => {
                // Treat developer messages as system messages in universal format
                let content = match msg.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => {
                        UserContent::String(text)
                    }
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(parts)) => {
                        let user_parts: Result<Vec<_>, _> = parts
                            .into_iter()
                            .map(TryFromLLM::try_from)
                            .collect();
                        UserContent::Array(user_parts?)
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };
                Ok(Message::System { content })
            }
            openai::ChatCompletionRequestMessageRole::Tool => {
                // Tool messages should extract tool_call_id and content
                let content_text = match msg.content {
                    Some(openai::ChatCompletionRequestMessageContent::String(text)) => text,
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(_)) => {
                        return Err(ConvertError::UnsupportedInputType); // Tool messages typically have string content
                    }
                    None => return Err(ConvertError::MissingRequiredField { field: "content".to_string() }),
                };

                let tool_call_id =
                    msg.tool_call_id
                        .ok_or_else(|| ConvertError::MissingRequiredField {
                            field: "tool_call_id".to_string(),
                        })?;

                // Convert to universal Tool message format
                let tool_result = ToolResultContentPart {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: String::new(), // OpenAI doesn't provide tool name in tool messages
                    output: serde_json::Value::String(content_text),
                    provider_options: None,
                };

                Ok(Message::Tool {
                    content: vec![ToolContentPart::ToolResult(tool_result)],
                })
            }
            _ => Err(ConvertError::InvalidRole {
                role: format!("{:?}", msg.role),
            }),
        }
    }
}

/// Convert ChatCompletionRequestMessageContentPart to UserContentPart
impl TryFromLLM<openai::ChatCompletionRequestMessageContentPart> for UserContentPart {
    type Error = ConvertError;

    fn try_from(
        part: openai::ChatCompletionRequestMessageContentPart,
    ) -> Result<Self, Self::Error> {
        match part.chat_completion_request_message_content_part_type {
            openai::PurpleType::Text => {
                if let Some(text) = part.text {
                    Ok(UserContentPart::Text(TextContentPart {
                        text,
                        provider_options: None,
                    }))
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "text".to_string(),
                    })
                }
            }
            openai::PurpleType::ImageUrl => {
                if let Some(image_url) = part.image_url {
                    // Convert ImageUrl to UserContentPart::Image
                    Ok(UserContentPart::Image {
                        image: serde_json::to_value(&image_url.url).unwrap_or_default(),
                        media_type: Some("image/url".to_string()),
                        provider_options: None,
                    })
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "image_url".to_string(),
                    })
                }
            }
            _ => Err(ConvertError::UnsupportedInputType),
        }
    }
}

/// Convert universal Message to ChatCompletionRequestMessage
impl TryFromLLM<Message> for openai::ChatCompletionRequestMessage {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::System { content } => Ok(openai::ChatCompletionRequestMessage {
                role: openai::ChatCompletionRequestMessageRole::System,
                content: Some(convert_user_content_to_chat_completion_content(content)?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                audio: None,
                function_call: None,
                refusal: None,
            }),
            Message::User { content } => Ok(openai::ChatCompletionRequestMessage {
                role: openai::ChatCompletionRequestMessageRole::User,
                content: Some(convert_user_content_to_chat_completion_content(content)?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                audio: None,
                function_call: None,
                refusal: None,
            }),
            Message::Assistant { content, id: _ } => {
                let (text_content, tool_calls) = extract_content_and_tool_calls(content)?;

                Ok(openai::ChatCompletionRequestMessage {
                    role: openai::ChatCompletionRequestMessageRole::Assistant,
                    content: text_content,
                    name: None,
                    tool_calls,
                    tool_call_id: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                })
            }
            Message::Tool { content } => {
                // Extract the tool result content
                let tool_result = content
                    .iter()
                    .map(|part| {
                        let ToolContentPart::ToolResult(result) = part;
                        result
                    })
                    .next()
                    .ok_or_else(|| ConvertError::MissingRequiredField {
                        field: "tool_result".to_string(),
                    })?;

                // Convert output to string for OpenAI
                let content_string = match &tool_result.output {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string(other).unwrap_or_default(),
                };

                Ok(openai::ChatCompletionRequestMessage {
                    role: openai::ChatCompletionRequestMessageRole::Tool,
                    content: Some(openai::ChatCompletionRequestMessageContent::String(
                        content_string,
                    )),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(tool_result.tool_call_id.clone()),
                    audio: None,
                    function_call: None,
                    refusal: None,
                })
            }
        }
    }
}

/// Convert UserContent to ChatCompletionRequestMessageContent
fn convert_user_content_to_chat_completion_content(
    content: UserContent,
) -> Result<openai::ChatCompletionRequestMessageContent, ConvertError> {
    match content {
        UserContent::String(text) => Ok(openai::ChatCompletionRequestMessageContent::String(text)),
        UserContent::Array(parts) => {
            let chat_parts: Result<Vec<_>, _> = parts
                .into_iter()
                .map(convert_user_content_part_to_chat_completion_part)
                .collect();
            Ok(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(chat_parts?))
        }
    }
}

/// Convert UserContentPart to ChatCompletionRequestMessageContentPart
fn convert_user_content_part_to_chat_completion_part(
    part: UserContentPart,
) -> Result<openai::ChatCompletionRequestMessageContentPart, ConvertError> {
    match part {
        UserContentPart::Text(text_part) => Ok(openai::ChatCompletionRequestMessageContentPart {
            text: Some(text_part.text),
            chat_completion_request_message_content_part_type: openai::PurpleType::Text,
            image_url: None,
            input_audio: None,
            file: None,
            refusal: None,
        }),
        UserContentPart::Image {
            image,
            media_type: _,
            provider_options: _,
        } => {
            // Convert image to ImageUrl format
            let url = match image {
                serde_json::Value::String(url) => url,
                _ => return Err(ConvertError::UnsupportedInputType),
            };
            Ok(openai::ChatCompletionRequestMessageContentPart {
                text: None,
                chat_completion_request_message_content_part_type: openai::PurpleType::ImageUrl,
                image_url: Some(openai::ImageUrl { url, detail: None }),
                input_audio: None,
                file: None,
                refusal: None,
            })
        }
        _ => Err(ConvertError::UnsupportedInputType),
    }
}

/// Extract text content and tool calls from AssistantContent
fn extract_content_and_tool_calls(
    content: AssistantContent,
) -> Result<
    (
        Option<openai::ChatCompletionRequestMessageContent>,
        Option<Vec<openai::ToolCall>>,
    ),
    ConvertError,
> {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    match content {
        AssistantContent::String(text) => {
            return Ok((
                Some(openai::ChatCompletionRequestMessageContent::String(text)),
                None,
            ));
        }
        AssistantContent::Array(parts) => {
            for part in parts {
                match part {
                    AssistantContentPart::Text(text_part) => {
                        text_parts.push(text_part.text);
                    }
                    AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        input,
                        ..
                    } => {
                        tool_calls.push(openai::ToolCall {
                            id: tool_call_id,
                            tool_call_type: openai::ToolType::Function,
                            function: Some(openai::PurpleFunction {
                                name: tool_name,
                                arguments: serde_json::to_string(&input).unwrap_or_default(),
                            }),
                            custom: None,
                        });
                    }
                    _ => {
                        // Handle other content types if needed
                    }
                }
            }
        }
    }

    let text_content = if text_parts.is_empty() && !tool_calls.is_empty() {
        None // When we have tool calls but no text, omit content entirely
    } else {
        Some(openai::ChatCompletionRequestMessageContent::String(
            text_parts.join(""),
        ))
    };

    let tool_calls_option = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    Ok((text_content, tool_calls_option))
}

/// Convert ChatCompletionResponseMessage to universal Message
impl TryFromLLM<&openai::ChatCompletionResponseMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: &openai::ChatCompletionResponseMessage) -> Result<Self, Self::Error> {
        match msg.role {
            openai::MessageRole::Assistant => {
                let mut content_parts: Vec<AssistantContentPart> = Vec::new();

                // Add text content if present
                if let Some(text) = &msg.content {
                    if !text.is_empty() {
                        content_parts.push(AssistantContentPart::Text(TextContentPart {
                            text: text.clone(),
                            provider_options: None,
                        }));
                    }
                }

                // Add tool calls if present
                if let Some(tool_calls) = &msg.tool_calls {
                    for tool_call in tool_calls {
                        if let Some(function) = &tool_call.function {
                            // Parse the arguments JSON back to serde_json::Value
                            let input = serde_json::from_str(&function.arguments)
                                .unwrap_or(serde_json::Value::Null);

                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id.clone(),
                                tool_name: function.name.clone(),
                                input,
                                provider_options: None,
                                provider_executed: None,
                            });
                        }
                    }
                }

                let content = if content_parts.is_empty() {
                    AssistantContent::String(String::new())
                } else if content_parts.len() == 1 {
                    // If there's only one text part, use string format
                    match &content_parts[0] {
                        AssistantContentPart::Text(text_part) => {
                            AssistantContent::String(text_part.text.clone())
                        }
                        _ => AssistantContent::Array(content_parts),
                    }
                } else {
                    AssistantContent::Array(content_parts)
                };

                Ok(Message::Assistant { content, id: None })
            }
        }
    }
}

/// Convert universal Message to ChatCompletionResponseMessage
impl TryFromLLM<&Message> for openai::ChatCompletionResponseMessage {
    type Error = ConvertError;

    fn try_from(msg: &Message) -> Result<Self, Self::Error> {
        match msg {
            Message::Assistant { content, id: _ } => {
                let (content_text, tool_calls) = match content {
                    AssistantContent::String(text) => (Some(text.clone()), None),
                    AssistantContent::Array(parts) => {
                        // Extract text from parts and concatenate
                        let texts: Vec<String> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::Text(text_part) => {
                                    Some(text_part.text.clone())
                                }
                                _ => None,
                            })
                            .collect();

                        // Extract tool calls from parts
                        let tool_calls: Vec<openai::ToolCall> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    input,
                                    ..
                                } => Some(openai::ToolCall {
                                    id: tool_call_id.clone(),
                                    tool_call_type: openai::ToolType::Function,
                                    function: Some(openai::PurpleFunction {
                                        name: tool_name.clone(),
                                        arguments: serde_json::to_string(input).unwrap_or_default(),
                                    }),
                                    custom: None,
                                }),
                                _ => None,
                            })
                            .collect();

                        let content_text = if texts.is_empty() {
                            None
                        } else {
                            Some(texts.join(""))
                        };

                        let tool_calls_option = if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        };

                        (content_text, tool_calls_option)
                    }
                };

                Ok(openai::ChatCompletionResponseMessage {
                    role: openai::MessageRole::Assistant,
                    content: content_text,
                    annotations: Some(vec![]), // Hardcode empty annotations for consistency
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls,
                })
            }
            _ => Err(ConvertError::InvalidRole {
                role: format!("{:?}", msg),
            }),
        }
    }
}
