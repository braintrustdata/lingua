# Adding a provider format

This guide walks through adding support for a new LLM provider to lingua using a test-first approach.

## Overview

Lingua uses the `ProviderAdapter` trait for unified provider handling. Each adapter handles:
- **Format detection**: Identifying payloads belonging to this provider
- **Request transformation**: Converting to/from universal format
- **Response transformation**: Converting to/from universal format
- **Streaming**: Handling streaming chunks (optional)

The transformation follows a three-layer pattern:

```
Source Payload → Universal Format → Target Payload
```

When source and target formats match, lingua returns the original bytes unchanged (zero-copy passthrough).

## Workflow summary

1. **Add payload snapshots** - Create test fixtures from real API calls
2. **Add to ProviderFormat enum** - Register the new format
3. **Create provider module** - Types based on your snapshots
4. **Implement ProviderAdapter** - Fill in methods one by one
5. **Run coverage-report** - Validate transformations and roundtrips
6. **Iterate** - Fix failures until all tests pass

---

## Step 1: Add payload snapshots

**Location**: `payloads/`

Start by capturing real API payloads. These become your test fixtures and source of truth for the provider's format.

### Setup

```bash
cd payloads
pnpm install
```

Set environment variables for API access:
```bash
export OPENAI_API_KEY="..."
export ANTHROPIC_API_KEY="..."
export GOOGLE_API_KEY="..."
export MY_PROVIDER_API_KEY="..."  # Your provider's API key
```

### Output structure

Captured payloads are saved to `payloads/snapshots/`:

```
payloads/snapshots/
├── simpleRequest/
│   ├── myprovider/           # Your provider's directory
│   │   ├── request.json
│   │   ├── response.json
│   │   ├── response-streaming.json
│   │   ├── followup-request.json
│   │   ├── followup-response.json
│   │   └── followup-response-streaming.json
│   ├── anthropic/
│   ├── chat-completions/
│   └── ...
├── toolCallRequest/
└── ...
```

### File naming convention

| File | Description |
|------|-------------|
| `request.json` | First turn request |
| `response.json` | Non-streaming response |
| `response-streaming.json` | Streaming events (JSON array) |
| `followup-request.json` | Second turn request (includes assistant response) |
| `followup-response.json` | Second turn response |
| `followup-response-streaming.json` | Second turn streaming |

### Option A: Add to the capture system (recommended)

The capture system automatically handles streaming, follow-up conversations, and tool call responses.

#### 1. Add provider to types

Edit `payloads/cases/types.ts`:

```typescript
export interface TestCase {
  "chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  responses: OpenAI.Responses.ResponseCreateParams | null;
  anthropic: Anthropic.Messages.MessageCreateParams | null;
  google: GenerateContentRequest | null;
  bedrock: BedrockConverseRequest | null;
  myprovider: MyProviderRequest | null;  // Add this
}

export const PROVIDER_TYPES = [
  "chat-completions",
  "responses",
  "anthropic",
  "google",
  "bedrock",
  "myprovider",  // Add this
] as const;
```

#### 2. Add model configuration

Edit `payloads/cases/models.ts`:

```typescript
export const MYPROVIDER_MODEL = "my-model-name";
```

#### 3. Add test cases

Edit `payloads/cases/simple.ts`:

```typescript
import { MYPROVIDER_MODEL } from "./models";

export const simpleCases: TestCaseCollection = {
  simpleRequest: {
    // ... existing providers ...

    myprovider: {
      model: MYPROVIDER_MODEL,
      messages: [
        { role: "user", content: "Say a one-sentence greeting." }
      ],
    },
  },

  // Add other cases: toolCallRequest, multimodalRequest, etc.
};
```

#### 4. Create provider executor

Create `payloads/scripts/providers/myprovider.ts`:

