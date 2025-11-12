use crate::universal::{
    AssistantContent, AssistantContentPart, Message, TextContentPart, ToolContent, UserContent,
    UserContentPart,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Computes a hash for a message based on its role and content.
/// This is used for deduplication - messages with the same hash are considered duplicates.
fn hash_message(msg: &Message) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash the role and content
    match msg {
        Message::System { content } => {
            "system".hash(&mut hasher);
            hash_user_content(content, &mut hasher);
        }
        Message::User { content } => {
            "user".hash(&mut hasher);
            hash_user_content(content, &mut hasher);
        }
        Message::Assistant { content, .. } => {
            "assistant".hash(&mut hasher);
            hash_assistant_content(content, &mut hasher);
        }
        Message::Tool { content } => {
            "tool".hash(&mut hasher);
            hash_tool_content(content, &mut hasher);
        }
    }

    hasher.finish()
}

fn hash_user_content(content: &UserContent, hasher: &mut DefaultHasher) {
    match content {
        UserContent::String(s) => {
            "text".hash(hasher);
            s.hash(hasher);
        }
        UserContent::Array(parts) => {
            // Normalize array with single text part to match string representation
            if parts.len() == 1 {
                if let Some(UserContentPart::Text(TextContentPart { text, .. })) = parts.first() {
                    "text".hash(hasher);
                    text.hash(hasher);
                    return;
                }
            }

            // For multi-part or non-text content, hash each part
            "parts".hash(hasher);
            parts.len().hash(hasher);
            for part in parts {
                match part {
                    UserContentPart::Text(TextContentPart { text, .. }) => {
                        "text".hash(hasher);
                        text.hash(hasher);
                    }
                    UserContentPart::Image {
                        image, media_type, ..
                    } => {
                        "image".hash(hasher);
                        image.hash(hasher);
                        media_type.hash(hasher);
                    }
                    UserContentPart::File {
                        data,
                        filename,
                        media_type,
                        ..
                    } => {
                        "file".hash(hasher);
                        data.hash(hasher);
                        filename.hash(hasher);
                        media_type.hash(hasher);
                    }
                }
            }
        }
    }
}

fn hash_assistant_content(content: &AssistantContent, hasher: &mut DefaultHasher) {
    match content {
        AssistantContent::String(s) => {
            "text".hash(hasher);
            s.hash(hasher);
        }
        AssistantContent::Array(parts) => {
            // Normalize array with single text part to match string representation
            if parts.len() == 1 {
                if let Some(AssistantContentPart::Text(TextContentPart { text, .. })) =
                    parts.first()
                {
                    "text".hash(hasher);
                    text.hash(hasher);
                    return;
                }
            }

            // For multi-part content, hash each part
            "parts".hash(hasher);
            parts.len().hash(hasher);
            for part in parts {
                match part {
                    AssistantContentPart::Text(TextContentPart { text, .. }) => {
                        "text".hash(hasher);
                        text.hash(hasher);
                    }
                    AssistantContentPart::File {
                        data,
                        filename,
                        media_type,
                        ..
                    } => {
                        "file".hash(hasher);
                        data.hash(hasher);
                        filename.hash(hasher);
                        media_type.hash(hasher);
                    }
                    AssistantContentPart::Reasoning {
                        text,
                        encrypted_content,
                    } => {
                        "reasoning".hash(hasher);
                        text.hash(hasher);
                        encrypted_content.hash(hasher);
                    }
                    AssistantContentPart::ToolCall {
                        tool_call_id,
                        tool_name,
                        arguments,
                        ..
                    } => {
                        "tool_call".hash(hasher);
                        tool_call_id.hash(hasher);
                        tool_name.hash(hasher);
                        // ToolCallArguments doesn't derive Hash, so handle each variant
                        match arguments {
                            crate::universal::ToolCallArguments::Valid(map) => {
                                "valid".hash(hasher);
                                map.hash(hasher);
                            }
                            crate::universal::ToolCallArguments::Invalid(s) => {
                                "invalid".hash(hasher);
                                s.hash(hasher);
                            }
                        }
                    }
                    AssistantContentPart::ToolResult {
                        tool_call_id,
                        tool_name,
                        output,
                        ..
                    } => {
                        "tool_result".hash(hasher);
                        tool_call_id.hash(hasher);
                        tool_name.hash(hasher);
                        output.hash(hasher);
                    }
                }
            }
        }
    }
}

fn hash_tool_content(content: &ToolContent, hasher: &mut DefaultHasher) {
    content.len().hash(hasher);
    for part in content {
        match part {
            crate::universal::ToolContentPart::ToolResult(result) => {
                "tool_result".hash(hasher);
                result.tool_call_id.hash(hasher);
                result.tool_name.hash(hasher);
                result.output.hash(hasher);
            }
        }
    }
}

