/*!
Lingua → Lingua transformations that run before converting to a provider format.
*/

use crate::capabilities::UniversalCapabilities;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContentPart,
    ToolResultContentPart, UserContent, UserContentPart,
};

/// Applies capability-aware mutations to universal messages.
pub struct UniversalTransformer<'a> {
    messages: &'a mut Vec<Message>,
    capabilities: UniversalCapabilities,
}

impl<'a> UniversalTransformer<'a> {
    /// Create a transformer for a mutable slice of messages.
    pub fn new(messages: &'a mut Vec<Message>, capabilities: UniversalCapabilities) -> Self {
        Self {
            messages,
            capabilities,
        }
    }

    /// Apply all transformations in-place.
    pub fn transform(&mut self) {
        if !self.capabilities.supports_system_messages {
            downgrade_system_messages(self.messages);
        }

        if !self.capabilities.supports_file_attachments {
            remove_file_attachments(self.messages);
        }

        if !self.capabilities.supports_tool_messages {
            remap_tool_messages(self.messages.as_mut_slice());
        }

        if let Some(limit) = self.capabilities.max_message_length {
            enforce_text_limit(self.messages, limit);
        }
    }
}

fn downgrade_system_messages(messages: &mut [Message]) {
    for message in messages.iter_mut() {
        if let Message::System { content } = message {
            let new_content = content.clone();
            *message = Message::User {
                content: new_content,
            };
        }
    }
}

fn remove_file_attachments(messages: &mut [Message]) {
    for message in messages.iter_mut() {
        match message {
            Message::User { content } | Message::System { content } => {
                sanitize_user_content(content);
            }
            Message::Assistant { content, .. } => {
                sanitize_assistant_content(content);
            }
            Message::Tool { .. } => {}
        }
    }
}

fn sanitize_user_content(content: &mut UserContent) {
    if let UserContent::Array(parts) = content {
        for part in parts.iter_mut() {
            if let UserContentPart::File {
                filename,
                media_type,
                ..
            } = part
            {
                let display_name = filename
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| media_type.clone());
                *part = UserContentPart::Text(TextContentPart {
                    text: format!("[file omitted: {}]", display_name),
                    provider_options: None,
                });
            }
        }
    }
}

fn sanitize_assistant_content(content: &mut AssistantContent) {
    if let AssistantContent::Array(parts) = content {
        for part in parts.iter_mut() {
            if let AssistantContentPart::File {
                filename,
                media_type,
                ..
            } = part
            {
                let display_name = filename
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| media_type.clone());
                *part = AssistantContentPart::Text(TextContentPart {
                    text: format!("[assistant file omitted: {}]", display_name),
                    provider_options: None,
                });
            }
        }
    }
}

fn remap_tool_messages(messages: &mut [Message]) {
    for message in messages.iter_mut() {
        if let Message::Tool { content } = message {
            let tool_summary = summarize_tool_content(content.as_slice());
            *message = Message::Assistant {
                content: AssistantContent::String(tool_summary),
                id: None,
            };
        }
    }
}

fn summarize_tool_content(content: &[ToolContentPart]) -> String {
    let mut summaries = Vec::with_capacity(content.len());

    for part in content {
        match part {
            ToolContentPart::ToolResult(ToolResultContentPart {
                tool_call_id,
                tool_name,
                ..
            }) => {
                if tool_name.is_empty() {
                    summaries.push(format!("tool result ({})", tool_call_id));
                } else {
                    summaries.push(format!("{} ({})", tool_name, tool_call_id));
                }
            }
        }
    }

    if summaries.is_empty() {
        "[tool result omitted]".to_string()
    } else {
        format!("[tool results: {}]", summaries.join(", "))
    }
}

fn enforce_text_limit(messages: &mut [Message], limit: usize) {
    for message in messages {
        match message {
            Message::System { content } | Message::User { content } => {
                truncate_user_content(content, limit);
            }
            Message::Assistant { content, .. } => truncate_assistant_content(content, limit),
            Message::Tool { .. } => {}
        }
    }
}

fn truncate_user_content(content: &mut UserContent, limit: usize) {
    match content {
        UserContent::String(text) => truncate(text, limit),
        UserContent::Array(parts) => {
            for part in parts {
                if let UserContentPart::Text(text_part) = part {
                    truncate(&mut text_part.text, limit);
                }
            }
        }
    }
}

fn truncate_assistant_content(content: &mut AssistantContent, limit: usize) {
    match content {
        AssistantContent::String(text) => truncate(text, limit),
        AssistantContent::Array(parts) => {
            for part in parts {
                if let AssistantContentPart::Text(text_part) = part {
                    truncate(&mut text_part.text, limit);
                }
            }
        }
    }
}

fn truncate(text: &mut String, limit: usize) {
    if text.len() > limit {
        text.truncate(limit);
    }
}
