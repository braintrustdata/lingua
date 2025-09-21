use crate::providers::openai::generated as openai;
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart, UserContent,
    UserContentPart,
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
            _ => {
                return Err(ConvertError::UnsupportedInputType);
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
                        ..Default::default()
                    }),
                    AssistantContent::Array(parts) => {
                        // Check if this is a reasoning-only message
                        let reasoning_parts: Vec<_> = parts
                            .iter()
                            .filter_map(|part| match part {
                                AssistantContentPart::Reasoning {
                                    text,
                                    encrypted_content: _,
                                } => Some(openai::SummaryText {
                                    text: text.clone(),
                                    summary_text_type: openai::SummaryType::SummaryText,
                                }),
                                _ => None,
                            })
                            .collect();

                        // Check if this is reasoning-only (either has reasoning parts or is empty array from empty reasoning)
                        let is_reasoning_only = if parts.is_empty() {
                            // Empty array likely came from empty reasoning summary
                            true
                        } else {
                            // Check if all parts are reasoning parts
                            !reasoning_parts.is_empty() && reasoning_parts.len() == parts.len()
                        };

                        if is_reasoning_only {
                            // Pure reasoning message - convert to reasoning InputItem
                            let reasoning_item = openai::InputItem {
                                role: None,
                                content: None,
                                input_item_type: Some(openai::InputItemType::Reasoning),
                                id: id.clone(),
                                summary: Some(reasoning_parts),
                                // Extract encrypted_content from first reasoning part
                                encrypted_content: parts.first().and_then(|part| match part {
                                    AssistantContentPart::Reasoning {
                                        encrypted_content, ..
                                    } => encrypted_content.clone(),
                                    _ => None,
                                }),
                                ..Default::default()
                            };
                            Ok(reasoning_item)
                        } else {
                            // Mixed content or regular message - use proper conversion
                            Ok(openai::InputItem {
                                role: Some(openai::InputItemRole::Assistant),
                                content: Some(TryFromLLM::try_from(AssistantContent::Array(
                                    parts,
                                ))?),
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
