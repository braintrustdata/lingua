use crate::error::ConvertError;
use crate::providers::openai::generated as openai;
use crate::serde_json;
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, ClientTool, Message, ProviderTool, TextContentPart,
    Tool, ToolContentPart, ToolResultContentPart, UserContent, UserContentPart,
};

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
                Some(openai::InputItemType::FunctionCall)
                | Some(openai::InputItemType::CustomToolCall) => {
                    // Function calls are converted to tool calls in assistant messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call call_id".to_string(),
                            })?;
                    let tool_name =
                        input
                            .name
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call name".to_string(),
                            })?;
                    let arguments_str = input.arguments.unwrap_or("{}".to_string());

                    let tool_call_part = AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments: arguments_str.into(),
                        provider_options: None,
                        provider_executed: None,
                    };

                    result.push(Message::Assistant {
                        content: AssistantContent::Array(vec![tool_call_part]),
                        id: input.id.clone(),
                    });
                }
                Some(openai::InputItemType::FunctionCallOutput)
                | Some(openai::InputItemType::CustomToolCallOutput) => {
                    // Function call outputs are converted to tool messages
                    let tool_call_id =
                        input
                            .call_id
                            .ok_or_else(|| ConvertError::MissingRequiredField {
                                field: "function call output call_id".to_string(),
                            })?;

                    let output = input.output.unwrap_or_else(|| "".to_string());

                    let output_value = serde_json::Value::String(output);

                    let tool_result = ToolResultContentPart {
                        tool_call_id,
                        tool_name: String::new(), // OpenAI doesn't provide tool name in output
                        output: output_value,
                        provider_options: None,
                    };

                    result.push(Message::Tool {
                        content: vec![ToolContentPart::ToolResult(tool_result)],
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
                        serde_json::to_value(detail).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: "detail".to_string(),
                                error: e.to_string(),
                            }
                        })?,
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
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "InputAudio content type".to_string(),
                });
            }
            openai::InputItemContentListType::InputFile => {
                // Handle file input if needed in the future
                return Err(ConvertError::UnsupportedInputType {
                    type_info: "InputFile content type".to_string(),
                });
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
                    _ => {
                        return Err(ConvertError::UnsupportedInputType {
                            type_info: format!("Image type must be string URL, got: {:?}", image),
                        })
                    }
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
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("UserContentPart variant: {:?}", part),
                })
            }
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
            AssistantContentPart::ToolCall {
                tool_call_id: _,
                tool_name: _,
                arguments,
                ..
            } => openai::InputContent {
                input_content_type: openai::InputItemContentListType::OutputText,
                text: Some(format!("{}", arguments)),
                annotations: Some(vec![]),
                logprobs: Some(vec![]),
                ..Default::default()
            },
            AssistantContentPart::Reasoning {
                text,
                encrypted_content: _,
            } => {
                // Convert reasoning back to reasoning text content
                openai::InputContent {
                    input_content_type: openai::InputItemContentListType::ReasoningText,
                    text: Some(text),
                    annotations: Some(vec![]),
                    logprobs: Some(vec![]),
                    ..Default::default()
                }
            }
            _ => {
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("AssistantContentPart variant: {:?}", part),
                })
            }
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
                return Err(ConvertError::UnsupportedInputType {
                    type_info: format!("InputContent type: {:?}", value.input_content_type),
                });
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
                        let mut tool_call_info: Option<(String, String, String)> = None; // (tool_call_id, name, arguments, call_id)

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
                                AssistantContentPart::ToolCall {
                                    tool_call_id,
                                    tool_name,
                                    arguments,
                                    provider_options: _,
                                    ..
                                } => {
                                    tool_call_info =
                                        Some((tool_call_id, tool_name, arguments.to_string()));
                                }
                                _ => {
                                    normal_parts.push(TryFromLLM::try_from(part)?);
                                }
                            }
                        }

                        if has_reasoning {
                            if tool_call_info.is_some() || !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed reasoning and other content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            // Pure reasoning message - convert to reasoning InputItem
                            let reasoning_item = openai::InputItem {
                                role: None, // Don't set role for reasoning items - let the original data determine this
                                content: None,
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                encrypted_content,
                                ..Default::default()
                            };
                            Ok(reasoning_item)
                        } else if let Some((call_id, name, arguments)) = tool_call_info {
                            if !normal_parts.is_empty() {
                                return Err(ConvertError::ContentConversionFailed {
                                    reason: "Mixed tool call and normal content parts are not supported in OpenAI format".to_string(),
                                });
                            }

                            let function_call_item = openai::InputItem {
                                role: None, // Preserve original role state - request context function calls don't have roles
                                content: None,
                                input_item_type: Some(openai::InputItemType::FunctionCall),
                                id: id.clone(),
                                call_id: Some(call_id),
                                name: Some(name),
                                arguments: Some(arguments),
                                status: Some(openai::FunctionCallItemStatus::Completed),
                                ..Default::default()
                            };
                            Ok(function_call_item)
                        } else {
                            // Regular message - use normal conversion
                            Ok(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(openai::InputItemContent::InputContentArray(
                                    normal_parts,
                                )),
                                input_item_type: Some(openai::InputItemType::Message),
                                id,
                                status: Some(openai::FunctionCallItemStatus::Completed),
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
                            // Convert tool result output to string for Refusal::String
                            let output_string = match &tool_result.output {
                                serde_json::Value::String(s) => s.clone(),
                                other => serde_json::to_string(other).map_err(|e| {
                                    ConvertError::JsonSerializationFailed {
                                        field: "tool_result_output".to_string(),
                                        error: e.to_string(),
                                    }
                                })?,
                            };

                            // Create a tool result InputItem using FunctionCallOutput type
                            result_items.push(openai::InputItem {
                                role: None,    // Function call outputs don't have roles
                                content: None, // Function call outputs use the output field, not content
                                input_item_type: Some(openai::InputItemType::FunctionCallOutput),
                                call_id: Some(tool_result.tool_call_id.clone()),
                                output: Some(output_string),
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
            Some(openai::OutputItemType::Message) => Some(openai::InputItemType::Message),
            Some(openai::OutputItemType::Reasoning) => Some(openai::InputItemType::Reasoning),
            Some(openai::OutputItemType::FunctionCall) => Some(openai::InputItemType::FunctionCall),
            Some(openai::OutputItemType::CustomToolCall) => {
                Some(openai::InputItemType::CustomToolCall)
            }
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
        // If no role is provided, infer it from the output_item_type only for certain types
        let role = output_item
            .role
            .map(|mr| match mr {
                openai::MessageRole::Assistant => openai::InputItemRole::Assistant,
            })
            .or({
                // Only infer role for regular messages, not for function calls or other items
                // Function calls and other tool-related items should preserve their original role state
                match output_item.output_item_type {
                    Some(openai::OutputItemType::Message) => Some(openai::InputItemRole::Assistant),
                    _ => None, // Don't infer role for function calls, reasoning, and other types
                }
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
            // Preserve structured function call fields
            arguments: output_item.arguments,
            name: output_item.name,
            // Set other fields to None/default - many OutputItem fields don't have InputItem equivalents
            queries: output_item.queries,
            call_id: output_item.call_id,
            results: output_item.results,
            action: output_item.action,
            pending_safety_checks: output_item.pending_safety_checks,
            acknowledged_safety_checks: None,
            output: None,
            encrypted_content: output_item.encrypted_content,
            result: output_item.result,
            code: output_item.code,
            container_id: output_item.container_id,
            outputs: output_item.outputs,
            error: output_item.error,
            server_label: output_item.server_label,
            tools: output_item.tools,
            approval_request_id: None,
            approve: None,
            reason: None,
            request_id: None,
            input: output_item.input,
        })
    }
}

/// Convert InputItem to OutputItem (reverse of OutputItem -> InputItem conversion)
impl TryFromLLM<openai::InputItem> for openai::OutputItem {
    type Error = ConvertError;

    fn try_from(input_item: openai::InputItem) -> Result<Self, Self::Error> {
        // Convert InputItem to OutputItem by mapping the fields
        let output_item_type = match input_item.input_item_type {
            Some(openai::InputItemType::Message) => Some(openai::OutputItemType::Message),
            Some(openai::InputItemType::Reasoning) => Some(openai::OutputItemType::Reasoning),
            Some(openai::InputItemType::FunctionCall) => Some(openai::OutputItemType::FunctionCall),
            Some(openai::InputItemType::CustomToolCall) => {
                Some(openai::OutputItemType::CustomToolCall)
            }
            _ => None,
        };

        // Convert content from InputItemContent to Vec<OutputMessageContent>
        let content = if let Some(input_content) = input_item.content {
            match input_content {
                openai::InputItemContent::String(text) => {
                    // Single string content becomes single OutputMessageContent
                    Some(vec![openai::OutputMessageContent {
                        output_message_content_type: openai::ContentType::OutputText,
                        text: Some(text),
                        annotations: Some(vec![]),
                        logprobs: Some(vec![]),
                        refusal: None,
                    }])
                }
                openai::InputItemContent::InputContentArray(input_contents) => {
                    // Convert InputContent array to OutputMessageContent array
                    let output_contents: Result<Vec<_>, _> = input_contents
                        .into_iter()
                        .map(convert_input_content_to_output_message_content)
                        .collect();
                    Some(output_contents?)
                }
            }
        } else {
            None
        };

        // Convert role from InputItemRole to MessageRole
        let role = input_item.role.and_then(|ir| match ir {
            openai::InputItemRole::Assistant => Some(openai::MessageRole::Assistant),
            _ => None, // OutputItem only supports Assistant role
        });

        Ok(openai::OutputItem {
            role,
            content,
            output_item_type,
            status: input_item.status,
            id: input_item.id,
            summary: input_item.summary,
            arguments: input_item.arguments,
            name: input_item.name,
            queries: input_item.queries,
            call_id: input_item.call_id,
            results: input_item.results,
            action: input_item.action,
            pending_safety_checks: input_item.pending_safety_checks,
            encrypted_content: input_item.encrypted_content,
            result: input_item.result,
            code: input_item.code,
            container_id: input_item.container_id,
            outputs: input_item.outputs,
            error: input_item.error,
            output: input_item.output,
            server_label: input_item.server_label,
            tools: input_item.tools,
            input: input_item.input,
        })
    }
}

impl TryFromLLM<Vec<openai::OutputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_from(messages: Vec<openai::OutputItem>) -> Result<Vec<Message>, Self::Error> {
        let input_items: Vec<openai::InputItem> = messages
            .into_iter()
            .map(TryFromLLM::try_from)
            .collect::<Result<_, _>>()?;
        TryFromLLM::try_from(input_items)
    }
}

/// Convert universal Message collection to OpenAI OutputItem collection
/// This leverages the Message -> InputItem -> OutputItem conversion chain
impl TryFromLLM<Vec<Message>> for Vec<openai::OutputItem> {
    type Error = ConvertError;

    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        // Convert each message to InputItem first, then to OutputItem
        let input_items: Vec<openai::InputItem> = messages
            .into_iter()
            .map(TryFromLLM::try_from)
            .collect::<Result<_, _>>()?;

        // Then convert InputItems to OutputItems
        input_items.into_iter().map(TryFromLLM::try_from).collect()
    }
}

