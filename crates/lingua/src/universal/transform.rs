/*!
Lingua → Lingua transformations that run before converting to a provider format.

This module provides universal transformations that are common across multiple
providers, mirroring the proxy's behavior:

- **Message flattening**: Merge consecutive messages of the same role
  (needed for Anthropic, Google, Bedrock)
- **System message extraction**: Extract system messages to a separate parameter
  (needed for Anthropic, Google, Bedrock)

# Example

```
use lingua::universal::{Message, UserContent, extract_system_messages, flatten_consecutive_messages};

let mut messages = vec![
    Message::System { content: UserContent::text("You are helpful") },
    Message::User { content: UserContent::text("Hello") },
    Message::User { content: UserContent::text("World") },
];

// Extract system messages (for providers that need them separate)
let system = extract_system_messages(&mut messages);
assert_eq!(system.len(), 1);

// Flatten consecutive messages of the same role
flatten_consecutive_messages(&mut messages);
assert_eq!(messages.len(), 1);
```
*/

use crate::universal::{AssistantContent, Message, ToolContent, UserContent};

/// Extract system messages from the message list.
///
/// Removes all `Message::System` variants and returns their content.
/// This is needed for providers like Anthropic, Google, and Bedrock
/// where system messages are passed as a separate parameter.
///
/// # Example
///
/// ```
/// use lingua::universal::{Message, UserContent, extract_system_messages};
///
/// let mut messages = vec![
///     Message::System { content: UserContent::text("System prompt") },
///     Message::User { content: UserContent::text("Hello") },
/// ];
///
/// let system = extract_system_messages(&mut messages);
/// assert_eq!(system.len(), 1);
/// assert_eq!(messages.len(), 1); // Only user message remains
/// ```
pub fn extract_system_messages(messages: &mut Vec<Message>) -> Vec<UserContent> {
    let mut system_contents = Vec::new();
    messages.retain(|msg| {
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
/// This is needed for providers like Anthropic, Google, and Bedrock
/// that don't allow consecutive messages of the same role.
///
/// For example:
/// - User("Hello") + User("World") → User(["Hello", "World"])
/// - Assistant("Hi") + Assistant("there") → Assistant(["Hi", "there"])
///
/// # Example
///
/// ```
/// use lingua::universal::{Message, UserContent, flatten_consecutive_messages};
///
/// let mut messages = vec![
///     Message::User { content: UserContent::text("Hello") },
///     Message::User { content: UserContent::text("World") },
/// ];
///
/// flatten_consecutive_messages(&mut messages);
/// assert_eq!(messages.len(), 1); // Merged into single user message
/// ```
pub fn flatten_consecutive_messages(messages: &mut Vec<Message>) {
    if messages.is_empty() {
        return;
    }

    let mut result: Vec<Message> = Vec::with_capacity(messages.len());

    for msg in messages.drain(..) {
        if let Some(last) = result.last_mut() {
            if can_merge(last, &msg) {
                merge_messages(last, msg);
                continue;
            }
        }
        result.push(msg);
    }

    *messages = result;
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
    let old_a = std::mem::replace(a, UserContent::new(vec![]));
    let mut merged = old_a.into_parts();
    merged.extend(b.into_parts());
    *a = UserContent::new(merged);
}

/// Merge two AssistantContent values.
fn merge_assistant_content(a: &mut AssistantContent, b: AssistantContent) {
    // Replace `a` with a temporary empty array, get the old value
    let old_a = std::mem::replace(a, AssistantContent::new(vec![]));
    let mut merged = old_a.into_parts();
    merged.extend(b.into_parts());
    *a = AssistantContent::new(merged);
}

/// Merge two ToolContent values (Vec<ToolContentPart>).
fn merge_tool_content(a: &mut ToolContent, b: ToolContent) {
    a.extend(b);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_system_messages() {
        let mut messages = vec![
            Message::System {
                content: UserContent::text("System prompt"),
            },
            Message::User {
                content: UserContent::text("Hello"),
            },
            Message::System {
                content: UserContent::text("Another system"),
            },
        ];

        let system = extract_system_messages(&mut messages);

        assert_eq!(system.len(), 2);
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0], Message::User { .. }));
    }

    #[test]
    fn test_flatten_consecutive_user_messages() {
        let mut messages = vec![
            Message::User {
                content: UserContent::text("Hello"),
            },
            Message::User {
                content: UserContent::text("World"),
            },
        ];

        flatten_consecutive_messages(&mut messages);

        assert_eq!(messages.len(), 1);
        if let Message::User { content } = &messages[0] {
            assert_eq!(content.parts().len(), 2);
        } else {
            panic!("Expected User message with Array content");
        }
    }

    #[test]
    fn test_flatten_consecutive_assistant_messages() {
        let mut messages = vec![
            Message::Assistant {
                content: AssistantContent::text("Hi"),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::text("there"),
                id: Some("id2".into()),
            },
        ];

        flatten_consecutive_messages(&mut messages);

        assert_eq!(messages.len(), 1);
        if let Message::Assistant {
            content,
            ..
        } = &messages[0]
        {
            assert_eq!(content.parts().len(), 2);
        } else {
            panic!("Expected Assistant message with Array content");
        }
    }

    #[test]
    fn test_no_modification_when_not_called() {
        let messages = [
            Message::User {
                content: UserContent::text("Hello"),
            },
            Message::User {
                content: UserContent::text("World"),
            },
        ];

        // If we don't call flatten_consecutive_messages, messages stay as-is
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_system_messages_preserved_when_not_extracted() {
        let messages = [
            Message::System {
                content: UserContent::text("System"),
            },
            Message::User {
                content: UserContent::text("Hello"),
            },
        ];

        // If we don't call extract_system_messages, system messages stay in place
        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0], Message::System { .. }));
    }

    #[test]
    fn test_mixed_message_sequence() {
        let mut messages = vec![
            Message::User {
                content: UserContent::text("1"),
            },
            Message::User {
                content: UserContent::text("2"),
            },
            Message::Assistant {
                content: AssistantContent::text("A"),
                id: None,
            },
            Message::User {
                content: UserContent::text("3"),
            },
        ];

        flatten_consecutive_messages(&mut messages);

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

        flatten_consecutive_messages(&mut messages);

        assert_eq!(messages.len(), 1);
        if let Message::Tool { content } = &messages[0] {
            assert_eq!(content.len(), 2);
        } else {
            panic!("Expected Tool message");
        }
    }

    #[test]
    fn test_combined_extract_and_flatten() {
        // Test the typical flow for providers like Anthropic
        let mut messages = vec![
            Message::System {
                content: UserContent::text("You are helpful"),
            },
            Message::User {
                content: UserContent::text("Hello"),
            },
            Message::User {
                content: UserContent::text("World"),
            },
        ];

        // Step 1: Extract system messages
        let system = extract_system_messages(&mut messages);
        assert_eq!(system.len(), 1);

        // Step 2: Flatten consecutive messages
        flatten_consecutive_messages(&mut messages);
        assert_eq!(messages.len(), 1);
        assert!(matches!(messages[0], Message::User { .. }));
    }
}
