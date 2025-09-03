# Provider type generation pipeline

This document outlines the process for keeping LLMIR's provider types in sync with the latest provider APIs. This pipeline generates Rust types directly from OpenAPI specifications using automated tooling.

**Supported providers**:
- **OpenAI**: Official OpenAPI specification (chat completions API)
- **Anthropic**: Unofficial OpenAPI specification (messages API)

## Summary

**Automated generation**: Uses typify with official OpenAPI specs  
**Approach**: OpenAPI spec download → typify generation → integration  
**Update frequency**: Check monthly or when providers release API updates  

## Pipeline overview

```
OpenAPI Spec Download → Automated Type Generation → Build Integration → Validation
```

## Step-by-step process

### 1. Download latest OpenAPI specification

**Automated approach**: The pipeline script downloads the latest OpenAPI spec automatically.

```bash
./pipelines/generate-provider-types.sh openai      # Generate OpenAI types
./pipelines/generate-provider-types.sh anthropic   # Generate Anthropic types
```

**Spec sources**:
- **OpenAI**: `https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml` → `specs/openai/openapi.yml`
- **Anthropic**: `https://raw.githubusercontent.com/laszukdawid/anthropic-openapi-spec/main/hosted_spec.json` → `specs/anthropic/openapi.json`

**What this provides**:
- Official API specification (always up-to-date)
- Complete type definitions for all endpoints
- No dependency on SDK versioning

### 2. Automated type generation

**Method**: Uses typify library to generate Rust types from OpenAPI schemas

**Build integration**: Types are generated automatically during `cargo build` via `build.rs`

**Generated types**:

**OpenAI** (from official OpenAPI spec):
- `CreateChatCompletionRequest` - Chat completion request parameters
- `CreateChatCompletionResponse` - Standard response format  
- `CreateChatCompletionStreamResponse` - Streaming response format
- `ChatCompletionRequestMessage` - Input message types
- `ChatCompletionResponseMessage` - Output message types
- `ChatCompletionTool` - Tool/function calling types
- `CompletionUsage` - Token usage information

**Anthropic** (from unofficial OpenAPI spec):
- `CreateMessageParams` - Message creation request parameters
- `Message` - Response message format
- `InputMessage` - Input message structure
- `ContentBlock` - Content block types
- `RequestTextBlock` / `ResponseTextBlock` - Text content blocks
- `Tool` / `ToolChoice` - Tool calling types
- `Usage` - Token usage information

**Output locations**: 
- `src/providers/openai/generated.rs`
- `src/providers/anthropic/generated.rs`

### 3. Build process and configuration

**Automatic integration**: Types are generated and integrated during `cargo build`

**Build script features** (`build.rs`):
1. **Reads local OpenAPI spec** from `specs/{provider}/openapi.yml`
2. **Parses YAML** using `serde_yaml` with `arbitrary_precision` support
3. **Generates focused types** for core chat completion APIs only
4. **Handles Rust keywords** automatically (e.g., `type` → `r#type` with `#[serde(rename)]`)
5. **Copies to source** directory for immediate use

**Dependencies**:
- `typify` - OpenAPI to Rust type generation
- `serde_yaml` - YAML parsing
- `serde_json` with `arbitrary_precision` - Large number handling
- `schemars` - JSON Schema support

### 4. Validation and testing

**Automatic validation**: Performed during pipeline execution

**Build validation**:
```bash
cargo build  # Ensures generated types compile correctly
```

**Type compatibility**: Generated types are validated against OpenAPI specification during build

**Integration testing**:
```bash
cargo test  # Run all tests including provider integration tests
cargo run --example simple_openai  # Test actual API usage
```

### 5. Generated output

**Generated files**:
- `src/providers/openai/generated.rs` - All generated types from OpenAPI spec
- Types are automatically integrated into provider module

**Key benefits**:
- **Zero maintenance overhead** - Types auto-update from official specs
- **Complete coverage** - All API types included 
- **Type safety** - Rust compiler ensures correctness
- **No internet dependency** - Build uses local spec files

## Usage in code

```rust
use crate::providers::openai::generated::{
    CreateChatCompletionRequest,
    CreateChatCompletionResponse
};

// Generated types work seamlessly with serde
let request = CreateChatCompletionRequest {
    model: Some(serde_json::Value::String("gpt-4".to_string())),
    messages: Some(vec![...]),
    // ... other fields
};
```

## Focused type generation

The build script generates only essential types for:
- Chat completions API (request/response/streaming)
- Core message and tool types
- Supporting enums and structures

This minimizes generated code size while covering primary use cases.
