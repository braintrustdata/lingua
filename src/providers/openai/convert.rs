use super::generated::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContent,
    ChatCompletionRequestMessageRole, InputItem, InputItemContent, InputItemRole, InputItemType,
};
use crate::universal::{AssistantContent, AssistantContentPart, ModelMessage, UserContent};
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

/// Convert OpenAI InputItem to universal ModelMessage
impl TryFrom<InputItem> for ModelMessage {
    type Error = ConvertError;

    fn try_from(input: InputItem) -> Result<Self, Self::Error> {
        if matches!(input.input_item_type, Some(InputItemType::Reasoning)) {
            return Ok(ModelMessage::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::Reasoning {
                    text: "Reasoning content (not yet implemented)".to_string(),
                    provider_options: None,
                }]),
            });
        }

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
                Ok(ModelMessage::System {
                    content: content_text,
                })
            }
            InputItemRole::User => {
                let user_content = convert_to_user_content(content)?;
                Ok(ModelMessage::User {
                    content: user_content,
                })
            }
            InputItemRole::Assistant => {
                let assistant_content = convert_to_assistant_content(content)?;
                Ok(ModelMessage::Assistant {
                    content: assistant_content,
                })
            }
            InputItemRole::Developer => {
                // Treat developer role as system for now
                let content_text = extract_text_from_content(content)?;
                Ok(ModelMessage::System {
                    content: content_text,
                })
            }
        }
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

/// Convert universal ModelMessage to OpenAI ChatCompletionRequestMessage
impl TryFrom<ModelMessage> for ChatCompletionRequestMessage {
    type Error = ConvertError;

    fn try_from(message: ModelMessage) -> Result<Self, Self::Error> {
        match message {
            ModelMessage::System { content } => Ok(ChatCompletionRequestMessage {
                role: ChatCompletionRequestMessageRole::System,
                content: Some(ChatCompletionRequestMessageContent::String(content)),
                name: None,
                audio: None,
                function_call: None,
                refusal: None,
                tool_calls: None,
                tool_call_id: None,
            }),
            ModelMessage::User { content } => {
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
            ModelMessage::Assistant { content } => {
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
            ModelMessage::Tool { content: _ } => {
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
        input_item_type: Some(InputItemType::Message),
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

/// Convert universal ModelMessage to OpenAI InputItem (for Responses API)
impl TryFrom<ModelMessage> for InputItem {
    type Error = ConvertError;

    fn try_from(message: ModelMessage) -> Result<Self, Self::Error> {
        match message {
            ModelMessage::System { content } => {
                Ok(create_basic_input_item(InputItemRole::System, content))
            }
            ModelMessage::User { content } => {
                let content_string = match content {
                    UserContent::String(text) => text,
                    UserContent::Array(_) => {
                        "Complex user content (not yet implemented)".to_string()
                    }
                };
                Ok(create_basic_input_item(InputItemRole::User, content_string))
            }
            ModelMessage::Assistant { content } => {
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
            ModelMessage::Tool { content: _ } => {
                // Basic implementation - convert tool to user for now
                Ok(create_basic_input_item(
                    InputItemRole::User,
                    "Tool content (not yet implemented)".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_message_to_openai_system() {
        let msg = ModelMessage::System {
            content: "You are a helpful assistant".to_string(),
        };

        let openai_msg: Result<ChatCompletionRequestMessage, _> = msg.try_into();
        assert!(openai_msg.is_ok());

        let openai_msg = openai_msg.unwrap();
        assert_eq!(openai_msg.role, ChatCompletionRequestMessageRole::System);

        if let Some(ChatCompletionRequestMessageContent::String(content)) = openai_msg.content {
            assert_eq!(content, "You are a helpful assistant");
        } else {
            panic!("Expected string content");
        }
    }

    #[test]
    fn test_model_message_to_openai_user() {
        let msg = ModelMessage::User {
            content: UserContent::String("Hello, world!".to_string()),
        };

        let openai_msg: Result<ChatCompletionRequestMessage, _> = msg.try_into();
        assert!(openai_msg.is_ok());

        let openai_msg = openai_msg.unwrap();
        assert_eq!(openai_msg.role, ChatCompletionRequestMessageRole::User);

        if let Some(ChatCompletionRequestMessageContent::String(content)) = openai_msg.content {
            assert_eq!(content, "Hello, world!");
        } else {
            panic!("Expected string content");
        }
    }

    // Note: InputItem has many required fields in the generated struct,
    // so we'll skip testing the InputItem -> ModelMessage conversion for now
    // and focus on testing the ModelMessage -> OpenAI conversion which is more important
}
