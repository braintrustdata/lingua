# Adding a provider

Lingua uses the `ProviderAdapter` trait for unified provider handling. To add a new provider:

## Step 1: Add to `ProviderFormat` enum

In `src/capabilities/format.rs`, add your provider variant:

```rust
pub enum ProviderFormat {
    // ... existing variants ...
    MyProvider,
}
```

Update the `Display`, `FromStr`, and `is_known()` implementations accordingly.

## Step 2: Create provider module with types

Create `src/providers/myprovider/mod.rs` and `src/providers/myprovider/detect.rs`:

```rust
// src/providers/myprovider/detect.rs
use crate::serde_json::Value;
use serde::{Deserialize, Serialize};

/// Request type for MyProvider API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderRequest {
    pub my_required_field: String,
    pub messages: Vec<MyProviderMessage>,
    // ... other fields
}

/// Attempt to parse a payload as MyProvider format.
pub fn try_parse_myprovider(payload: &Value) -> Result<MyProviderRequest, serde_json::Error> {
    serde_json::from_value(payload.clone())
}
```

## Step 3: Implement `ProviderAdapter`

The `ProviderAdapter` trait has 11 required methods organized into 3 categories:

| Category | Method | Purpose |
|----------|--------|---------|
| Metadata | `format()` | Returns the `ProviderFormat` enum variant |
| | `directory_name()` | Directory name for test payloads (e.g., `"openai"`) |
| | `display_name()` | Human-readable name (e.g., `"OpenAI"`) |
| Request | `detect_request()` | Returns `true` if payload matches this provider's request format |
| | `request_to_universal()` | Converts provider request → `UniversalRequest` |
| | `request_from_universal()` | Converts `UniversalRequest` → provider request |
| | `apply_defaults()` | Sets provider-specific defaults (e.g., Anthropic's `max_tokens`) |
| Response | `detect_response()` | Returns `true` if payload matches this provider's response format |
| | `response_to_universal()` | Converts provider response → `UniversalResponse` |
| | `response_from_universal()` | Converts `UniversalResponse` → provider response |
| | `map_finish_reason()` | Maps `FinishReason` enum to provider's string value |

Create `src/providers/myprovider/adapter.rs`:

```rust
use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{collect_extras, ProviderAdapter};
use crate::processing::transform::TransformError;
use crate::providers::myprovider::detect::{try_parse_myprovider, MyProviderRequest};
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::request_response::{
    FinishReason, UniversalParams, UniversalRequest, UniversalResponse, UniversalUsage,
};

/// Known request fields - fields not in this list go into `extras`.
const MYPROVIDER_KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "temperature",
    // ... other standard params
];

pub struct MyProviderAdapter;

impl ProviderAdapter for MyProviderAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::MyProvider
    }

    fn directory_name(&self) -> &'static str {
        "myprovider"  // Used for payload directory in tests/payloads/
    }

    fn display_name(&self) -> &'static str {
        "MyProvider"  // Human-readable name for reports
    }

    fn detect_request(&self, payload: &Value) -> bool {
        try_parse_myprovider(payload).is_ok()
    }

    fn request_to_universal(&self, payload: &Value) -> Result<UniversalRequest, TransformError> {
        let request: MyProviderRequest = serde_json::from_value(payload.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Convert messages to universal format
        let messages = <Vec<Message> as TryFromLLM<Vec<_>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        let params = UniversalParams {
            temperature: request.temperature,
            // ... map other params
            ..Default::default()
        };

        Ok(UniversalRequest {
            model: Some(request.model),
            messages,
            params,
            extras: collect_extras(payload, MYPROVIDER_KNOWN_KEYS),
        })
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        let model = req.model.as_ref().ok_or(TransformError::ValidationFailed {
            target: ProviderFormat::MyProvider,
            reason: "missing model".to_string(),
        })?;

        // Convert messages from universal format
        let provider_messages: Vec<MyProviderMessage> =
            <Vec<MyProviderMessage> as TryFromLLM<Vec<Message>>>::try_from(req.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        let mut obj = Map::new();
        obj.insert("model".into(), Value::String(model.clone()));
        obj.insert("messages".into(), serde_json::to_value(provider_messages)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?);

        // Add params if present
        if let Some(temp) = req.params.temperature {
            obj.insert("temperature".into(), Value::Number(serde_json::Number::from_f64(temp).unwrap()));
        }
        // ... other params

        // Merge extras (preserves provider-specific fields)
        for (k, v) in &req.extras {
            obj.insert(k.clone(), v.clone());
        }

        Ok(Value::Object(obj))
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Set any required defaults (e.g., Anthropic requires max_tokens)
    }

    // ============= Response methods (required) =============

    fn detect_response(&self, payload: &Value) -> bool {
        // Check for provider-specific response structure
        // e.g., OpenAI has choices[].message, Google has candidates[].content
        payload
            .get("my_response_field")
            .is_some()
    }

    fn response_to_universal(&self, payload: &Value) -> Result<UniversalResponse, TransformError> {
        // Extract message content from provider response envelope
        let message_val = payload
            .get("output")
            .ok_or_else(|| TransformError::ToUniversalFailed("missing output".to_string()))?;

        let provider_message: MyProviderMessage = serde_json::from_value(message_val.clone())
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Convert to universal message format
        let messages = <Vec<Message> as TryFromLLM<Vec<MyProviderMessage>>>::try_from(vec![provider_message])
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

        // Extract finish reason (map provider's value to FinishReason)
        let finish_reason = payload
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(FinishReason::from_str);

        // Extract usage info
        let usage = payload.get("usage").map(|u| UniversalUsage {
            input_tokens: u.get("input_tokens").and_then(Value::as_i64),
            output_tokens: u.get("output_tokens").and_then(Value::as_i64),
        });

        Ok(UniversalResponse {
            model: payload.get("model").and_then(Value::as_str).map(String::from),
            messages,
            usage,
            finish_reason,
            extras: Map::new(),
        })
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        // Convert universal messages to provider format
        let provider_messages: Vec<MyProviderMessage> =
            <Vec<MyProviderMessage> as TryFromLLM<Vec<Message>>>::try_from(resp.messages.clone())
                .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

        // Build provider response envelope
        let message = provider_messages.into_iter().next()
            .ok_or_else(|| TransformError::FromUniversalFailed("no messages".to_string()))?;

        let message_value = serde_json::to_value(message)
            .map_err(|e| TransformError::SerializationFailed(e.to_string()))?;

        let stop_reason = self
            .map_finish_reason(resp.finish_reason.as_ref())
            .unwrap_or_else(|| "stop".to_string());

        let mut obj = serde_json::json!({
            "output": message_value,
            "stop_reason": stop_reason
        });

        if let Some(usage) = &resp.usage {
            obj.as_object_mut().unwrap().insert(
                "usage".into(),
                serde_json::json!({
                    "input_tokens": usage.input_tokens.unwrap_or(0),
                    "output_tokens": usage.output_tokens.unwrap_or(0)
                }),
            );
        }

        Ok(obj)
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
        // Map universal FinishReason to provider-specific string
        reason.map(|r| match r {
            FinishReason::Stop => "stop".to_string(),
            FinishReason::Length => "max_tokens".to_string(),
            FinishReason::ToolCalls => "tool_use".to_string(),
            FinishReason::ContentFilter => "content_filter".to_string(),
            FinishReason::Other(s) => s.clone(),
        })
    }
}
```

## Step 4: Register the adapter

In `src/processing/adapters.rs`, add your adapter to the `adapters()` function:

```rust
pub fn adapters() -> Vec<Box<dyn ProviderAdapter>> {
    let mut list: Vec<Box<dyn ProviderAdapter>> = Vec::new();

    // ... existing adapters ...

    #[cfg(feature = "myprovider")]
    list.push(Box::new(crate::providers::myprovider::MyProviderAdapter));

    list
}
```

**Important:** Place more distinctive formats earlier in the list. See detection priority below.

## Step 5: Implement TryFromLLM conversions

In your provider module, implement conversions to/from universal message format:

```rust
// src/providers/myprovider/convert.rs
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;

impl TryFromLLM<Vec<MyProviderMessage>> for Vec<Message> {
    type Error = ConversionError;
    fn try_from(messages: Vec<MyProviderMessage>) -> Result<Self, Self::Error> {
        // Convert provider messages to universal
    }
}

impl TryFromLLM<Vec<Message>> for Vec<MyProviderMessage> {
    type Error = ConversionError;
    fn try_from(messages: Vec<Message>) -> Result<Self, Self::Error> {
        // Convert universal to provider messages
    }
}
```

## Step 6: Export from module

Update `src/providers/myprovider/mod.rs`:

```rust
pub mod adapter;
pub mod convert;
pub mod detect;

pub use adapter::MyProviderAdapter;
pub use detect::{try_parse_myprovider, MyProviderRequest};
```

## Detection priority

Format detection is determined by order in `adapters()`. Check in this order (most specific first):

| Order | Format | Why |
|-------|--------|-----|
| 1 | Responses | OpenAI responses API has distinct structure |
| 2 | Bedrock | `modelId` field is unique |
| 3 | Google | `contents[].parts[]` structure |
| 4 | Anthropic | `max_tokens` required + specific structure |
| 5 | OpenAI | Most permissive (fallback) |

Place more distinctive formats earlier in `adapters()` to avoid false positives.

## Helper functions

The `adapters` module provides helpers for building provider payloads:

```rust
use crate::processing::adapters::{
    collect_extras,      // Extract unknown fields into extras map
    insert_opt_value,    // Insert Option<Value> if Some
    insert_opt_f64,      // Insert Option<f64> as Number
    insert_opt_i64,      // Insert Option<i64> as Number
    insert_opt_bool,     // Insert Option<bool> as Bool
};
```

## Testing

Add tests in your `adapter.rs` for both request and response handling:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_json::json;

    // ============= Request tests =============

    #[test]
    fn test_myprovider_detect_request() {
        let adapter = MyProviderAdapter;
        let payload = json!({
            "model": "my-model",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        assert!(adapter.detect_request(&payload));
    }

    #[test]
    fn test_myprovider_request_round_trip() {
        let adapter = MyProviderAdapter;
        let payload = json!({
            "model": "my-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 0.7
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        let reconstructed = adapter.request_from_universal(&universal).unwrap();

        assert_eq!(reconstructed.get("model").unwrap(), "my-model");
        assert!(reconstructed.get("messages").is_some());
    }

    #[test]
    fn test_myprovider_preserves_extras() {
        let adapter = MyProviderAdapter;
        let payload = json!({
            "model": "my-model",
            "messages": [{"role": "user", "content": "Hello"}],
            "custom_field": "preserved"  // Not in KNOWN_KEYS
        });

        let universal = adapter.request_to_universal(&payload).unwrap();
        assert!(universal.extras.contains_key("custom_field"));

        let reconstructed = adapter.request_from_universal(&universal).unwrap();
        assert_eq!(reconstructed.get("custom_field").unwrap(), "preserved");
    }

    // ============= Response tests =============

    #[test]
    fn test_myprovider_detect_response() {
        let adapter = MyProviderAdapter;
        let payload = json!({
            "output": {"role": "assistant", "content": "Hello!"},
            "stop_reason": "stop"
        });
        assert!(adapter.detect_response(&payload));
    }

    #[test]
    fn test_myprovider_response_round_trip() {
        let adapter = MyProviderAdapter;
        let payload = json!({
            "model": "my-model",
            "output": {"role": "assistant", "content": "Hello!"},
            "stop_reason": "stop",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let universal = adapter.response_to_universal(&payload).unwrap();
        assert!(universal.messages.len() > 0);
        assert_eq!(universal.finish_reason, Some(FinishReason::Stop));

        let reconstructed = adapter.response_from_universal(&universal).unwrap();
        assert!(reconstructed.get("output").is_some());
    }

    #[test]
    fn test_myprovider_map_finish_reason() {
        let adapter = MyProviderAdapter;

        assert_eq!(adapter.map_finish_reason(Some(&FinishReason::Stop)), Some("stop".to_string()));
        assert_eq!(adapter.map_finish_reason(Some(&FinishReason::Length)), Some("max_tokens".to_string()));
        assert_eq!(adapter.map_finish_reason(Some(&FinishReason::ToolCalls)), Some("tool_use".to_string()));
        assert_eq!(adapter.map_finish_reason(None), None);
    }
}
```
