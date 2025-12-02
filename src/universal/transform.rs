/*!
Lingua → Lingua transformations that run before converting to a provider format.

This module provides universal transformations that are common across multiple
providers, mirroring the proxy's behavior:

- **Message flattening**: Merge consecutive messages of the same role
  (needed for Anthropic, Google, Bedrock)
- **System message extraction**: Extract system messages to a separate parameter
  (needed for Anthropic, Google, Bedrock)
*/

use crate::capabilities::universal::UniversalCapabilities;
use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContent, UserContent,
    UserContentPart,
};

/// Result of applying universal transformations.
#[derive(Debug, Default)]
pub struct TransformResult {
    /// System messages extracted from the message list.
    /// These should be passed to the provider's system parameter.
    pub system_messages: Vec<UserContent>,
}

/// Applies universal transformations to messages before provider conversion.
///
/// This transformer handles transformations that are common across multiple
/// providers, reducing duplication in provider-specific converters.
///
/// # Example
///
/// ```
/// use lingua::universal::{Message, UserContent, UniversalTransformer};
/// use lingua::capabilities::universal::UniversalCapabilities;
///
/// let mut messages = vec![
///     Message::System { content: UserContent::String("You are helpful".into()) },
///     Message::User { content: UserContent::String("Hello".into()) },
/// ];
///
/// let caps = UniversalCapabilities::for_provider("anthropic");
/// let mut transformer = UniversalTransformer::new(&mut messages, caps);
/// let result = transformer.transform();
///
/// // System messages are extracted
/// assert_eq!(result.system_messages.len(), 1);
/// // Only user message remains
/// assert_eq!(messages.len(), 1);
/// ```
pub struct UniversalTransformer<'a> {
    messages: &'a mut Vec<Message>,
    capabilities: UniversalCapabilities,
}

impl<'a> UniversalTransformer<'a> {
    /// Create a transformer for a mutable vector of messages.
    pub fn new(messages: &'a mut Vec<Message>, capabilities: UniversalCapabilities) -> Self {
        Self {
            messages,
            capabilities,
        }
    }

    /// Apply all transformations based on capabilities.
    ///
    /// Returns extracted system messages (if `system_messages_separate` is true).
    pub fn transform(&mut self) -> TransformResult {
        let system_messages = if self.capabilities.system_messages_separate {
            self.extract_system_messages()
        } else {
            vec![]
        };

        if self.capabilities.requires_message_flattening {
            self.flatten_consecutive_messages();
        }

        TransformResult { system_messages }
    }

    /// Extract system messages from the message list.
    ///
    /// Removes all `Message::System` variants and returns their content.
    /// This mirrors the proxy's behavior for Anthropic, Google, and Bedrock
    /// where system messages are passed as a separate parameter.
    fn extract_system_messages(&mut self) -> Vec<UserContent> {
        let mut system_contents = Vec::new();
        self.messages.retain(|msg| {
            if let Message::System { content } = msg {
                system_contents.push(content.clone());
                false
            } else {
                true
            }
        });
        system_contents
    }

    /// Merge consecutive messages of the same role.
    ///
    /// This mirrors the proxy's `flattenAnthropicMessages()`, `flattenMessages()`,
    /// and the inline flattening in `openAIMessagesToGoogleMessages()`.
    ///
    /// For example:
    /// - User("Hello") + User("World") → User(["Hello", "World"])
    /// - Assistant("Hi") + Assistant("there") → Assistant(["Hi", "there"])
    fn flatten_consecutive_messages(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        let mut result: Vec<Message> = Vec::with_capacity(self.messages.len());

        for msg in self.messages.drain(..) {
            if let Some(last) = result.last_mut() {
                if can_merge(last, &msg) {
                    merge_messages(last, msg);
                    continue;
                }
            }
            result.push(msg);
        }

        *self.messages = result;
    }
}

/// Check if two messages can be merged (same role).
fn can_merge(a: &Message, b: &Message) -> bool {
    matches!(
        (a, b),
        (Message::User { .. }, Message::User { .. })
            | (Message::Assistant { .. }, Message::Assistant { .. })
            | (Message::System { .. }, Message::System { .. })
            | (Message::Tool { .. }, Message::Tool { .. })
    )
}

/// Merge message `b` into message `a`.
fn merge_messages(a: &mut Message, b: Message) {
    match (a, b) {
        (Message::User { content: a_content }, Message::User { content: b_content }) => {
            merge_user_content(a_content, b_content);
        }
        (
            Message::Assistant {
                content: a_content, ..
            },
            Message::Assistant {
                content: b_content, ..
            },
        ) => {
            merge_assistant_content(a_content, b_content);
        }
        (Message::System { content: a_content }, Message::System { content: b_content }) => {
            merge_user_content(a_content, b_content);
        }
        (Message::Tool { content: a_content }, Message::Tool { content: b_content }) => {
            merge_tool_content(a_content, b_content);
        }
        _ => {} // Can't merge different roles
    }
}

/// Merge two UserContent values.
fn merge_user_content(a: &mut UserContent, b: UserContent) {
    // Replace `a` with a temporary empty array, get the old value
    let old_a = std::mem::replace(a, UserContent::Array(vec![]));
    let a_parts = user_content_to_parts(old_a);
    let b_parts = user_content_to_parts(b);
    let mut merged = a_parts;
    merged.extend(b_parts);
    *a = UserContent::Array(merged);
}

