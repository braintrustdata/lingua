use super::generated::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
    ChatCompletionRequestMessageRole, InputItem, InputItemContent, InputItemRole, InputItemType,
};
use crate::universal::convert::TryConvert;
use crate::universal::{AssistantContent, AssistantContentPart, Message, UserContent};
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
impl TryConvert<Vec<InputItem>> for Vec<Message> {
    type Error = ConvertError;

    fn try_convert(inputs: Vec<InputItem>) -> Result<Self, Self::Error> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < inputs.len() {
            let input = &inputs[i];

            // Handle reasoning + message pairs
            if matches!(input.input_item_type, Some(InputItemType::Reasoning)) {
                // Look for the next message item to combine with reasoning
                if i + 1 < inputs.len() {
                    let next_input = &inputs[i + 1];
                    if matches!(next_input.input_item_type, Some(InputItemType::Message)) {
                        // Combine reasoning + message into single assistant message
                        let reasoning_text = extract_reasoning_summary(&input)?;
                        let message_content = extract_assistant_content_from_message(&next_input)?;

                        result.push(Message::Assistant {
                            content: AssistantContent::Array(vec![
                                AssistantContentPart::Reasoning {
                                    text: reasoning_text,
                                    provider_options: None,
                                },
                                message_content,
                            ]),
                        });

                        // Skip the next item since we consumed it
                        i += 2;
                        continue;
                    }
                }

                // Standalone reasoning item
                result.push(Message::Assistant {
                    content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                        text: extract_reasoning_summary(&input)?,
                        provider_options: None,
                    }]),
                });
                i += 1;
            } else {
                // Convert individual item using existing logic
                result.push(convert_single_input_item(input.clone())?);
                i += 1;
            }
        }

        Ok(result)
    }
}

/// Convert a single OpenAI InputItem to universal Message (internal helper)
fn convert_single_input_item(input: InputItem) -> Result<Message, ConvertError> {
    let role = input
        .role
        .ok_or_else(|| ConvertError::MissingRequiredField {
            field: "role".to_string(),
        })?;

    let content = input
        .content
        .ok_or_else(|| ConvertError::MissingRequiredField {
            field: "content".to_string(),
        })?;

    match role {
        InputItemRole::System => {
            let content_text = extract_text_from_content(content)?;
            Ok(Message::System {
                content: content_text,
            })
        }
        InputItemRole::User => {
            let user_content = convert_to_user_content(content)?;
            Ok(Message::User {
                content: user_content,
            })
        }
        InputItemRole::Assistant => {
            let assistant_content = convert_to_assistant_content(content)?;
            Ok(Message::Assistant {
                content: assistant_content,
            })
        }
        InputItemRole::Developer => {
            // Treat developer role as system for now
            let content_text = extract_text_from_content(content)?;
            Ok(Message::System {
                content: content_text,
            })
        }
    }
}

/// Extract reasoning summary from a reasoning InputItem
fn extract_reasoning_summary(input: &InputItem) -> Result<String, ConvertError> {
    if let Some(summary) = &input.summary {
        if !summary.is_empty() {
            // Convert Vec<SummaryText> to String - assuming SummaryText has text field or Display
            let text_parts: Vec<String> = summary
                .iter()
                .map(|s| format!("{:?}", s)) // Use debug format for now
                .collect();
            return Ok(text_parts.join("\n"));
        }
    }
    Ok("Reasoning step".to_string())
}

/// Extract assistant content from a message InputItem
fn extract_assistant_content_from_message(
    input: &InputItem,
) -> Result<AssistantContentPart, ConvertError> {
    use crate::universal::TextContentPart;

    if let Some(content) = &input.content {
        match content {
            InputItemContent::String(text) => Ok(AssistantContentPart::Text(TextContentPart {
                text: text.clone(),
                provider_options: None,
            })),
            InputItemContent::InputContentArray(items) => {
                // For complex content, extract text from the first item that has text
                for item in items {
                    if let Some(text) = &item.text {
                        return Ok(AssistantContentPart::Text(TextContentPart {
                            text: text.clone(),
                            provider_options: None,
                        }));
                    }
                }
                Ok(AssistantContentPart::Text(TextContentPart {
                    text: "Complex assistant content".to_string(),
                    provider_options: None,
                }))
            }
        }
    } else {
        Ok(AssistantContentPart::Text(TextContentPart {
            text: "Empty assistant message".to_string(),
            provider_options: None,
        }))
    }
}

/// Extract text content from InputItemContent (basic implementation)
fn extract_text_from_content(content: InputItemContent) -> Result<String, ConvertError> {
    match content {
        InputItemContent::String(text) => Ok(text),
        InputItemContent::InputContentArray(_) => {
            // For now, just return placeholder for complex content
            Ok("Complex content (not yet implemented)".to_string())
        }
    }
}

