use super::{parse_tool_call_arguments, Value};
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart,
    ToolResultContentPart, UserContent, UserContentPart,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Deserialize)]
struct KindMessage {
    role: Role,
    content: Vec<Part>,
}

#[derive(Deserialize)]
struct WrappedMessage {
    message: KindMessage,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Part {
    Text {
        value: String,
    },
    ToolCall {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        content: Value,
    },
    ToolResult {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        #[serde(flatten)]
        result: Observation,
    },
    ThinkingBlock {
        summaries: Vec<Summary>,
        metadata: ThinkingMetadata,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    observation: Value,
    is_error: bool,
}

#[derive(Deserialize)]
struct Summary {
    summary: String,
}

#[derive(Deserialize)]
struct ThinkingMetadata {
    #[serde(rename = "thoughtSignature")]
    thought_signature: String,
}

fn text_part(text: String) -> TextContentPart {
    TextContentPart {
        text,
        encrypted_content: None,
        cache_control: None,
        provider_options: None,
    }
}

impl Part {
    fn user(self) -> Option<UserContentPart> {
        match self {
            Self::Text { value } => Some(UserContentPart::Text(text_part(value))),
            _ => None,
        }
    }

    fn assistant(self) -> Option<AssistantContentPart> {
        match self {
            Self::Text { value } => Some(AssistantContentPart::Text(text_part(value))),
            Self::ToolCall {
                tool_call_id,
                tool_name,
                content,
            } => Some(AssistantContentPart::ToolCall {
                tool_call_id,
                tool_name,
                arguments: parse_tool_call_arguments(Some(content))?,
                caller: None,
                encrypted_content: None,
                provider_options: None,
                status: None,
                provider_executed: None,
            }),
            Self::ThinkingBlock {
                summaries,
                metadata,
            } => Some(AssistantContentPart::Reasoning {
                text: summaries
                    .into_iter()
                    .map(|item| item.summary)
                    .collect::<Vec<_>>()
                    .join("\n"),
                encrypted_content: Some(metadata.thought_signature),
            }),
            _ => None,
        }
    }

    fn tool(self) -> Option<ToolContentPart> {
        match self {
            Self::ToolResult {
                tool_call_id,
                tool_name,
                result,
            } => Some(ToolContentPart::ToolResult(ToolResultContentPart {
                tool_call_id,
                tool_name,
                output: crate::serde_json::to_value(result).ok()?,
                custom_tool_call: None,
                caller: None,
                provider_options: None,
            })),
            _ => None,
        }
    }
}

impl KindMessage {
    fn into_message(self) -> Option<Message> {
        match self.role {
            Role::Assistant => Some(Message::Assistant {
                content: AssistantContent::Array(
                    self.content
                        .into_iter()
                        .map(Part::assistant)
                        .collect::<Option<_>>()?,
                ),
                id: None,
            }),
            Role::Tool => Some(Message::Tool {
                content: self
                    .content
                    .into_iter()
                    .map(Part::tool)
                    .collect::<Option<_>>()?,
            }),
            role => {
                let content = UserContent::Array(
                    self.content
                        .into_iter()
                        .map(Part::user)
                        .collect::<Option<_>>()?,
                );
                Some(match role {
                    Role::System => Message::System { content },
                    Role::Developer => Message::Developer { content },
                    Role::User => Message::User { content },
                    _ => unreachable!(),
                })
            }
        }
    }
}

pub(super) fn parse_message(data: &Value) -> Option<Vec<Message>> {
    let message = KindMessage::deserialize(data)
        .or_else(|_| WrappedMessage::deserialize(data).map(|wrapped| wrapped.message))
        .ok()?;
    Some(vec![message.into_message()?])
}

pub(super) fn parse_output(data: &Value) -> Option<Vec<Message>> {
    let content = Vec::<Part>::deserialize(data).ok()?;
    if content.is_empty() {
        return None;
    }
    Some(vec![KindMessage {
        role: Role::Assistant,
        content,
    }
    .into_message()?])
}
