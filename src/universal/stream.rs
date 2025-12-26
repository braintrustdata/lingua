/*!
Universal streaming types for cross-provider stream transformation.

This module provides a canonical representation of LLM streaming chunks that can be
converted to/from any provider format. The format follows OpenAI's streaming chunk
structure as the canonical representation.
*/

use crate::serde_json::{self, Value};
use crate::universal::response::UniversalUsage;
use serde::{Deserialize, Serialize};

/// A single choice in a streaming chunk.
///
/// Mirrors OpenAI's StreamChoice structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalStreamChoice {
    /// Index of this choice in the choices array
    pub index: u32,

    /// Delta content for this chunk (role, content, tool_calls, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<Value>,

    /// Reason why generation stopped (only present on final chunk)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A normalized streaming chunk following OpenAI's format.
///
/// This is the universal representation for streaming events from all providers.
/// Provider-specific formats are normalized to this structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalStreamChunk {
    /// Unique identifier for this completion
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Model that generated this chunk
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Array of choices (usually single element for streaming)
    #[serde(default)]
    pub choices: Vec<UniversalStreamChoice>,

    /// Unix timestamp when chunk was created
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Token usage (usually only on final chunk)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UniversalUsage>,

    /// Internal flag for keep-alive events (not serialized)
    #[serde(skip)]
    keep_alive: bool,
}

impl UniversalStreamChunk {
    /// Create a new streaming chunk with the given fields.
    pub fn new(
        id: Option<String>,
        model: Option<String>,
        choices: Vec<UniversalStreamChoice>,
        created: Option<u64>,
        usage: Option<UniversalUsage>,
    ) -> Self {
        Self {
            id,
            model,
            choices,
            created,
            usage,
            keep_alive: false,
        }
    }

    /// Create a keep-alive chunk that signals the stream is active but has no content.
    ///
    /// Keep-alive chunks are used for:
    /// - SSE ping events
    /// - Anthropic metadata events (message_start, content_block_start/stop)
    /// - Events that don't produce user-visible content
    pub fn keep_alive() -> Self {
        Self {
            id: None,
            model: None,
            choices: Vec::new(),
            created: None,
            usage: None,
            keep_alive: true,
        }
    }

    /// Check if this is a keep-alive chunk.
    pub fn is_keep_alive(&self) -> bool {
        self.keep_alive
    }

    /// Create a simple text delta chunk.
    pub fn text_delta(index: u32, content: &str) -> Self {
        Self::new(
            None,
            None,
            vec![UniversalStreamChoice {
                index,
                delta: Some(serde_json::json!({
                    "role": "assistant",
                    "content": content
                })),
                finish_reason: None,
            }],
            None,
            None,
        )
    }

    /// Create a finish chunk with the given reason.
    pub fn finish(index: u32, reason: &str) -> Self {
        Self::new(
            None,
            None,
            vec![UniversalStreamChoice {
                index,
                delta: Some(serde_json::json!({})),
                finish_reason: Some(reason.to_string()),
            }],
            None,
            None,
        )
    }
}

impl UniversalStreamChoice {
    /// Create a new stream choice with a text delta.
    pub fn text_delta(index: u32, content: &str) -> Self {
        Self {
            index,
            delta: Some(serde_json::json!({
                "role": "assistant",
                "content": content
            })),
            finish_reason: None,
        }
    }

    /// Create a finish choice with the given reason.
    pub fn finish(index: u32, reason: &str) -> Self {
        Self {
            index,
            delta: Some(serde_json::json!({})),
            finish_reason: Some(reason.to_string()),
        }
    }
}

// Implement Serialize for UniversalUsage to support streaming chunk serialization
impl Serialize for UniversalUsage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("UniversalUsage", 4)?;
        if let Some(prompt) = self.prompt_tokens {
            state.serialize_field("prompt_tokens", &prompt)?;
        }
        if let Some(completion) = self.completion_tokens {
            state.serialize_field("completion_tokens", &completion)?;
        }
        if let Some(cached) = self.prompt_cached_tokens {
            state.serialize_field("prompt_cached_tokens", &cached)?;
        }
        if let Some(cache_creation) = self.prompt_cache_creation_tokens {
            state.serialize_field("prompt_cache_creation_tokens", &cache_creation)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for UniversalUsage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            prompt_tokens: Option<i64>,
            completion_tokens: Option<i64>,
            prompt_cached_tokens: Option<i64>,
            prompt_cache_creation_tokens: Option<i64>,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(UniversalUsage {
            prompt_tokens: helper.prompt_tokens,
            completion_tokens: helper.completion_tokens,
            prompt_cached_tokens: helper.prompt_cached_tokens,
            prompt_cache_creation_tokens: helper.prompt_cache_creation_tokens,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keep_alive_chunk() {
        let chunk = UniversalStreamChunk::keep_alive();
        assert!(chunk.is_keep_alive());
        assert!(chunk.choices.is_empty());
    }

    #[test]
    fn test_text_delta_chunk() {
        let chunk = UniversalStreamChunk::text_delta(0, "Hello");
        assert!(!chunk.is_keep_alive());
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].index, 0);

        let delta = chunk.choices[0].delta.as_ref().unwrap();
        assert_eq!(delta["content"], "Hello");
        assert_eq!(delta["role"], "assistant");
    }

    #[test]
    fn test_finish_chunk() {
        let chunk = UniversalStreamChunk::finish(0, "stop");
        assert!(!chunk.is_keep_alive());
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_serialization() {
        let chunk = UniversalStreamChunk::new(
            Some("test-id".to_string()),
            Some("gpt-4".to_string()),
            vec![UniversalStreamChoice::text_delta(0, "Hi")],
            Some(1234567890),
            None,
        );

        let json = serde_json::to_value(&chunk).unwrap();
        assert_eq!(json["id"], "test-id");
        assert_eq!(json["model"], "gpt-4");
        assert_eq!(json["created"], 1234567890);
        assert!(json.get("keep_alive").is_none()); // Should be skipped
    }
}