/// Convert InputItemContent to UserContent (basic implementation)
fn convert_to_user_content(content: InputItemContent) -> Result<UserContent, ConvertError> {
    match content {
        InputItemContent::String(text) => Ok(UserContent::String(text)),
        InputItemContent::InputContentArray(_) => {
            // For now, just convert to simple string
            Ok(UserContent::String(
                "Complex user content (not yet implemented)".to_string(),
            ))
        }
    }
}

/// Convert InputItemContent to AssistantContent (basic implementation)
fn convert_to_assistant_content(
    content: InputItemContent,
) -> Result<AssistantContent, ConvertError> {
    match content {
        InputItemContent::String(text) => Ok(AssistantContent::String(text)),
        InputItemContent::InputContentArray(_) => {
            // For now, just convert to simple string
            Ok(AssistantContent::String(
                "Complex assistant content (not yet implemented)".to_string(),
            ))
        }
    }
}

/// Convert universal Message to OpenAI ChatCompletionRequestMessage
impl TryFrom<Message> for ChatCompletionRequestMessage {
    type Error = ConvertError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => Ok(ChatCompletionRequestMessage {
                role: ChatCompletionRequestMessageRole::System,
                content: Some(ChatCompletionRequestMessageContent::String(content)),
                name: None,
                audio: None,
                function_call: None,
                refusal: None,
                tool_calls: None,
                tool_call_id: None,
            }),
            Message::User { content } => {
                let openai_content = convert_user_content_to_openai(content)?;
                Ok(ChatCompletionRequestMessage {
                    role: ChatCompletionRequestMessageRole::User,
                    content: Some(openai_content),
                    name: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls: None,
                    tool_call_id: None,
                })
            }
            Message::Assistant { content } => {
                let openai_content = convert_assistant_content_to_openai(content)?;
                Ok(ChatCompletionRequestMessage {
                    role: ChatCompletionRequestMessageRole::Assistant,
                    content: Some(openai_content),
                    name: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls: None, // TODO: Handle tool calls from assistant content
                    tool_call_id: None,
                })
            }
            Message::Tool { content: _ } => {
                // Basic implementation - convert tool to user message for now
                Ok(ChatCompletionRequestMessage {
                    role: ChatCompletionRequestMessageRole::Tool,
                    content: Some(ChatCompletionRequestMessageContent::String(
                        "Tool content (not yet implemented)".to_string(),
                    )),
                    name: None,
                    audio: None,
                    function_call: None,
                    refusal: None,
                    tool_calls: None,
                    tool_call_id: None,
                })
            }
        }
    }
}

/// Convert UserContent to OpenAI ChatCompletionRequestMessageContent
fn convert_user_content_to_openai(
    content: UserContent,
) -> Result<ChatCompletionRequestMessageContent, ConvertError> {
    match content {
        UserContent::String(text) => Ok(ChatCompletionRequestMessageContent::String(text)),
        UserContent::Array(_) => {
            // For now, convert complex content to placeholder
            Ok(ChatCompletionRequestMessageContent::String(
                "Complex user content (not yet implemented)".to_string(),
            ))
        }
    }
}

/// Convert AssistantContent to OpenAI ChatCompletionRequestMessageContent
fn convert_assistant_content_to_openai(
    content: AssistantContent,
) -> Result<ChatCompletionRequestMessageContent, ConvertError> {
    match content {
        AssistantContent::String(text) => Ok(ChatCompletionRequestMessageContent::String(text)),
        AssistantContent::Array(_) => {
            // For now, convert complex content to placeholder
            Ok(ChatCompletionRequestMessageContent::String(
                "Complex assistant content (not yet implemented)".to_string(),
            ))
        }
    }
}

/// Create a basic InputItem with default values
fn create_basic_input_item(role: InputItemRole, content: String) -> InputItem {
    InputItem {
        role: Some(role),
        content: Some(InputItemContent::String(content)),
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
impl TryConvert<Message> for InputItem {
    type Error = ConvertError;

    fn try_convert(message: Message) -> Result<Self, Self::Error> {
        match message {
            Message::System { content } => {
                Ok(create_basic_input_item(InputItemRole::System, content))
            }
            Message::User { content } => {
                let content_string = match content {
                    UserContent::String(text) => text,
                    UserContent::Array(_) => {
                        "Complex user content (not yet implemented)".to_string()
                    }
                };
                Ok(create_basic_input_item(InputItemRole::User, content_string))
            }
            Message::Assistant { content } => {
                let content_string = match content {
                    AssistantContent::String(text) => text,
                    AssistantContent::Array(_) => {
                        "Complex assistant content (not yet implemented)".to_string()
                    }
                };
                Ok(create_basic_input_item(
                    InputItemRole::Assistant,
                    content_string,
                ))
            }
            Message::Tool { content: _ } => {
                // Basic implementation - convert tool to user for now
                Ok(create_basic_input_item(
                    InputItemRole::User,
                    "Tool content (not yet implemented)".to_string(),
                ))
            }
        }
    }
}
