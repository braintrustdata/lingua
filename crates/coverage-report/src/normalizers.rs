/*!
Semantic-equivalence normalizers for coverage-report diffs.

These rules apply only to Universal types and keep scope explicit and type-safe.
*/

use lingua::serde_json::Value;
use lingua::universal::{
    message::{
        AssistantContent, AssistantContentPart, Message, TextContentPart, UserContent,
        UserContentPart,
    },
    UniversalRequest, UniversalResponse, UniversalStreamChunk,
};

/// Normalize a UniversalRequest for semantic comparison.
///
/// Rule: message content strings are equivalent to a single text-part array.
pub fn normalize_request_for_comparison(req: &UniversalRequest) -> UniversalRequest {
    let mut normalized = req.clone();
    for message in &mut normalized.messages {
        normalize_message_content(message);
    }
    normalized
}

/// Normalize a UniversalResponse for semantic comparison.
///
/// Rule: message content strings are equivalent to a single text-part array.
pub fn normalize_response_for_comparison(resp: &UniversalResponse) -> UniversalResponse {
    let mut normalized = resp.clone();
    for message in &mut normalized.messages {
        normalize_message_content(message);
    }
    normalized
}

/// Normalize a UniversalStreamChunk for semantic comparison.
///
/// Rule: stream deltas with content strings are equivalent to a single text-part array.
pub fn normalize_stream_chunk_for_comparison(chunk: &UniversalStreamChunk) -> UniversalStreamChunk {
    let mut normalized = chunk.clone();
    for choice in &mut normalized.choices {
        if let Some(Value::Object(map)) = choice.delta.as_mut() {
            if let Some(Value::String(text)) = map.get("content").cloned() {
                map.insert("content".to_string(), text_part_value(text));
            }
        }
    }
    normalized
}

fn normalize_message_content(message: &mut Message) {
    match message {
        Message::System { content }
        | Message::Developer { content }
        | Message::User { content } => {
            normalize_user_content(content);
        }
        Message::Assistant { content, .. } => {
            normalize_assistant_content(content);
        }
        Message::Tool { .. } => {}
    }
}

fn normalize_user_content(content: &mut UserContent) {
    if let UserContent::String(text) = content {
        let text = std::mem::take(text);
        *content = UserContent::Array(vec![UserContentPart::Text(text_part(text))]);
    }
}

fn normalize_assistant_content(content: &mut AssistantContent) {
    if let AssistantContent::String(text) = content {
        let text = std::mem::take(text);
        *content = AssistantContent::Array(vec![AssistantContentPart::Text(text_part(text))]);
    }
}

fn text_part(text: String) -> TextContentPart {
    TextContentPart {
        text,
        provider_options: None,
    }
}

fn text_part_value(text: String) -> Value {
    Value::Array(vec![Value::Object(
        [
            ("type".to_string(), Value::String("text".to_string())),
            ("text".to_string(), Value::String(text)),
        ]
        .into_iter()
        .collect(),
    )])
}
