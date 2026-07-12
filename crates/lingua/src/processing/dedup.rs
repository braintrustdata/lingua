use crate::universal::tools::ToolAvailability;
use crate::universal::{
    AssistantContent, AssistantContentPart, BuiltinToolProvider, CacheControl, CacheControlTtl,
    CacheControlType, Message, TextContentPart, ToolContent, UniversalTool, UniversalToolType,
    UserContent, UserContentPart,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::serde_json;

/// Stable deduplication key for a message based on its role and normalized content.
///
/// Two messages that [`deduplicate_messages`] would treat as duplicates share the same
/// key. Exposed so callers can implement source-aware deduplication (for example,
/// preferring an LLM-derived copy of a message over a wrapper-span copy) while staying
/// consistent with [`deduplicate_messages`].
pub fn message_dedup_hash(msg: &Message) -> u64 {
    hash_message(msg)
}

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
        Message::Developer { content } => {
            "developer".hash(&mut hasher);
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
        Message::AdditionalTools { tools, id } => {
            "additional_tools".hash(&mut hasher);
            id.hash(&mut hasher);
            serde_json::to_string(tools)
                .unwrap_or_default()
                .hash(&mut hasher);
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
                if let Some(UserContentPart::Text(text_part)) = parts.first() {
                    if can_normalize_text_part_to_string(text_part) {
                        "text".hash(hasher);
                        text_part.text.hash(hasher);
                        return;
                    }
                }
            }

            // For multi-part or non-text content, hash each part
            "parts".hash(hasher);
            parts.len().hash(hasher);
            for part in parts {
                match part {
                    UserContentPart::Text(text_part) => {
                        "text".hash(hasher);
                        hash_text_content_part(text_part, hasher);
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
                if let Some(AssistantContentPart::Text(text_part)) = parts.first() {
                    if can_normalize_text_part_to_string(text_part) {
                        "text".hash(hasher);
                        text_part.text.hash(hasher);
                        return;
                    }
                }
            }

            // For multi-part content, hash each part
            "parts".hash(hasher);
            parts.len().hash(hasher);
            for part in parts {
                match part {
                    AssistantContentPart::Text(text_part) => {
                        "text".hash(hasher);
                        hash_text_content_part(text_part, hasher);
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
                        status,
                        caller,
                        ..
                    } => {
                        "tool_call".hash(hasher);
                        tool_call_id.hash(hasher);
                        tool_name.hash(hasher);
                        status.hash(hasher);
                        caller.hash(hasher);
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
                            crate::universal::ToolCallArguments::Custom(s) => {
                                "custom".hash(hasher);
                                s.hash(hasher);
                            }
                        }
                    }
                    AssistantContentPart::ToolDiscoveryCall {
                        tool_call_id,
                        discovery_tool_name,
                        query,
                        arguments,
                        status,
                        execution,
                        ..
                    } => {
                        "tool_discovery_call".hash(hasher);
                        tool_call_id.hash(hasher);
                        discovery_tool_name.hash(hasher);
                        query.hash(hasher);
                        arguments.hash(hasher);
                        status.hash(hasher);
                        execution.hash(hasher);
                    }
                    AssistantContentPart::ToolResult {
                        tool_call_id,
                        tool_name,
                        output,
                        caller,
                        ..
                    } => {
                        "tool_result".hash(hasher);
                        tool_call_id.hash(hasher);
                        tool_name.hash(hasher);
                        output.hash(hasher);
                        caller.hash(hasher);
                    }
                    AssistantContentPart::Program {
                        call_id,
                        code,
                        fingerprint,
                        id,
                    } => {
                        "program".hash(hasher);
                        call_id.hash(hasher);
                        code.hash(hasher);
                        fingerprint.hash(hasher);
                        id.hash(hasher);
                    }
                    AssistantContentPart::ProgramOutput {
                        call_id,
                        result,
                        status,
                        id,
                    } => {
                        "program_output".hash(hasher);
                        call_id.hash(hasher);
                        result.hash(hasher);
                        status.hash(hasher);
                        id.hash(hasher);
                    }
                }
            }
        }
    }
}

fn can_normalize_text_part_to_string(part: &TextContentPart) -> bool {
    part.encrypted_content.is_none() && part.cache_control.is_none()
}

fn hash_text_content_part(part: &TextContentPart, hasher: &mut DefaultHasher) {
    part.text.hash(hasher);
    part.encrypted_content.hash(hasher);
    hash_cache_control(&part.cache_control, hasher);
}

fn hash_cache_control(cache_control: &Option<CacheControl>, hasher: &mut DefaultHasher) {
    match cache_control {
        Some(cache_control) => {
            "some".hash(hasher);
            match cache_control.cache_control_type {
                CacheControlType::Ephemeral => "ephemeral".hash(hasher),
            }
            match cache_control.ttl {
                Some(CacheControlTtl::The1H) => "1h".hash(hasher),
                Some(CacheControlTtl::The5M) => "5m".hash(hasher),
                None => "none".hash(hasher),
            }
        }
        None => "none".hash(hasher),
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
                result.caller.hash(hasher);
            }
            crate::universal::ToolContentPart::ToolDiscoveryResult(result) => {
                "tool_discovery_result".hash(hasher);
                result.tool_call_id.hash(hasher);
                result.discovery_tool_name.hash(hasher);
                result.status.hash(hasher);
                result.execution.hash(hasher);
                for item in &result.tools {
                    item.tool_name.hash(hasher);
                    hash_optional_universal_tool(&item.tool, hasher);
                }
            }
        }
    }
}