```typescript
import { CaptureResult, ProviderExecutor } from "../types";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";

type MyProviderRequest = { /* match your API */ };
type MyProviderResponse = { /* match your API */ };
type MyProviderStreamChunk = { /* match your API */ };

export const myproviderCases: Record<string, MyProviderRequest> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(allTestCases, caseName, "myprovider");
  if (caseData) {
    myproviderCases[caseName] = caseData;
  }
});

export async function executeMyProvider(
  caseName: string,
  payload: MyProviderRequest,
  stream?: boolean
): Promise<CaptureResult<MyProviderRequest, MyProviderResponse, MyProviderStreamChunk>> {
  const client = new MyProviderClient({ apiKey: process.env.MY_PROVIDER_API_KEY });
  const result: CaptureResult<...> = { request: payload };

  // Non-streaming call
  if (stream !== true) {
    result.response = await client.create({ ...payload, stream: false });
  }

  // Streaming call
  if (stream !== false) {
    const chunks = [];
    const streamResponse = await client.create({ ...payload, stream: true });
    for await (const chunk of streamResponse) {
      chunks.push(chunk);
    }
    result.streamingResponse = chunks;
  }

  // Build follow-up request (append assistant response + user message)
  // Handle tool calls if present
  // ...

  return result;
}

export const myproviderExecutor: ProviderExecutor<...> = {
  name: "myprovider",
  cases: myproviderCases,
  execute: executeMyProvider,
};
```

#### 5. Register executor

Edit `payloads/scripts/capture.ts`:

```typescript
import { myproviderExecutor } from "./providers/myprovider";

const allProviders = [
  // ... existing providers ...
  myproviderExecutor,
] as const;
```

#### 6. Run capture

```bash
# Capture all cases for your provider
pnpm capture --providers myprovider

# Capture specific cases
pnpm capture --providers myprovider --cases simpleRequest

# Force re-capture (skip cache)
pnpm capture --providers myprovider --force

# List available cases and status
pnpm capture --list
```

### Option B: Manually capture payloads

For quick prototyping, manually create the JSON files:

1. Make a real API call to your provider
2. Save the request body as `request.json`
3. Save the response body as `response.json`
4. For streaming, collect all SSE event payloads into a JSON array as `response-streaming.json`

```bash
mkdir -p payloads/snapshots/simpleRequest/myprovider
```

**`payloads/snapshots/simpleRequest/myprovider/request.json`**:
```json
{
  "model": "my-model-name",
  "messages": [
    {"role": "user", "content": "Hello, how are you?"}
  ]
}
```

**`payloads/snapshots/simpleRequest/myprovider/response.json`**:
```json
{
  "model": "my-model-name",
  "output": {
    "role": "assistant",
    "content": "I'm doing well, thank you for asking!"
  },
  "stop_reason": "stop",
  "usage": {
    "input_tokens": 12,
    "output_tokens": 15
  }
}
```

### Recommended test cases

| Case | Tests | Priority |
|------|-------|----------|
| `simpleRequest` | Basic text chat | Required |
| `toolCallRequest` | Function/tool calling | High |
| `multimodalRequest` | Images, files | Medium |
| `reasoningRequest` | Extended thinking | If supported |

### Capture commands reference

```bash
# List all cases and capture status
pnpm capture --list

# Capture all providers, all cases
pnpm capture

# Filter by provider
pnpm capture --providers myprovider
pnpm capture --providers myprovider,anthropic

# Filter by case
pnpm capture --cases simpleRequest,toolCallRequest

# Filter by name pattern
pnpm capture --filter reasoning

# Streaming only / non-streaming only
pnpm capture --stream true
pnpm capture --stream false

# Force re-capture (ignore cache)
pnpm capture --force

# Combine filters
pnpm capture --providers myprovider --cases simpleRequest --force
```

### Prune orphaned snapshots

Remove snapshots that no longer have corresponding test cases:

```bash
pnpm prune
```

---

## Step 2: Add to ProviderFormat enum

**Location**: `crates/lingua/src/capabilities/format.rs`

```rust
pub enum ProviderFormat {
    OpenAI,
    Anthropic,
    Google,
    Mistral,
    Converse,   // Bedrock
    Responses,  // OpenAI Responses API
    MyProvider, // Add here
    Unknown,
}
```

Update the implementations:

```rust
// Display
impl std::fmt::Display for ProviderFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            // ... existing variants ...
            ProviderFormat::MyProvider => "myprovider",
            ProviderFormat::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

// FromStr
impl std::str::FromStr for ProviderFormat {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            // ... existing variants ...
            "myprovider" => Ok(ProviderFormat::MyProvider),
            _ => Err(()),
        }
    }
}
```

---

## Step 3: Create provider module

Create the directory structure:

```
crates/lingua/src/providers/myprovider/
├── mod.rs
├── adapter.rs
├── convert.rs
└── detect.rs
```

### detect.rs

Define types based on your payload snapshots:

```rust
use serde::{Deserialize, Serialize};

/// Request type - match structure from your request.json snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderRequest {
    pub model: String,
    pub messages: Vec<MyProviderMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    // Add fields as you see them in your snapshots
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderMessage {
    pub role: String,
    pub content: String,
}

/// Response type - match structure from your response.json snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderResponse {
    pub model: Option<String>,
    pub output: MyProviderOutputMessage,
    pub stop_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<MyProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderOutputMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Detection function - tries to parse payload as this provider's format
pub fn try_parse_myprovider(
    payload: &serde_json::Value,
) -> Result<MyProviderRequest, serde_json::Error> {
    serde_json::from_value(payload.clone())
}
```

### mod.rs

```rust
pub mod adapter;
pub mod convert;
pub mod detect;

pub use adapter::MyProviderAdapter;
pub use detect::{try_parse_myprovider, MyProviderRequest, MyProviderResponse};
```

### Add to providers/mod.rs

```rust
#[cfg(feature = "myprovider")]
pub mod myprovider;
```

### Add feature flag to Cargo.toml

```toml
[features]
default = ["openai", "anthropic", "google", "bedrock", "myprovider"]
myprovider = []
```

---

## Step 4: Implement ProviderAdapter

**Location**: `crates/lingua/src/providers/myprovider/adapter.rs`

The `ProviderAdapter` trait has 11 required methods:

