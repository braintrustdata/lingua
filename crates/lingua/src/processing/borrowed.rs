use crate::serde_json::Value;
use crate::universal::{
    AssistantContent, AssistantContentPart, CacheControl, Message, ProviderOptions,
    TextContentPart, UserContent, UserContentPart,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{hash_map::DefaultHasher, HashSet},
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct BorrowText<'a>(pub Cow<'a, str>);

impl<'de: 'a, 'a> Deserialize<'de> for BorrowText<'a> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = BorrowText<'de>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string")
            }
            fn visit_borrowed_str<E: serde::de::Error>(
                self,
                value: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(BorrowText(Cow::Borrowed(value)))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(BorrowText(Cow::Owned(value.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(BorrowText(Cow::Owned(value)))
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageView<'a> {
    Owned(Message),
    Text(TextMessage<'a>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum TextMessage<'a> {
    System {
        content: TextContent<'a>,
    },
    Developer {
        content: TextContent<'a>,
    },
    User {
        content: TextContent<'a>,
    },
    Assistant {
        content: TextContent<'a>,
        id: Option<Cow<'a, str>>,
    },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TextContent<'a> {
    String(Cow<'a, str>),
    Array(Vec<TextPart<'a>>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextPart<'a> {
    Text {
        text: Cow<'a, str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
    Reasoning {
        text: Cow<'a, str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<Cow<'a, str>>,
    },
    #[serde(untagged)]
    Owned(Box<AssistantContentPart>),
}

impl<'a> TextPart<'a> {
    pub fn is_reasoning(&self) -> bool {
        match self {
            Self::Reasoning { .. } => true,
            Self::Owned(part) => matches!(part.as_ref(), AssistantContentPart::Reasoning { .. }),
            _ => false,
        }
    }
    fn into_assistant(self) -> AssistantContentPart {
        match self {
            Self::Owned(part) => *part,
            Self::Text {
                text,
                cache_control,
                provider_options,
            } => AssistantContentPart::Text(TextContentPart {
                text: text.into_owned(),
                encrypted_content: None,
                cache_control,
                provider_options,
            }),
            Self::Reasoning {
                text,
                encrypted_content,
            } => AssistantContentPart::Reasoning {
                text: text.into_owned(),
                encrypted_content: encrypted_content.map(Cow::into_owned),
            },
        }
    }
    fn into_user(self) -> UserContentPart {
        match self {
            Self::Text {
                text,
                cache_control,
                provider_options,
            } => UserContentPart::Text(TextContentPart {
                text: text.into_owned(),
                encrypted_content: None,
                cache_control,
                provider_options,
            }),
            Self::Owned(_) | Self::Reasoning { .. } => {
                unreachable!("reasoning is only constructed for assistant messages")
            }
        }
    }
    fn normalized_text(&self) -> Option<&str> {
        match self {
            Self::Text {
                text,
                cache_control: None,
                ..
            } => Some(text),
            Self::Owned(part) => match part.as_ref() {
                AssistantContentPart::Text(part)
                    if part.encrypted_content.is_none() && part.cache_control.is_none() =>
                {
                    Some(&part.text)
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn hash_content(&self, state: &mut DefaultHasher) {
        match self {
            Self::Owned(part) => super::dedup::hash_assistant_part(part, state),
            Self::Text {
                text,
                cache_control,
                ..
            } => {
                "text".hash(state);
                super::dedup::hash_text_fields(text, None, cache_control, state);
            }
            Self::Reasoning {
                text,
                encrypted_content,
            } => {
                "reasoning".hash(state);
                text.hash(state);
                encrypted_content.hash(state);
            }
        }
    }
}

impl<'a> TextContent<'a> {
    fn into_assistant(self) -> AssistantContent {
        match self {
            Self::String(text) => AssistantContent::String(text.into_owned()),
            Self::Array(parts) => {
                AssistantContent::Array(parts.into_iter().map(TextPart::into_assistant).collect())
            }
        }
    }
    fn into_user(self) -> UserContent {
        match self {
            Self::String(text) => UserContent::String(text.into_owned()),
            Self::Array(parts) => {
                UserContent::Array(parts.into_iter().map(TextPart::into_user).collect())
            }
        }
    }
    fn hash_content(&self, state: &mut DefaultHasher) {
        let normalized = match self {
            Self::String(text) => Some(text.as_ref()),
            Self::Array(parts) if parts.len() == 1 => parts[0].normalized_text(),
            _ => None,
        };
        if let Some(text) = normalized {
            "text".hash(state);
            text.hash(state);
            return;
        }
        if let Self::Array(parts) = self {
            "parts".hash(state);
            parts.len().hash(state);
            for part in parts {
                part.hash_content(state);
            }
        }
    }
}

impl<'a> MessageView<'a> {
    pub(crate) fn into_text_assistant(self) -> Option<TextMessage<'a>> {
        match self {
            Self::Text(message @ TextMessage::Assistant { .. }) => Some(message),
            Self::Owned(Message::Assistant { content, id }) => Some(TextMessage::Assistant {
                content: match content {
                    AssistantContent::String(text) => TextContent::String(Cow::Owned(text)),
                    AssistantContent::Array(parts) => TextContent::Array(
                        parts
                            .into_iter()
                            .map(|part| TextPart::Owned(Box::new(part)))
                            .collect(),
                    ),
                },
                id: id.map(Cow::Owned),
            }),
            _ => None,
        }
    }
    pub(crate) fn is_assistant(&self) -> bool {
        matches!(
            self,
            Self::Text(TextMessage::Assistant { .. }) | Self::Owned(Message::Assistant { .. })
        )
    }
    pub(crate) fn is_reasoning_only(&self) -> bool {
        match self {
            Self::Text(TextMessage::Assistant {
                content: TextContent::Array(parts),
                ..
            }) => !parts.is_empty() && parts.iter().all(TextPart::is_reasoning),
            Self::Owned(Message::Assistant {
                content: AssistantContent::Array(parts),
                ..
            }) => {
                !parts.is_empty()
                    && parts
                        .iter()
                        .all(|part| matches!(part, AssistantContentPart::Reasoning { .. }))
            }
            _ => false,
        }
    }
    pub fn into_owned(self) -> Message {
        match self {
            Self::Owned(message) => message,
            Self::Text(message) => match message {
                TextMessage::System { content } => Message::System {
                    content: content.into_user(),
                },
                TextMessage::Developer { content } => Message::Developer {
                    content: content.into_user(),
                },
                TextMessage::User { content } => Message::User {
                    content: content.into_user(),
                },
                TextMessage::Assistant { content, id } => Message::Assistant {
                    content: content.into_assistant(),
                    id: id.map(Cow::into_owned),
                },
            },
        }
    }
    pub fn is_system(&self) -> bool {
        matches!(
            self,
            Self::Owned(Message::System { .. }) | Self::Text(TextMessage::System { .. })
        )
    }
    pub fn dedup_hash(&self) -> u64 {
        let Self::Text(message) = self else {
            if let Self::Owned(message) = self {
                return super::dedup::message_dedup_hash(message);
            }
            unreachable!()
        };
        let mut state = DefaultHasher::new();
        let (role, content) = match message {
            TextMessage::System { content } => ("system", content),
            TextMessage::Developer { content } => ("developer", content),
            TextMessage::User { content } => ("user", content),
            TextMessage::Assistant { content, .. } => ("assistant", content),
        };
        role.hash(&mut state);
        content.hash_content(&mut state);
        state.finish()
    }
}

pub fn deduplicate_message_views(messages: Vec<MessageView<'_>>) -> Vec<MessageView<'_>> {
    let mut seen = HashSet::new();
    messages
        .into_iter()
        .filter(|message| seen.insert(message.dedup_hash()))
        .collect()
}

pub struct SpanRef<'a> {
    pub input: Option<&'a Value>,
    pub output: Option<&'a Value>,
    pub metadata: Option<&'a Value>,
}

impl<'a> From<&'a super::import::Span> for SpanRef<'a> {
    fn from(span: &'a super::import::Span) -> Self {
        Self {
            input: span.input.as_ref(),
            output: span.output.as_ref(),
            metadata: span.other.get("metadata"),
        }
    }
}

#[cfg(all(test, feature = "openai"))]
mod tests {
    use super::*;
    use crate::processing::import::import_messages_from_span_refs;
    use crate::serde_json::json;

    #[test]
    fn responses_text_and_reasoning_borrow_source_and_match_owned_hashes() {
        let source = json!([
            {"type":"reasoning","id":"reasoning", "encrypted_content":"encrypted 日本語", "summary":[{"type":"summary_text","text":"reasoning café"}]},
            {"type":"message","role":"assistant","id":"message", "content":[{"type":"output_text","text":"line\n🚀","annotations":[], "prompt_cache_breakpoint":{"mode":"explicit"}}]}
        ]);
        let messages = import_messages_from_span_refs([SpanRef {
            input: None,
            output: Some(&source),
            metadata: None,
        }]);
        assert_eq!(messages.len(), 1);
        let MessageView::Text(TextMessage::Assistant {
            content: TextContent::Array(parts),
            ..
        }) = &messages[0]
        else {
            panic!("expected borrowed assistant")
        };
        let TextPart::Reasoning {
            text,
            encrypted_content,
        } = &parts[0]
        else {
            panic!("expected reasoning")
        };
        assert!(matches!(text, Cow::Borrowed(_)));
        assert_eq!(
            text.as_ptr(),
            source[0]["summary"][0]["text"].as_str().unwrap().as_ptr()
        );
        assert_eq!(
            encrypted_content.as_ref().unwrap().as_ptr(),
            source[0]["encrypted_content"].as_str().unwrap().as_ptr()
        );
        let TextPart::Text { text, .. } = &parts[1] else {
            panic!("expected text")
        };
        assert_eq!(
            text.as_ptr(),
            source[1]["content"][0]["text"].as_str().unwrap().as_ptr()
        );
        for message in messages {
            let encoded = crate::serde_json::to_value(&message).unwrap();
            let hash = message.dedup_hash();
            let owned = message.into_owned();
            assert_eq!(encoded, crate::serde_json::to_value(&owned).unwrap());
            assert_eq!(hash, crate::processing::message_dedup_hash(&owned));
        }
    }

    #[test]
    fn text_view_hashes_match_owned_normalization_and_cache_control() {
        for role in ["system", "developer", "user", "assistant"] {
            for content in [
                json!("café"),
                json!([{"type":"input_text", "text":"café"}]),
                json!([{"type":"output_text", "text":"café", "prompt_cache_breakpoint":{"mode":"explicit"}}]),
            ] {
                let source = json!({"type":"message", "role":role,"content":content});
                let messages = import_messages_from_span_refs([SpanRef {
                    input: Some(&source),
                    output: None,
                    metadata: None,
                }]);
                for message in messages {
                    let hash = message.dedup_hash();
                    let encoded = crate::serde_json::to_value(&message).unwrap();
                    let owned = message.into_owned();
                    assert_eq!(hash, crate::processing::message_dedup_hash(&owned));
                    assert_eq!(encoded, crate::serde_json::to_value(&owned).unwrap());
                }
            }
        }
    }

    #[test]
    fn borrowed_canonical_types_validate_fields_outside_borrowed_content() {
        use crate::providers::openai::{borrowed_generated, generated};
        for source in [
            json!({"type":"reasoning", "summary":[{"type":"summary_text","text":"ok"}]}),
            json!({"type":"reasoning", "status":5, "summary":[{"type":"summary_text","text":"ok"}]}),
            json!({"type":"reasoning", "summary":[{"type":"not_a_summary","text":"ok"}]}),
            json!({"type":"message", "role":"assistant", "content":[{"type":"output_text","text":false}]}),
        ] {
            assert_eq!(
                borrowed_generated::InputItem::deserialize(&source).is_ok(),
                generated::InputItem::deserialize(&source).is_ok()
            );
        }
    }
    #[test]
    fn borrowed_spans_skip_empty_values_before_provider_parsing() {
        for value in [Value::Null, json!([]), json!({})] {
            assert!(import_messages_from_span_refs([SpanRef {
                input: Some(&value),
                output: Some(&value),
                metadata: None
            }])
            .is_empty());
        }
    }
    #[test]
    fn borrowing_preserves_mixed_tool_conversion_and_late_failures() {
        use crate::providers::openai::convert::{
            try_parse_openai_borrowed_for_import, try_parse_openai_for_import,
        };
        let reasoning = json!({"type":"reasoning","id":"reason", "encrypted_content":"secret", "summary":[{"type":"summary_text", "text":"reasoning"}]});
        let tool = json!({"type":"function_call", "id":"tool", "call_id":"call", "name":"lookup", "arguments":"{\"query\":\"日本語\"}"});
        let text = json!({"type":"message", "role":"assistant", "id":"text", "content":[{"type":"output_text", "text":"result", "annotations":[]}]});
        for source in [
            json!([reasoning, tool, text]),
            json!([tool, reasoning, text]),
            json!([reasoning, text, tool]),
            json!([reasoning,tool,{"type":"message","role":"assistant","content":[{"type":"output_text","text":false}]}]),
            json!([{"type":"tool_search_call","id":"search", "call_id":null, "status":"completed", "execution":"server", "arguments":{}}, {"type":"tool_search_output", "id":"search_output", "call_id":null, "status":"completed", "execution":"server", "tools":[{"type":"function","name":"lookup", "description":"Lookup"}]}, reasoning, tool]),
        ] {
            let expected = try_parse_openai_for_import(&source);
            let actual = try_parse_openai_borrowed_for_import(&source);
            if source.as_array().unwrap().len() == 4 {
                assert!(expected.is_some());
            }
            assert_eq!(
                crate::serde_json::to_value(&actual).unwrap(),
                crate::serde_json::to_value(&expected).unwrap()
            );
            if let Some(messages) = actual {
                for message in messages {
                    let hash = message.dedup_hash();
                    let owned = message.into_owned();
                    assert_eq!(hash, crate::processing::message_dedup_hash(&owned));
                }
            }
        }
        let source = json!([reasoning, tool]);
        let messages = try_parse_openai_borrowed_for_import(&source).unwrap();
        let MessageView::Text(TextMessage::Assistant {
            content: TextContent::Array(parts),
            ..
        }) = &messages[0]
        else {
            panic!("expected mixed borrowed assistant")
        };
        assert!(matches!(
            &parts[0],
            TextPart::Reasoning {
                text: Cow::Borrowed(_),
                ..
            }
        ));
        assert!(matches!(
            &parts[1],
            TextPart::Owned(part) if matches!(part.as_ref(), AssistantContentPart::ToolCall { .. })
        ));
    }
}