/// Helper function to convert InputContent to OutputMessageContent
fn convert_input_content_to_output_message_content(
    input_content: openai::InputContent,
) -> Result<openai::OutputMessageContent, ConvertError> {
    match input_content.input_content_type {
        openai::InputItemContentListType::OutputText
        | openai::InputItemContentListType::InputText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        openai::InputItemContentListType::ReasoningText => Ok(openai::OutputMessageContent {
            output_message_content_type: openai::ContentType::OutputText,
            text: input_content.text,
            annotations: input_content.annotations,
            logprobs: input_content.logprobs,
            refusal: input_content.refusal,
        }),
        _ => {
            // For other content types, try to preserve as much information as possible
            Ok(openai::OutputMessageContent {
                output_message_content_type: openai::ContentType::OutputText,
                text: input_content.text,
                annotations: input_content.annotations,
                logprobs: input_content.logprobs,
                refusal: input_content.refusal,
            })
        }
    }
}

/// Default implementation for OutputItem
impl Default for openai::OutputItem {
    fn default() -> Self {
        Self {
            content: None,
            id: None,
            role: None,
            status: None,
            output_item_type: None, // Don't add type field if original didn't have it
            queries: None,
            results: None,
            arguments: None,
            call_id: None,
            name: None,
            action: None,
            pending_safety_checks: None,
            encrypted_content: None,
            summary: None,
            result: None,
            code: None,
            container_id: None,
            outputs: None,
            error: None,
            output: None,
            server_label: None,
            tools: None,
            input: None,
        }
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
                                    _ => Err(ConvertError::UnsupportedInputType {
                                        type_info: format!("ChatCompletionRequestMessageContentPart type: {:?}", part.chat_completion_request_message_content_part_type),
                                    }),
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
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id,
                                tool_name: function.name,
                                arguments: function.arguments.into(),
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
                    Some(openai::ChatCompletionRequestMessageContent::ChatCompletionRequestMessageContentPartArray(mut arr)) => {
                        if arr.len() != 1 {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: format!("Tool messages must have a single array element (found {})", arr.len()),
                            });
                        }
                        let part = arr.remove(0);
                        if let Some(text) = part.text {
                            text
                        } else {
                            return Err(ConvertError::UnsupportedInputType {
                                type_info: "Tool content part must have text".to_string(),
                            });
                        }
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
                        image: serde_json::to_value(&image_url.url).map_err(|e| {
                            ConvertError::JsonSerializationFailed {
                                field: "image_url".to_string(),
                                error: e.to_string(),
                            }
                        })?,
                        media_type: Some("image/url".to_string()),
                        provider_options: None,
                    })
                } else {
                    Err(ConvertError::MissingRequiredField {
                        field: "image_url".to_string(),
                    })
                }
            }
            _ => Err(ConvertError::UnsupportedInputType {
                type_info: format!(
                    "ChatCompletionRequestMessageContentPart type: {:?}",
                    part.chat_completion_request_message_content_part_type
                ),
            }),
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
                    other => serde_json::to_string(other).map_err(|e| {
                        ConvertError::JsonSerializationFailed {
                            field: "tool_result_content".to_string(),
                            error: e.to_string(),
                        }
                    })?,
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
                _ => {
                    return Err(ConvertError::UnsupportedInputType {
                        type_info: format!(
                            "Image must be string URL for ChatCompletion, got: {:?}",
                            image
                        ),
                    })
                }
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
        _ => Err(ConvertError::UnsupportedInputType {
            type_info: format!(
                "UserContentPart variant in ChatCompletion conversion: {:?}",
                part
            ),
        }),
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
                        arguments,
                        ..
                    } => {
                        tool_calls.push(openai::ToolCall {
                            id: tool_call_id,
                            tool_call_type: openai::ToolType::Function,
                            function: Some(openai::PurpleFunction {
                                name: tool_name,
                                arguments: arguments.to_string(),
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
                            content_parts.push(AssistantContentPart::ToolCall {
                                tool_call_id: tool_call.id.clone(),
                                tool_name: function.name.clone(),
                                arguments: function.arguments.clone().into(),
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
                                    arguments,
                                    ..
                                } => Some(openai::ToolCall {
                                    id: tool_call_id.clone(),
                                    tool_call_type: openai::ToolType::Function,
                                    function: Some(openai::PurpleFunction {
                                        name: tool_name.clone(),
                                        arguments: arguments.to_string(),
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

// ============================================================================
// Tool Conversions
// ============================================================================

/// Convert Lingua Tool to OpenAI Tool
impl TryFromLLM<Tool> for openai::Tool {
    type Error = ConvertError;

    fn try_from(tool: Tool) -> Result<Self, Self::Error> {
        match tool {
            Tool::Client(client_tool) => {
                let parameters = match client_tool.input_schema {
                    serde_json::Value::Object(map) => map,
                    _ => {
                        return Err(ConvertError::ContentConversionFailed {
                            reason: "input_schema must be a JSON object".to_string(),
                        });
                    }
                };

                let strict = client_tool
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.get("strict"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                Ok(openai::Tool::Function(openai::FunctionTool {
                    name: client_tool.name,
                    description: Some(client_tool.description),
                    parameters,
                    strict,
                }))
            }
            Tool::Provider(provider_tool) => match provider_tool.tool_type.as_str() {
                "computer_use_preview" | "computer_20250124" => {
                    let config = provider_tool
                        .config
                        .unwrap_or_else(|| serde_json::json!({}));
                    let display_width = config
                        .get("display_width_px")
                        .or_else(|| config.get("display_width"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1920);
                    let display_height = config
                        .get("display_height_px")
                        .or_else(|| config.get("display_height"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1080);
                    let environment = config
                        .get("environment")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!("browser"));

                    Ok(openai::Tool::ComputerUsePreview(
                        openai::ComputerUsePreviewTool {
                            display_height,
                            display_width,
                            environment,
                        },
                    ))
                }
                "code_interpreter" => {
                    let container = provider_tool
                        .config
                        .and_then(|c| c.get("container").cloned())
                        .unwrap_or_else(|| serde_json::json!({"type": "auto"}));

                    Ok(openai::Tool::CodeInterpreter(openai::CodeInterpreterTool {
                        container,
                    }))
                }
                "web_search" | "web_search_2025_08_26" => {
                    let config = provider_tool
                        .config
                        .unwrap_or_else(|| serde_json::json!({}));

                    Ok(openai::Tool::WebSearch(openai::WebSearchTool {
                        filters: None,
                        search_context_size: config
                            .get("search_context_size")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        user_location: config.get("user_location").cloned(),
                    }))
                }
                "file_search" => {
                    let config = provider_tool
                        .config
                        .unwrap_or_else(|| serde_json::json!({}));
                    let vector_store_ids = config
                        .get("vector_store_ids")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    let filters = config
                        .get("filters")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let ranking_options = config
                        .get("ranking_options")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    let max_num_results = config.get("max_num_results").and_then(|v| v.as_i64());

                    Ok(openai::Tool::FileSearch(openai::FileSearchTool {
                        filters,
                        max_num_results,
                        ranking_options,
                        vector_store_ids,
                    }))
                }
                "mcp" => {
                    let config = provider_tool
                        .config
                        .unwrap_or_else(|| serde_json::json!({}));
                    Ok(openai::Tool::MCP(openai::MCPTool {
                        server_label: provider_tool.name.unwrap_or_else(|| "mcp".to_string()),
                        authorization: config
                            .get("authorization")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        connector_id: config
                            .get("connector_id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        server_description: config
                            .get("server_description")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        server_url: config
                            .get("server_url")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        allowed_tools: None,
                        headers: None,
                        require_approval: None,
                    }))
                }
                "image_generation" => Ok(openai::Tool::ImageGen(openai::ImageGenTool {
                    background: None,
                    input_fidelity: None,
                    input_image_mask: None,
                    model: None,
                    moderation: None,
                    output_compression: None,
                    output_format: None,
                    partial_images: None,
                    quality: None,
                    size: None,
                })),
                "local_shell" => Ok(openai::Tool::LocalShell(openai::LocalShellTool {})),
                "web_search_preview" => Ok(openai::Tool::WebSearchPreview(
                    openai::WebSearchPreviewTool {
                        search_context_size: None,
                        user_location: None,
                    },
                )),
                unknown => Ok(openai::Tool::Unknown {
                    tool_type: unknown.to_string(),
                    name: provider_tool.name.unwrap_or_else(|| unknown.to_string()),
                    config: provider_tool
                        .config
                        .and_then(|c| {
                            c.as_object().map(|map| {
                                map.iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect::<std::collections::HashMap<_, _>>()
                            })
                        })
                        .unwrap_or_default(),
                }),
            },
        }
    }
}

/// Convert OpenAI Tool to Lingua Tool
impl TryFromLLM<openai::Tool> for Tool {
    type Error = ConvertError;

    fn try_from(tool: openai::Tool) -> Result<Self, Self::Error> {
        match tool {
            openai::Tool::Function(function) => Ok(Tool::Client(ClientTool {
                name: function.name,
                description: function.description.unwrap_or_default(),
                input_schema: serde_json::Value::Object(function.parameters),
                provider_options: if function.strict {
                    Some(serde_json::json!({ "strict": true }))
                } else {
                    None
                },
            })),
            openai::Tool::Custom(custom) => Ok(Tool::Provider(ProviderTool {
                tool_type: "custom".to_string(),
                name: Some(custom.name),
                config: None,
            })),
            openai::Tool::ComputerUsePreview(computer) => {
                let mut config = serde_json::Map::new();
                config.insert(
                    "display_width_px".to_string(),
                    serde_json::Value::Number(computer.display_width.into()),
                );
                config.insert(
                    "display_height_px".to_string(),
                    serde_json::Value::Number(computer.display_height.into()),
                );
                if let Ok(env) = serde_json::to_value(computer.environment) {
                    config.insert("environment".to_string(), env);
                }

                Ok(Tool::Provider(ProviderTool {
                    tool_type: "computer_use_preview".to_string(),
                    name: None,
                    config: Some(serde_json::Value::Object(config)),
                }))
            }
            openai::Tool::CodeInterpreter(code) => {
                let config = serde_json::json!({ "container": code.container });
                Ok(Tool::Provider(ProviderTool {
                    tool_type: "code_interpreter".to_string(),
                    name: None,
                    config: Some(config),
                }))
            }
            openai::Tool::WebSearch(search) => {
                let mut config = serde_json::Map::new();
                if let Some(context_size) = search.search_context_size {
                    if let Ok(v) = serde_json::to_value(context_size) {
                        config.insert("search_context_size".to_string(), v);
                    }
                }
                if let Some(location) = search.user_location {
                    if let Ok(v) = serde_json::to_value(location) {
                        config.insert("user_location".to_string(), v);
                    }
                }
                Ok(Tool::Provider(ProviderTool {
                    tool_type: "web_search".to_string(),
                    name: None,
                    config: if config.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(config))
                    },
                }))
            }
            openai::Tool::WebSearchPreview(search) => {
                let mut config = serde_json::Map::new();
                if let Some(loc) = search.user_location {
                    if let Ok(v) = serde_json::to_value(loc) {
                        config.insert("user_location".to_string(), v);
                    }
                }
                Ok(Tool::Provider(ProviderTool {
                    tool_type: "web_search_preview".to_string(),
                    name: None,
                    config: if config.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(config))
                    },
                }))
            }
            openai::Tool::FileSearch(file_search) => {
                let mut config = serde_json::Map::new();
                config.insert(
                    "vector_store_ids".to_string(),
                    serde_json::Value::Array(
                        file_search
                            .vector_store_ids
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                );
                if let Some(max) = file_search.max_num_results {
                    config.insert(
                        "max_num_results".to_string(),
                        serde_json::Value::Number(max.into()),
                    );
                }
                if let Some(filters) = file_search.filters {
                    if let Ok(v) = serde_json::to_value(filters) {
                        config.insert("filters".to_string(), v);
                    }
                }
                if let Some(ranking) = file_search.ranking_options {
                    if let Ok(v) = serde_json::to_value(ranking) {
                        config.insert("ranking_options".to_string(), v);
                    }
                }
                Ok(Tool::Provider(ProviderTool {
                    tool_type: "file_search".to_string(),
                    name: None,
                    config: Some(serde_json::Value::Object(config)),
                }))
            }
            openai::Tool::MCP(mcp) => {
                let mut config = serde_json::Map::new();
                if let Some(auth) = mcp.authorization {
                    config.insert("authorization".to_string(), serde_json::Value::String(auth));
                }
                if let Some(connector) = mcp.connector_id {
                    if let Ok(v) = serde_json::to_value(connector) {
                        config.insert("connector_id".to_string(), v);
                    }
                }
                if let Some(desc) = mcp.server_description {
                    config.insert(
                        "server_description".to_string(),
                        serde_json::Value::String(desc),
                    );
                }
                if let Some(url) = mcp.server_url {
                    config.insert("server_url".to_string(), serde_json::Value::String(url));
                }
                Ok(Tool::Provider(ProviderTool {
                    tool_type: "mcp".to_string(),
                    name: Some(mcp.server_label),
                    config: if config.is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::Object(config))
                    },
                }))
            }
            openai::Tool::ImageGen(_) => Ok(Tool::Provider(ProviderTool {
                tool_type: "image_generation".to_string(),
                name: None,
                config: None,
            })),
            openai::Tool::LocalShell(_) => Ok(Tool::Provider(ProviderTool {
                tool_type: "local_shell".to_string(),
                name: None,
                config: None,
            })),
            openai::Tool::Unknown {
                tool_type,
                name,
                config,
            } => {
                let config = if config.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(
                        config.into_iter().collect::<serde_json::Map<_, _>>(),
                    ))
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

// ============================================================================
// ToolElement Conversions (for Chat Completions API)
// ============================================================================
//
// ToolElement is the correct type for OpenAI's Chat Completions API.
// It uses a nested structure: { "type": "function", "function": {...} }
// which is different from the flat discriminated union used by other APIs.

/// Convert Lingua Tool to OpenAI ToolElement (for Chat Completions API)
impl TryFromLLM<Tool> for openai::ToolElement {
    type Error = ConvertError;

    fn try_from(tool: Tool) -> Result<Self, Self::Error> {
        match tool {
            Tool::Client(client_tool) => {
                let parameters = match client_tool.input_schema {
                    serde_json::Value::Object(map) => {
                        let mut params = std::collections::HashMap::new();
                        for (key, value) in map {
                            params.insert(key, Some(value));
                        }
                        Some(params)
                    }
                    _ => {
                        return Err(ConvertError::ContentConversionFailed {
                            reason: "input_schema must be a JSON object".to_string(),
                        });
                    }
                };

                let strict = client_tool
                    .provider_options
                    .as_ref()
                    .and_then(|opts| opts.get("strict"))
                    .and_then(|v| v.as_bool());

                Ok(openai::ToolElement {
                    tool_type: openai::ToolType::Function,
                    function: Some(openai::FunctionObject {
                        name: client_tool.name,
                        description: Some(client_tool.description),
                        parameters,
                        strict,
                    }),
                    custom: None,
                })
            }
            Tool::Provider(provider_tool) => {
                // Chat Completions API only supports function tools.
                // Provider tools (web_search, computer_use, etc.) should use different APIs
                // or be passed via specific fields (like web_search_options).
                Err(ConvertError::ContentConversionFailed {
                    reason: format!(
                        "Provider tool '{}' cannot be converted to ToolElement. \
                         Chat Completions API only supports function tools. \
                         Use Tool enum for other APIs or pass provider tools via specific options.",
                        provider_tool.tool_type
                    ),
                })
            }
        }
    }
}

/// Convert OpenAI ToolElement to Lingua Tool
impl TryFromLLM<openai::ToolElement> for Tool {
    type Error = ConvertError;

    fn try_from(tool: openai::ToolElement) -> Result<Self, Self::Error> {
        match tool.tool_type {
            openai::ToolType::Function => {
                let function = tool.function.ok_or(ConvertError::MissingRequiredField {
                    field: "function".to_string(),
                })?;

                let mut schema_map = serde_json::Map::new();
                if let Some(params) = function.parameters {
                    for (key, value) in params {
                        if let Some(v) = value {
                            schema_map.insert(key, v);
                        }
                    }
                }

                Ok(Tool::Client(ClientTool {
                    name: function.name,
                    description: function.description.unwrap_or_default(),
                    input_schema: serde_json::Value::Object(schema_map),
                    provider_options: function
                        .strict
                        .map(|strict| serde_json::json!({ "strict": strict })),
                }))
            }
            openai::ToolType::Custom => {
                let custom = tool.custom.ok_or(ConvertError::MissingRequiredField {
                    field: "custom".to_string(),
                })?;

                Ok(Tool::Provider(ProviderTool {
                    tool_type: "custom".to_string(),
                    name: Some(custom.name),
                    config: None,
                }))
            }
        }
    }
}