fn hash_optional_universal_tool(tool: &Option<UniversalTool>, hasher: &mut DefaultHasher) {
    match tool {
        Some(tool) => {
            "some".hash(hasher);
            hash_universal_tool(tool, hasher);
        }
        None => "none".hash(hasher),
    }
}

fn hash_universal_tool(tool: &UniversalTool, hasher: &mut DefaultHasher) {
    tool.name.hash(hasher);
    tool.description.hash(hasher);
    tool.parameters.hash(hasher);
    tool.strict.hash(hasher);
    hash_tool_availability(tool.availability, hasher);
    tool.allowed_callers.hash(hasher);
    tool.output_schema.hash(hasher);

    match &tool.tool_type {
        UniversalToolType::Function => "function".hash(hasher),
        UniversalToolType::Custom { format } => {
            "custom".hash(hasher);
            format.hash(hasher);
        }
        UniversalToolType::Builtin {
            provider,
            builtin_type,
            config,
        } => {
            "builtin".hash(hasher);
            hash_builtin_tool_provider(*provider, hasher);
            builtin_type.hash(hasher);
            config.hash(hasher);
        }
    }
}

fn hash_tool_availability(availability: ToolAvailability, hasher: &mut DefaultHasher) {
    match availability {
        ToolAvailability::Immediate => "immediate".hash(hasher),
        ToolAvailability::Deferred => "deferred".hash(hasher),
    }
}

