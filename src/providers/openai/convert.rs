use super::{self as openai};
use crate::universal::convert::TryFromLLM;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, UserContent, UserContentPart,
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

/// Create a basic InputItem with default values
fn create_basic_input_item(role: openai::InputItemRole, content: String) -> openai::InputItem {
    openai::InputItem {
        role: Some(role),
        content: Some(openai::InputItemContent::String(content)),
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

/// Convert universal Message to OpenAI InputItem (for Responses API)
impl TryFromLLM<Message> for openai::InputItem {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => Ok(create_basic_input_item(
                openai::InputItemRole::System,
                content,
            )),
            Message::User { content } => {
                let content_string = match content {
                    UserContent::String(text) => text,
                    UserContent::Array(_) => {
                        "Complex user content (not yet implemented)".to_string()
                    }
                };
                Ok(create_basic_input_item(
                    openai::InputItemRole::User,
                    content_string,
                ))
            }
            Message::Assistant { content, .. } => {
                let content_string = match content {
                    AssistantContent::String(text) => text,
                    AssistantContent::Array(_) => {
                        "Complex assistant content (not yet implemented)".to_string()
                    }
                };
                Ok(create_basic_input_item(
                    openai::InputItemRole::Assistant,
                    content_string,
                ))
            }
            Message::Tool { content: _ } => {
                // Basic implementation - convert tool to user for now
                Ok(create_basic_input_item(
                    openai::InputItemRole::User,
                    "Tool content (not yet implemented)".to_string(),
                ))
            }
        }
    }
}
