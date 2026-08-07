/*!
Universal streaming types for cross-provider stream transformation.

This module provides a canonical representation of LLM streaming chunks that can be
converted to/from any provider format. The format follows OpenAI's streaming chunk
structure as the canonical representation.
*/

use crate::serde_json::{self, Value};
use crate::universal::message::BuiltinToolIdentity;
use crate::universal::response::{ServedServiceTier, UniversalUsage};
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalToolFunctionDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalReasoningDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalToolCallDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_tool_call: Option<bool>,
    /// Typed identity for provider-executed built-in calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin_tool: Option<BuiltinToolIdentity>,
    /// Opaque provider data that must be replayed with this specific call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<UniversalToolFunctionDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UniversalStreamDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<UniversalToolCallDelta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning: Vec<UniversalReasoningDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_signature: Option<String>,
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

    /// Service tier that served this streamed response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served_service_tier: Option<ServedServiceTier>,

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
            served_service_tier: None,
            keep_alive: false,
        }
    }

    pub fn with_served_service_tier(mut self, service_tier: Option<ServedServiceTier>) -> Self {
        self.served_service_tier = service_tier;
        self
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
            served_service_tier: None,
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
    pub fn delta_view(&self) -> Option<UniversalStreamDelta> {
        let delta = self.delta.clone()?;
        serde_json::from_value::<UniversalStreamDelta>(delta).ok()
    }

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

impl From<UniversalStreamDelta> for Value {
    fn from(delta: UniversalStreamDelta) -> Self {
        let has_structured_delta = !delta.tool_calls.is_empty()
            || !delta.reasoning.is_empty()
            || delta.reasoning_signature.is_some();
        let mut map = serde_json::Map::new();
        if let Some(role) = delta.role {
            map.insert("role".into(), Value::String(role));
        }
        if let Some(content) = delta.content {
            map.insert("content".into(), Value::String(content));
        } else if has_structured_delta {
            // Preserve explicit null content for tool/reasoning deltas to maintain
            // roundtrip-equivalent semantics with existing universal stream snapshots.
            map.insert("content".into(), Value::Null);
        }
        if !delta.tool_calls.is_empty() {
            let value = serde_json::to_value(delta.tool_calls).unwrap_or(Value::Array(vec![]));
            map.insert("tool_calls".into(), value);
        }
        if !delta.reasoning.is_empty() {
            let value = serde_json::to_value(delta.reasoning).unwrap_or(Value::Array(vec![]));
            map.insert("reasoning".into(), value);
        }
        if let Some(signature) = delta.reasoning_signature {
            map.insert("reasoning_signature".into(), Value::String(signature));
        }
        Value::Object(map)
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
    fn test_stream_choice_delta_view() {
        let choice = UniversalStreamChoice {
            index: 0,
            delta: Some(crate::serde_json::json!({
                "role": "assistant",
                "content": "hello",
                "tool_calls": [
                    {
                        "index": 1,
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"SF\"}"
                        }
                    }
                ]
            })),
            finish_reason: None,
        };

        let delta = choice.delta_view().unwrap();
        assert_eq!(delta.role.as_deref(), Some("assistant"));
        assert_eq!(delta.content.as_deref(), Some("hello"));
        assert_eq!(delta.tool_calls.len(), 1);
        assert_eq!(delta.tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(
            delta.tool_calls[0]
                .function
                .as_ref()
                .and_then(|f| f.name.as_deref()),
            Some("get_weather")
        );
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

    #[test]
    fn test_usage_details_roundtrip() {
        let usage = UniversalUsage {
            total_tokens: Some(5),
            input_details: Some(crate::universal::response::InputTokenDetails {
                content_by_modality: Some(vec![crate::universal::response::ModalityTokenCount {
                    modality: Some(crate::universal::response::TokenModality::Text),
                    token_count: Some(3),
                }]),
                tool_prompt: Some(crate::universal::response::TokenBreakdown {
                    total_tokens: Some(2),
                    by_modality: Some(vec![crate::universal::response::ModalityTokenCount {
                        modality: Some(crate::universal::response::TokenModality::Text),
                        token_count: Some(2),
                    }]),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let value = serde_json::to_value(&usage).unwrap();
        assert_eq!(
            value,
            crate::serde_json::json!({
                "total_tokens": 5,
                "input_details": {
                    "content_by_modality": [{
                        "modality": "text",
                        "token_count": 3
                    }],
                    "tool_prompt": {
                        "total_tokens": 2,
                        "by_modality": [{
                            "modality": "text",
                            "token_count": 2
                        }]
                    }
                }
            })
        );
        let roundtrip: UniversalUsage = serde_json::from_value(value).unwrap();

        assert!(!roundtrip.prompt_tokens_exclude_cache);
        assert_eq!(roundtrip.total_tokens, Some(5));
        assert_eq!(roundtrip.input_details, usage.input_details);

        let exclusive = UniversalUsage {
            prompt_tokens_exclude_cache: true,
            ..Default::default()
        };
        let value = serde_json::to_value(exclusive).unwrap();
        assert_eq!(
            value,
            crate::serde_json::json!({"prompt_tokens_exclude_cache": true})
        );
    }

    #[test]
    fn test_stream_delta_reasoning_from_into_value() {
        let delta = UniversalStreamDelta {
            role: Some("assistant".to_string()),
            reasoning: vec![UniversalReasoningDelta {
                content: Some("thought".to_string()),
            }],
            reasoning_signature: Some("sig_123".to_string()),
            ..Default::default()
        };

        let value = Value::from(delta.clone());
        let parsed: UniversalStreamDelta = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.role.as_deref(), Some("assistant"));
        assert_eq!(parsed.reasoning.len(), 1);
        assert_eq!(parsed.reasoning[0].content.as_deref(), Some("thought"));
        assert_eq!(parsed.reasoning_signature.as_deref(), Some("sig_123"));
    }

    #[test]
    fn test_stream_delta_builtin_tool_identity_and_signature_from_into_value() {
        use crate::universal::tools::BuiltinToolProvider;

        let delta = UniversalStreamDelta {
            tool_calls: vec![UniversalToolCallDelta {
                index: Some(0),
                call_type: Some("builtin_tool_call".to_string()),
                builtin_tool: Some(BuiltinToolIdentity {
                    provider: BuiltinToolProvider::Google,
                    builtin_type: "GOOGLE_SEARCH_WEB".to_string(),
                }),
                encrypted_content: Some("call_signature".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let value = Value::from(delta);
        assert_eq!(
            value["tool_calls"][0]["builtin_tool"],
            crate::serde_json::json!({
                "provider": "google",
                "builtin_type": "GOOGLE_SEARCH_WEB"
            })
        );
        assert_eq!(
            value["tool_calls"][0]["encrypted_content"],
            "call_signature"
        );

        let parsed: UniversalStreamDelta = serde_json::from_value(value).unwrap();
        let tool_call = &parsed.tool_calls[0];
        assert_eq!(
            tool_call.builtin_tool.as_ref().unwrap().provider,
            BuiltinToolProvider::Google
        );
        assert_eq!(
            tool_call.encrypted_content.as_deref(),
            Some("call_signature")
        );
    }
}