fn hash_builtin_tool_provider(provider: BuiltinToolProvider, hasher: &mut DefaultHasher) {
    match provider {
        BuiltinToolProvider::Anthropic => "anthropic".hash(hasher),
        BuiltinToolProvider::Responses => "responses".hash(hasher),
        BuiltinToolProvider::Google => "google".hash(hasher),
        BuiltinToolProvider::Converse => "converse".hash(hasher),
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
        AssistantContent, CacheControl, CacheControlTtl, CacheControlType, Message,
        TextContentPart, UserContent, UserContentPart,
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
                    encrypted_content: None,
                    cache_control: None,
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
    fn test_dedup_preserves_user_cache_control_text_part() {
        let messages = vec![
            Message::User {
                content: UserContent::String("foo".to_string()),
            },
            Message::User {
                content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                    text: "foo".to_string(),
                    encrypted_content: None,
                    cache_control: Some(CacheControl {
                        cache_control_type: CacheControlType::Ephemeral,
                        ttl: Some(CacheControlTtl::The1H),
                    }),
                    provider_options: None,
                })]),
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
        assert!(matches!(
            &result[1],
            Message::User {
                content: UserContent::Array(parts)
            } if matches!(
                parts.first(),
                Some(UserContentPart::Text(TextContentPart {
                    cache_control: Some(_),
                    ..
                }))
            )
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
                        encrypted_content: None,
                        cache_control: None,
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
    fn test_dedup_preserves_assistant_cache_control_text_part() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::String("response".to_string()),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::Text(
                    TextContentPart {
                        text: "response".to_string(),
                        encrypted_content: None,
                        cache_control: Some(CacheControl {
                            cache_control_type: CacheControlType::Ephemeral,
                            ttl: Some(CacheControlTtl::The1H),
                        }),
                        provider_options: None,
                    },
                )]),
                id: None,
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
        assert!(matches!(
            &result[1],
            Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            } if matches!(
                parts.first(),
                Some(AssistantContentPart::Text(TextContentPart {
                    cache_control: Some(_),
                    ..
                }))
            )
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
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }),
                    UserContentPart::Text(TextContentPart {
                        text: "world".to_string(),
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }),
                ]),
            },
            Message::User {
                content: UserContent::Array(vec![
                    UserContentPart::Text(TextContentPart {
                        text: "hello".to_string(),
                        encrypted_content: None,
                        cache_control: None,
                        provider_options: None,
                    }),
                    UserContentPart::Text(TextContentPart {
                        text: "world".to_string(),
                        encrypted_content: None,
                        cache_control: None,
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
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: None,
                })]),
            },
            Message::User {
                content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                    text: "test".to_string(),
                    encrypted_content: None,
                    cache_control: None,
                    provider_options: Some(crate::universal::ProviderOptions {
                        options: serde_json::Map::new(),
                    }),
                })]),
            },
        ];

        let result = deduplicate_messages(messages);
        // These should be considered duplicates since we only compare content text
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_preserves_discovery_results_with_different_tool_definitions() {
        let first_tool = UniversalTool::function(
            "search_code",
            Some("Search code.".to_string()),
            Some(crate::serde_json::json!({
                "type": "object",
                "properties": {}
            })),
            Some(true),
        );
        let second_tool = UniversalTool::function(
            "search_code",
            Some("Search code.".to_string()),
            Some(crate::serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            })),
            Some(true),
        );

        let messages = vec![
            Message::Tool {
                content: vec![crate::universal::ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![crate::universal::ToolDiscoveryResultItem {
                            tool_name: "search_code".to_string(),
                            tool: Some(first_tool),
                            provider_options: None,
                        }],
                        status: Some("completed".to_string()),
                        execution: Some("client".to_string()),
                        provider_options: None,
                    },
                )],
            },
            Message::Tool {
                content: vec![crate::universal::ToolContentPart::ToolDiscoveryResult(
                    crate::universal::ToolDiscoveryResultContentPart {
                        tool_call_id: "call_tool_search_123".to_string(),
                        discovery_tool_name: "tool_search".to_string(),
                        tools: vec![crate::universal::ToolDiscoveryResultItem {
                            tool_name: "search_code".to_string(),
                            tool: Some(second_tool),
                            provider_options: None,
                        }],
                        status: Some("completed".to_string()),
                        execution: Some("client".to_string()),
                        provider_options: None,
                    },
                )],
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_preserves_tool_calls_with_different_callers() {
        let caller_a = crate::universal::ToolCaller {
            caller_type: crate::universal::ToolCallerType::Program,
            caller_id: "call_prog_a".to_string(),
        };
        let caller_b = crate::universal::ToolCaller {
            caller_type: crate::universal::ToolCallerType::Program,
            caller_id: "call_prog_b".to_string(),
        };

        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                    tool_call_id: "call_inventory_123".to_string(),
                    tool_name: "get_inventory".to_string(),
                    arguments: crate::universal::ToolCallArguments::from(
                        "{\"sku\":\"sku_123\"}".to_string(),
                    ),
                    caller: Some(caller_a),
                    encrypted_content: None,
                    provider_options: None,
                    status: None,
                    provider_executed: None,
                }]),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                    tool_call_id: "call_inventory_123".to_string(),
                    tool_name: "get_inventory".to_string(),
                    arguments: crate::universal::ToolCallArguments::from(
                        "{\"sku\":\"sku_123\"}".to_string(),
                    ),
                    caller: Some(caller_b),
                    encrypted_content: None,
                    provider_options: None,
                    status: None,
                    provider_executed: None,
                }]),
                id: None,
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_preserves_tool_calls_with_different_statuses() {
        let messages = vec![
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                    tool_call_id: "call_inventory_123".to_string(),
                    tool_name: "get_inventory".to_string(),
                    arguments: crate::universal::ToolCallArguments::from(
                        "{\"sku\":\"sku_123\"}".to_string(),
                    ),
                    status: Some("in_progress".to_string()),
                    caller: None,
                    encrypted_content: None,
                    provider_options: None,
                    provider_executed: None,
                }]),
                id: None,
            },
            Message::Assistant {
                content: AssistantContent::Array(vec![AssistantContentPart::ToolCall {
                    tool_call_id: "call_inventory_123".to_string(),
                    tool_name: "get_inventory".to_string(),
                    arguments: crate::universal::ToolCallArguments::from(
                        "{\"sku\":\"sku_123\"}".to_string(),
                    ),
                    status: Some("completed".to_string()),
                    caller: None,
                    encrypted_content: None,
                    provider_options: None,
                    provider_executed: None,
                }]),
                id: None,
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_preserves_tool_results_with_different_callers() {
        let caller_a = crate::universal::ToolCaller {
            caller_type: crate::universal::ToolCallerType::Program,
            caller_id: "call_prog_a".to_string(),
        };
        let caller_b = crate::universal::ToolCaller {
            caller_type: crate::universal::ToolCallerType::Program,
            caller_id: "call_prog_b".to_string(),
        };

        let messages = vec![
            Message::Tool {
                content: vec![crate::universal::ToolContentPart::ToolResult(
                    crate::universal::ToolResultContentPart {
                        tool_call_id: "call_inventory_123".to_string(),
                        tool_name: "get_inventory".to_string(),
                        output: crate::serde_json::json!({
                            "sku": "sku_123",
                            "available_units": 42
                        }),
                        custom_tool_call: None,
                        caller: Some(caller_a),
                        provider_options: None,
                    },
                )],
            },
            Message::Tool {
                content: vec![crate::universal::ToolContentPart::ToolResult(
                    crate::universal::ToolResultContentPart {
                        tool_call_id: "call_inventory_123".to_string(),
                        tool_name: "get_inventory".to_string(),
                        output: crate::serde_json::json!({
                            "sku": "sku_123",
                            "available_units": 42
                        }),
                        custom_tool_call: None,
                        caller: Some(caller_b),
                        provider_options: None,
                    },
                )],
            },
        ];

        let result = deduplicate_messages(messages);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_original_message_unmodified() {
        // Test that the original message structure is preserved exactly
        let original = Message::User {
            content: UserContent::Array(vec![UserContentPart::Text(TextContentPart {
                text: "preserve me".to_string(),
                encrypted_content: None,
                cache_control: None,
                provider_options: Some(crate::universal::ProviderOptions {
                    options: {
                        let mut map = serde_json::Map::new();
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
                ..
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