| Category | Method | Purpose |
|----------|--------|---------|
| **Metadata** | `format()` | Returns the `ProviderFormat` enum variant |
| | `directory_name()` | Directory name in `payloads/snapshots/` |
| | `display_name()` | Human-readable name for reports |
| **Request** | `detect_request()` | Returns `true` if payload matches this format |
| | `request_to_universal()` | Provider request → `UniversalRequest` |
| | `request_from_universal()` | `UniversalRequest` → Provider request |
| | `apply_defaults()` | Set required defaults (e.g., Anthropic's `max_tokens`) |
| **Response** | `detect_response()` | Returns `true` if payload matches this format |
| | `response_to_universal()` | Provider response → `UniversalResponse` |
| | `response_from_universal()` | `UniversalResponse` → Provider response |
| | `map_finish_reason()` | Map `FinishReason` to provider's string |

### Start with a skeleton

```rust
use crate::capabilities::ProviderFormat;
use crate::processing::adapters::{collect_extras, insert_opt_f64, ProviderAdapter};
use crate::processing::transform::TransformError;
use crate::serde_json::{self, Map, Value};
use crate::universal::convert::TryFromLLM;
use crate::universal::message::Message;
use crate::universal::{FinishReason, UniversalRequest, UniversalResponse, UniversalUsage};

use super::detect::{try_parse_myprovider, MyProviderMessage};

/// Fields to extract to UniversalRequest.params - everything else goes to extras
const MYPROVIDER_KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "temperature",
    "max_tokens",
];

pub struct MyProviderAdapter;

impl ProviderAdapter for MyProviderAdapter {
    fn format(&self) -> ProviderFormat {
        ProviderFormat::MyProvider
    }

    fn directory_name(&self) -> &'static str {
        "myprovider"  // Must match payloads/snapshots/*/myprovider/
    }

    fn display_name(&self) -> &'static str {
        "MyProvider"
    }

    fn detect_request(&self, payload: &Value) -> bool {
        // Start simple, refine based on coverage-report failures
        try_parse_myprovider(payload).is_ok()
    }

    fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
        todo!("Implement based on your request.json snapshots")
    }

    fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
        todo!("Implement to produce your provider's request format")
    }

    fn apply_defaults(&self, _req: &mut UniversalRequest) {
        // Set required defaults if your provider needs them
    }

    fn detect_response(&self, payload: &Value) -> bool {
        // Check for your provider's response structure
        payload.get("output").is_some() && payload.get("stop_reason").is_some()
    }

    fn response_to_universal(&self, payload: Value) -> Result<UniversalResponse, TransformError> {
        todo!("Implement based on your response.json snapshots")
    }

    fn response_from_universal(&self, resp: &UniversalResponse) -> Result<Value, TransformError> {
        todo!("Implement to produce your provider's response format")
    }

    fn map_finish_reason(&self, reason: Option<&FinishReason>) -> Option<String> {
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

### Register the adapter

**Location**: `crates/lingua/src/processing/adapters.rs`

```rust
static ADAPTERS: LazyLock<Vec<Box<dyn ProviderAdapter>>> = LazyLock::new(|| {
    let mut list: Vec<Box<dyn ProviderAdapter>> = Vec::new();

    // Order matters! More specific formats first
    #[cfg(feature = "openai")]
    list.push(Box::new(crate::providers::openai::ResponsesAdapter));

    #[cfg(feature = "myprovider")]  // Add with feature gate
    list.push(Box::new(crate::providers::myprovider::MyProviderAdapter));

    #[cfg(feature = "bedrock")]
    list.push(Box::new(crate::providers::bedrock::BedrockAdapter));

    // ... other adapters ...

    #[cfg(feature = "openai")]
    list.push(Box::new(crate::providers::openai::OpenAIAdapter));  // Last - most permissive

    list
});
```

**Detection priority**: Place more distinctive formats earlier. OpenAI must be last because its detection is permissive.

---

## Step 5: Run coverage-report

Now run the coverage report to see what's failing:

```bash
cargo run --bin coverage-report
```

This tests:
1. **Cross-provider transformations**: Source → Target for all provider pairs
2. **Roundtrip transformations**: Provider → Universal → Provider

### Reading the output

```
## Cross-Provider Transformation Coverage

| Source → Target | OpenAI | Anthropic | MyProvider |
|-----------------|--------|-----------|------------|
| OpenAI          | ✓      | ✓         | ✗          |  <- MyProvider failing
| Anthropic       | ✓      | ✓         | ✗          |
| MyProvider      | ✗      | ✗         | ✓          |  <- Your provider

## Roundtrip Transform Coverage

| Provider   | Request | Response | Streaming |
|------------|---------|----------|-----------|
| MyProvider | ✗       | ✗        | -         |  <- Implement these
```

The `✗` marks show what needs implementation.

---

## Step 6: Iterate until tests pass

### Implementing request_to_universal

Look at your `request.json` snapshot and extract fields:

```rust
fn request_to_universal(&self, payload: Value) -> Result<UniversalRequest, TransformError> {
    let request: MyProviderRequest = serde_json::from_value(payload.clone())
        .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

    // Convert messages using TryFromLLM (implement in convert.rs)
    let messages: Vec<Message> =
        <Vec<Message> as TryFromLLM<Vec<MyProviderMessage>>>::try_from(request.messages)
            .map_err(|e| TransformError::ToUniversalFailed(e.to_string()))?;

    Ok(UniversalRequest {
        model: Some(request.model),
        messages,
        params: crate::universal::UniversalParams {
            temperature: request.temperature,
            ..Default::default()
        },
        extras: collect_extras(&payload, MYPROVIDER_KNOWN_KEYS),
    })
}
```

### Implementing request_from_universal

Build the provider format from universal:

```rust
fn request_from_universal(&self, req: &UniversalRequest) -> Result<Value, TransformError> {
    let model = req.model.clone().ok_or_else(|| TransformError::ValidationFailed {
        target: ProviderFormat::MyProvider,
        reason: "model is required".to_string(),
    })?;

    let provider_messages: Vec<MyProviderMessage> =
        <Vec<MyProviderMessage> as TryFromLLM<Vec<Message>>>::try_from(req.messages.clone())
            .map_err(|e| TransformError::FromUniversalFailed(e.to_string()))?;

    let mut obj = Map::new();
    obj.insert("model".into(), Value::String(model));
    obj.insert("messages".into(), serde_json::to_value(provider_messages)
        .map_err(|e| TransformError::SerializationFailed(e.to_string()))?);

    insert_opt_f64(&mut obj, "temperature", req.params.temperature);

    // Merge extras (preserves unknown fields for roundtrip)
    for (k, v) in &req.extras {
        if !MYPROVIDER_KNOWN_KEYS.contains(&k.as_str()) {
            obj.insert(k.clone(), v.clone());
        }
    }

    Ok(Value::Object(obj))
}
```

### Implementing TryFromLLM conversions

**Location**: `crates/lingua/src/providers/myprovider/convert.rs`

```rust
use crate::error::ConvertError;
use crate::universal::convert::TryFromLLM;
use crate::universal::message::{AssistantContent, Message, UserContent};

use super::detect::MyProviderMessage;

// Provider -> Universal
impl TryFromLLM<MyProviderMessage> for Message {
    type Error = ConvertError;

    fn try_from(msg: MyProviderMessage) -> Result<Self, Self::Error> {
        match msg.role.as_str() {
            "user" => Ok(Message::User {
                content: UserContent::String(msg.content),
            }),
            "assistant" => Ok(Message::Assistant {
                content: AssistantContent::String(msg.content),
                id: None,
            }),
            "system" => Ok(Message::System {
                content: UserContent::String(msg.content),
            }),
            other => Err(ConvertError::InvalidRole { role: other.to_string() }),
        }
    }
}

// Universal -> Provider
impl TryFromLLM<Message> for MyProviderMessage {
    type Error = ConvertError;

    fn try_from(msg: Message) -> Result<Self, Self::Error> {
        match msg {
            Message::User { content } => Ok(MyProviderMessage {
                role: "user".to_string(),
                content: content.to_string(),
            }),
            Message::Assistant { content, .. } => Ok(MyProviderMessage {
                role: "assistant".to_string(),
                content: content.to_string(),
            }),
            Message::System { content } => Ok(MyProviderMessage {
                role: "system".to_string(),
                content: content.to_string(),
            }),
            Message::Tool { .. } => {
                // Handle tool results based on your provider's format
                todo!("Implement tool result handling")
            }
        }
    }
}
```

### Run coverage-report again

```bash
cargo run --bin coverage-report
```

Watch the `✗` marks turn to `✓` as you implement each method.

### Roundtrip validation

The coverage report compares original JSON with roundtripped JSON. If fields are lost or changed, you'll see:

```
Roundtrip diff for myprovider/simpleRequest:
  - lost_fields: ["custom_field"]
  - added_fields: []
  - changed_fields: ["temperature"]
```

Fix by:
- Adding missing fields to `KNOWN_KEYS` or handling in `extras`
- Ensuring numeric precision is preserved
- Checking field name casing matches

---

## Streaming support (optional)

If your provider supports streaming, override the default methods:

```rust
fn detect_stream_response(&self, payload: &Value) -> bool {
    payload.get("type").and_then(Value::as_str)
        .map(|t| t.starts_with("stream."))
        .unwrap_or(false)
}

fn stream_to_universal(
    &self,
    payload: Value,
) -> Result<Option<UniversalStreamChunk>, TransformError> {
    // Parse streaming events from your response-streaming.json snapshots
    todo!()
}

fn stream_from_universal(
    &self,
    chunk: &UniversalStreamChunk,
) -> Result<Value, TransformError> {
    todo!()
}
```

Add `response-streaming.json` snapshots and run coverage-report to validate.

---

## Helper functions

The `adapters` module provides helpers:

```rust
use crate::processing::adapters::{
    collect_extras,      // Extract unknown fields into extras map
    insert_opt_value,    // Insert Option<Value> if Some
    insert_opt_f64,      // Insert Option<f64> as Number
    insert_opt_i64,      // Insert Option<i64> as Number
    insert_opt_bool,     // Insert Option<bool> as Bool
    insert_opt_string,   // Insert Option<&str> as String
};
```

---

## Provider comparison reference

### Field mappings by provider

| Field | OpenAI | Anthropic | Google | Bedrock |
|-------|--------|-----------|--------|---------|
| Model | `model` | `model` | `model` | `modelId` |
| Messages | `messages` | `messages` | `contents` | `messages` |
| Temperature | `temperature` | `temperature` | `generationConfig.temperature` | `inferenceConfig.temperature` |
| Max tokens | `max_tokens` | `max_tokens` (required) | `generationConfig.maxOutputTokens` | `inferenceConfig.maxTokens` |
| System | In messages | Separate `system` param | `systemInstruction` | Separate `system` array |

### Finish reason mappings

| Universal | OpenAI | Anthropic | Google | Bedrock |
|-----------|--------|-----------|--------|---------|
| `Stop` | `"stop"` | `"end_turn"` | `"STOP"` | `"end_turn"` |
| `Length` | `"length"` | `"max_tokens"` | `"MAX_TOKENS"` | `"max_tokens"` |
| `ToolCalls` | `"tool_calls"` | `"tool_use"` | `"TOOL_CALLS"` | `"tool_use"` |
| `ContentFilter` | `"content_filter"` | `"content_filter"` | `"SAFETY"` | `"content_filtered"` |

### Detection priority

| Priority | Format | Distinctive feature |
|----------|--------|---------------------|
| 1 | Responses | Has `input` field, no `messages` |
| 2 | Bedrock | Has `modelId` (not `model`) |
| 3 | Google | Has `contents[].parts[]` |
| 4 | Anthropic | Requires `max_tokens` |
| 5 | OpenAI | Most permissive (fallback) |