/// Deduplicates messages based on role and content.
///
/// Two messages are considered duplicates if:
/// - They have the same role
/// - Their content hashes to the same value
///
/// This handles equivalence between string and array content representations:
/// - `{"role": "user", "content": "foo"}` equals `{"role": "user", "content": [{"type": "text", "text": "foo"}]}`
///
/// The function preserves the order of messages and keeps the first occurrence of each unique message.
/// **Important**: The original messages are returned unmodified - hashing is only used for deduplication.
pub fn deduplicate_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut seen_hashes = HashSet::new();
    let mut result = Vec::with_capacity(messages.len());

    for msg in messages {
        let hash = hash_message(&msg);
        if seen_hashes.insert(hash) {
            result.push(msg);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::{
        AssistantContent, Message, TextContentPart, UserContent, UserContentPart,
    };

    #[test]
    fn test_dedup_exact_duplicates() {
        let messages = vec![
            Message::User {
                content: UserContent::String("hello".to_string()),
            },
            Message::User {
                content: UserContent::String("hello".to_string()),
            },
            Message::User {
                content: UserContent::String("world".to_string()),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_string_vs_array_text() {
        let messages = vec![
            Message::User {
                content: UserContent::String("foo".to_string()),
            },
            Message::User {
                content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                    text: "foo".to_string(),
                    provider_options: None,
                })]),
            },
            Message::User {
                content: UserContent::String("bar".to_string()),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2); // "foo" appears twice but should be deduped

        // Check that first "foo" was kept (as String, not Array)
        if let Message::User {
            content: UserContent::String(s),
        } = &result[0]
        {
            assert_eq!(s, "foo");
        } else {
            panic!("Expected first message to be User with String content");
        }

        // Verify the original message format was preserved
        assert!(matches!(
            result[0],
            Message::User {
                content: UserContent::String(_)
            }
        ));
    }

    #[test]
    fn test_dedup_assistant_string_vs_array() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::String("response".to_string()),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::Text(
                    TextContentPart {
                        text: "response".to_string(),
                        provider_options: None,
                    },
                )]),
                id: None,
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 1);

        // Verify original format preserved
        assert!(matches!(
            result[0],
            Message::Assistant {
                content: AssistantContent::String(_),
                ..
            }
        ));
    }

    #[test]
    fn test_dedup_preserves_order() {
        let messages = vec![
            Message::User {
                content: UserContent::String("first".to_string()),
            },
            Message::Assistant {
                content: AssistantContent::String("second".to_string()),
                id: None,
            },
            Message::User {
                content: UserContent::String("third".to_string()),
            },
            Message::User {
                content: UserContent::String("first".to_string()),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 3);

        // Verify order
        if let Message::User {
            content: UserContent::String(s),
        } = &result[0]
        {
            assert_eq!(s, "first");
        }
        if let Message::Assistant {
            content: AssistantContent::String(s),
            ..
        } = &result[1]
        {
            assert_eq!(s, "second");
        }
        if let Message::User {
            content: UserContent::String(s),
        } = &result[2]
        {
            assert_eq!(s, "third");
        }
    }

    #[test]
    fn test_dedup_different_roles_not_deduped() {
        let messages = vec![
            Message::User {
                content: UserContent::String("same content".to_string()),
            },
            Message::Assistant {
                content: AssistantContent::String("same content".to_string()),
                id: None,
            },
            Message::System {
                content: UserContent::String("same content".to_string()),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 3); // All different roles, no deduplication
    }

    #[test]
    fn test_dedup_empty_messages() {
        let messages: Vec<Message> = vec![];
        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_dedup_single_message() {
        let messages = vec![Message::User {
            content: UserContent::String("only one".to_string()),
        }];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_all_duplicates() {
        let messages = vec![
            Message::User {
                content: UserContent::String("same".to_string()),
            },
            Message::User {
                content: UserContent::String("same".to_string()),
            },
            Message::User {
                content: UserContent::String("same".to_string()),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_multipart_content() {
        let messages = vec![
            Message::User {
                content: UserContent::Array(vec![
                    UserContentPart::Text(TextContentPart {
                        text: "hello".to_string(),
                        provider_options: None,
                    }),
                    UserContentPart::Text(TextContentPart {
                        text: "world".to_string(),
                        provider_options: None,
                    }),
                ]),
            },
            Message::User {
                content: UserContent::Array(vec![
                    UserContentPart::Text(TextContentPart {
                        text: "hello".to_string(),
                        provider_options: None,
                    }),
                    UserContentPart::Text(TextContentPart {
                        text: "world".to_string(),
                        provider_options: None,
                    }),
                ]),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_with_provider_options_ignored() {
        let messages = vec![
            Message::User {
                content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                    text: "test".to_string(),
                    provider_options: None,
                })]),
            },
            Message::User {
                content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                    text: "test".to_string(),
                    provider_options: Some(crate::universal::ProviderOptions {
                        options: crate::serde_json::Map::new(),
                    }),
                })]),
            },
        ];

        let result = deduplicate_messages(messages);
        // These should be considered duplicates since we only compare content text
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_original_message_unmodified() {
        // Test that the original message structure is preserved exactly
        let original = Message::User {
            content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                text: "preserve me".to_string(),
                provider_options: Some(crate::universal::ProviderOptions {
                    options: {
                        let mut map = crate::serde_json::Map::new();
                        map.insert("custom".to_string(), crate::serde_json::json!("value"));
                        map
                    },
                }),
            })]),
        };

        let messages = vec![original.clone()];
        let result = deduplicate_messages(messages);

        assert_eq!(result.len(), 1);

        // Verify it's still an Array, not converted to String
        if let Message::User {
            content: UserContent::Array(parts),
        } = &result[0]
        {
            assert_eq!(parts.len(), 1);
            if let UserContentPart::Text(TextContentPart {
                text,
                provider_options,
            }) = &parts[0]
            {
                assert_eq!(text, "preserve me");
                assert!(provider_options.is_some());
            } else {
                panic!("Expected Text part");
            }
        } else {
            panic!("Expected Array content to be preserved");
        }
    }
}