/// Merge two AssistantContent values.
fn merge_assistant_content(a: &mut AssistantContent, b: AssistantContent) {
    // Replace `a` with a temporary empty array, get the old value
    let old_a = std::mem::replace(a, AssistantContent::Array(vec![]));
    let a_parts = assistant_content_to_parts(old_a);
    let b_parts = assistant_content_to_parts(b);
    let mut merged = a_parts;
    merged.extend(b_parts);
    *a = AssistantContent::Array(merged);
}

/// Merge two ToolContent values (Vec<ToolContentPart>).
fn merge_tool_content(a: &mut ToolContent, b: ToolContent) {
    a.extend(b);
}

/// Convert UserContent to Vec<UserContentPart>.
fn user_content_to_parts(content: UserContent) -> Vec<UserContentPart> {
    match content {
        UserContent::String(text) => vec![UserContentPart::Text(TextContentPart {
            text,
            provider_options: None,
        })],
        UserContent::Array(parts) => parts,
    }
}

/// Convert AssistantContent to Vec<AssistantContentPart>.
fn assistant_content_to_parts(content: AssistantContent) -> Vec<AssistantContentPart> {
    match content {
        AssistantContent::String(text) => vec![AssistantContentPart::Text(TextContentPart {
            text,
            provider_options: None,
        })],
        AssistantContent::Array(parts) => parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_system_messages() {
        let mut messages = vec![
            Message::System {
                content: UserContent::String("System prompt".into()),
            },
            Message::User {
                content: UserContent::String("Hello".into()),
            },
            Message::System {
                content: UserContent::String("Another system".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("anthropic");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        let result = transformer.transform();

        assert_eq!(result.system_messages.len(), 2);
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0], Message::User { .. }));
    }

    #[test]
    fn test_flatten_consecutive_user_messages() {
        let mut messages = vec![
            Message::User {
                content: UserContent::String("Hello".into()),
            },
            Message::User {
                content: UserContent::String("World".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("anthropic");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        transformer.transform();

        assert_eq!(messages.len(), 1);
        if let Message::User {
            content: UserContent::Array(parts),
        } = &messages[0]
        {
            assert_eq!(parts.len(), 2);
        } else {
            panic!("Expected User message with Array content");
        }
    }

    #[test]
    fn test_flatten_consecutive_assistant_messages() {
        let mut messages = vec![
            Message::Assistant {
                content: AssistantContent::String("Hi".into()),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::String("there".into()),
                id: Some("id2".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("anthropic");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        transformer.transform();

        assert_eq!(messages.len(), 1);
        if let Message::Assistant {
            content: AssistantContent::Array(parts),
            ..
        } = &messages[0]
        {
            assert_eq!(parts.len(), 2);
        } else {
            panic!("Expected Assistant message with Array content");
        }
    }

    #[test]
    fn test_no_flattening_for_openai() {
        let mut messages = vec![
            Message::User {
                content: UserContent::String("Hello".into()),
            },
            Message::User {
                content: UserContent::String("World".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("openai");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        transformer.transform();

        // Should NOT be flattened for OpenAI
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_no_system_extraction_for_openai() {
        let mut messages = vec![
            Message::System {
                content: UserContent::String("System".into()),
            },
            Message::User {
                content: UserContent::String("Hello".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("openai");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        let result = transformer.transform();

        // System messages should NOT be extracted for OpenAI
        assert!(result.system_messages.is_empty());
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_mixed_message_sequence() {
        let mut messages = vec![
            Message::User {
                content: UserContent::String("1".into()),
            },
            Message::User {
                content: UserContent::String("2".into()),
            },
            Message::Assistant {
                content: AssistantContent::String("A".into()),
                id: None,
            },
            Message::User {
                content: UserContent::String("3".into()),
            },
        ];

        let caps = UniversalCapabilities::for_provider("anthropic");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        transformer.transform();

        // [User1, User2] -> [User], [Assistant] -> [Assistant], [User3] -> [User]
        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0], Message::User { .. }));
        assert!(matches!(messages[1], Message::Assistant { .. }));
        assert!(matches!(messages[2], Message::User { .. }));
    }

    #[test]
    fn test_flatten_tool_messages() {
        use crate::serde_json;
        use crate::universal::{ToolContentPart, ToolResultContentPart};

        let mut messages = vec![
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "1".into(),
                    tool_name: "test".into(),
                    output: serde_json::json!("result1"),
                    provider_options: None,
                })],
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultContentPart {
                    tool_call_id: "2".into(),
                    tool_name: "test".into(),
                    output: serde_json::json!("result2"),
                    provider_options: None,
                })],
            },
        ];

        let caps = UniversalCapabilities::for_provider("anthropic");
        let mut transformer = UniversalTransformer::new(&mut messages, caps);
        transformer.transform();

        assert_eq!(messages.len(), 1);
        if let Message::Tool { content } = &messages[0] {
            assert_eq!(content.len(), 2);
        } else {
            panic!("Expected Tool message");
        }
    }
}
