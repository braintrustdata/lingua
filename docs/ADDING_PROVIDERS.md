# Adding a Provider

Lingua uses struct-based validation for provider format detection. To add a new provider:

## Step 1: Add to `ProviderFormat` enum

In `src/capabilities/format.rs`, add your provider variant:

```rust
pub enum ProviderFormat {
    // ... existing variants ...
    MyProvider,
}
```

Update the `Display`, `FromStr`, and `is_known()` implementations accordingly.

## Step 2: Create provider module with parser

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
/// Returns Ok if the payload matches the expected structure.
pub fn try_parse_myprovider(payload: &Value) -> Result<MyProviderRequest, serde_json::Error> {
    serde_json::from_value(payload.clone())
}
```

## Step 3: Register in transform.rs

In `src/processing/transform.rs`, add your provider to the detection and conversion functions:

```rust
// In is_valid_for_format()
#[cfg(feature = "myprovider")]
ProviderFormat::MyProvider => try_parse_myprovider(payload).is_ok(),

// In detect_source_format()
#[cfg(feature = "myprovider")]
if try_parse_myprovider(payload).is_ok() {
    return Ok(ProviderFormat::MyProvider);
}

// In to_universal()
#[cfg(feature = "myprovider")]
ProviderFormat::MyProvider => {
    // Convert MyProvider messages to universal format
}

// In from_universal()
#[cfg(feature = "myprovider")]
ProviderFormat::MyProvider => {
    // Convert universal messages to MyProvider format
}
```

## Step 4: Implement TryFromLLM conversions

In your provider module, implement conversions to/from universal format:

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

## Detection Priority

Format detection uses struct-based validation, checked in this order (most specific first):

| Order | Format | Why |
|-------|--------|-----|
| 1 | Bedrock | `modelId` field is unique |
| 2 | Google | `contents[].parts[]` structure |
| 3 | Anthropic | `max_tokens` required |
| 4 | Mistral | Model name prefix heuristic |
| 5 | OpenAI | Most permissive (fallback) |

Place more distinctive formats earlier in `detect_source_format()` to avoid false positives.

