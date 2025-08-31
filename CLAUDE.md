# Elmir project guide for Claude

This guide helps AI assistants understand and work with the Elmir codebase effectively.

## Project overview

Elmir (LLM Intermediate Representation) is a universal message format that compiles to provider-specific formats with zero runtime overhead. It's designed to allow seamless interoperability between different LLM providers without runtime penalties.

## Key principles

- **Universal compatibility**: Supports 100% of provider-specific quirks and capabilities
- **Zero runtime overhead**: Pure compile-time translation to native provider formats  
- **Type safety**: Full TypeScript and Rust type generation with bidirectional validation
- **No network calls**: This is a message format library, not an API client

## Documentation style guide

**Always use sentence case for all headings, not title case**:
- ✅ `## Pipeline overview` 
- ❌ `## Pipeline Overview`

**Be concise and direct**:
- Focus on what, not why (unless specifically asked)
- Avoid unnecessary explanations or summaries
- Use bullet points and structured formats

## Project structure

```
src/
├── universal/             # Core LLMIR message types
├── providers/             # Provider-specific API type definitions
├── translators/           # Bidirectional format conversion logic
├── capabilities/          # Provider capability detection
└── lib.rs                 # Main entry point and re-exports
```

## Working with providers

Each provider should have:
- **Separate request/response types**: Don't conflate them into single structs
- **Complete type coverage**: All fields from provider SDKs, even optional ones
- **Validation tests**: TypeScript compatibility tests in `tests/typescript/{provider}/`

## Type generation workflow

1. **Check for SDK updates** in provider test directories
2. **Extract TypeScript types** manually from provider SDKs
3. **Convert to Rust** following consistent patterns (see pipelines/ docs)
4. **Validate compatibility** through multi-layer testing
5. **Update translators** to use new types

## Common patterns

**Rust type derivations**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")] // when needed
```

**TypeScript exports** (for ts-rs):
```rust
#[derive(TS)]
#[ts(export, export_to = "bindings/typescript/")]
```

**Optional fields**: Always use `Option<T>` for optional provider fields

**Union types**: Convert TypeScript unions to Rust enums or separate structs

## Testing approach

**Type compatibility**: Verify Rust-generated TypeScript matches provider SDK types
**Round-trip testing**: Ensure lossless serialization/deserialization
**Real API integration**: Test with actual provider APIs when possible

## Development priorities

1. **Correctness over convenience**: Match provider APIs exactly
2. **Type safety over flexibility**: Strict typing prevents runtime errors
3. **Manual precision over automation**: Control type design decisions
4. **Validation over assumptions**: Test everything thoroughly

## File naming conventions

- Provider modules: `src/providers/{provider}/` (e.g., `openai/`, `anthropic/`)
- Request types: `{provider}_request.rs` or `request.rs` in provider directory  
- Response types: `{provider}_response.rs` or `response.rs` in provider directory
- Tests: `tests/typescript/{provider}/` with provider-specific validation

## ⚠️ CRITICAL: Never edit generated files directly

**🚨 DO NOT EDIT `generated.rs` FILES DIRECTLY 🚨**

Files named `generated.rs` are automatically generated and will be overwritten:
- `src/providers/google/generated.rs` - Generated from protobuf files
- `src/providers/openai/generated.rs` - Generated from OpenAPI specs  
- `src/providers/anthropic/generated.rs` - Generated from OpenAPI specs

**If you need to fix issues in generated files:**
1. ✅ **DO**: Edit the generation logic in `scripts/generate-types.rs`
2. ✅ **DO**: Add fixes to the `fix_google_type_references()` or similar functions
3. ✅ **DO**: Regenerate using `cargo run --bin generate-types <provider>`
4. ❌ **DON'T**: Edit the generated files directly - your changes will be lost!

**Example of proper fix approach:**
```rust
// In scripts/generate-types.rs, in fix_google_type_references():
fn fix_google_type_references(content: String) -> String {
    let mut fixed = content;
    
    // Fix doctest JSON examples that fail to compile
    fixed = fixed.replace(
        "    /// ```\n    /// {\n    ///    \"type\": \"object\",",
        "    /// ```json\n    /// {\n    ///    \"type\": \"object\","
    );
    
    fixed
}
```

This ensures fixes are permanent and survive regeneration cycles.

## Common gotchas

**TypeScript → Rust conversions**:
- `string | number` unions need careful handling (usually separate enums)
- Optional properties (`field?:`) become `Option<field>`
- Nested objects may need `serde_json::Value` for unknown structures
- Array types become `Vec<T>`

**Serde configuration**:
- Use `rename_all = "snake_case"` sparingly (only when provider uses snake_case)
- Most providers use camelCase, so default serde behavior is correct
- Add `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields

## Pipeline maintenance

The `pipelines/` directory contains automated tooling for:
- Downloading latest OpenAPI specifications from providers
- Generating Rust types automatically using typify
- Building and validating generated code
- Minimal type generation focused on chat completion APIs

Run the pipeline to update provider types:
```bash
./pipelines/generate-provider-types.sh openai
```

This process is fully automated and generates only essential types to minimize code size.

## Development setup

**Git hooks installation**:
After cloning the repository, install pre-commit hooks for consistent formatting:
```bash
./scripts/install-hooks.sh
```

This installs hooks that automatically run:
- `cargo fmt` - ensures consistent formatting

**Code quality checks**:
Clippy linting is handled by GitHub Actions CI and will run on pull requests.
- `cargo clippy` - catches common issues and enforces best practices

Hooks run automatically before each commit. To bypass temporarily: `git commit --no-verify`

## Adding new providers

Follow this step-by-step guide to add support for a new LLM provider:

### 1. Create provider directory structure

```bash
mkdir -p src/providers/{provider}
touch src/providers/{provider}/mod.rs
touch src/providers/{provider}/request.rs  
touch src/providers/{provider}/response.rs
```

### 2. Add feature flag to Cargo.toml

```toml
[features]
default = ["openai", "anthropic", "google", "bedrock", "{provider}"]
{provider} = ["dep:{provider-sdk}"]  # Only if external SDK needed

[dependencies]
{provider-sdk} = { version = "1.0", optional = true }  # If needed
```

### 3. Create provider types

**src/providers/{provider}/request.rs**:
```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct {Provider}Request {
    pub messages: Vec<{Provider}Message>,
    pub model: String,
    // ... other required fields
}

// Define all necessary types following provider API exactly
```

**src/providers/{provider}/response.rs**:
```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct {Provider}Response {
    pub choices: Vec<{Provider}Choice>,
    pub usage: {Provider}Usage,
    // ... other response fields
}
```

**src/providers/{provider}/mod.rs**:
```rust
/*!
{Provider} API provider types.
*/

pub mod request;
pub mod response;

pub use request::{Provider}Request;
pub use response::{Provider}Response;
```

### 4. Add conditional compilation

**src/providers/mod.rs**:
```rust
#[cfg(feature = "{provider}")]
pub mod {provider};
```

### 5. Create translator

**src/translators/{provider}.rs**:
```rust
use crate::providers::{provider}::{Provider}Request, {Provider}Response};
use crate::translators::{TranslationResult, Translator};
use crate::universal::{SimpleMessage, SimpleRole};

pub struct {Provider}Translator;

impl Translator<{Provider}Request, {Provider}Response> for {Provider}Translator {
    fn to_provider_request(messages: Vec<SimpleMessage>) -> TranslationResult<{Provider}Request> {
        // Convert SimpleMessage to provider format
        todo!()
    }

    fn from_provider_response(response: {Provider}Response) -> TranslationResult<Vec<SimpleMessage>> {
        // Convert provider response back to SimpleMessage  
        todo!()
    }
}

// Convenience functions
pub fn to_{provider}_format(messages: Vec<SimpleMessage>) -> TranslationResult<{Provider}Request> {
    {Provider}Translator::to_provider_request(messages)
}

pub fn from_{provider}_response(response: {Provider}Response) -> TranslationResult<Vec<SimpleMessage>> {
    {Provider}Translator::from_provider_response(response)
}
```

### 6. Update translator module

**src/translators/mod.rs**:
```rust
#[cfg(feature = "{provider}")]
pub mod {provider};

// Re-export convenience functions
#[cfg(feature = "{provider}")]
pub use {provider}::{from_{provider}_response, to_{provider}_format};
```

### 7. Type design guidelines

**Message structure**:
- Use `Vec<ContentBlock>` pattern for multi-modal content
- Support text, images, tool calls as separate enum variants
- Follow provider's exact field names and casing

**Serde configuration**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "camelCase")]  // Match provider API casing
pub struct {Provider}Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}
```

**Handle serde_json::Value for TypeScript**:
```rust
// For unknown/flexible JSON structures
#[ts(type = "any")]
pub field: serde_json::Value,

// For fields that shouldn't appear in TypeScript
#[ts(skip)]
pub internal_field: InternalType,
```

### 8. Testing and validation

1. **Compile test**: `cargo check --features="{provider}"`
2. **Isolation test**: `cargo check --no-default-features --features="{provider}"`
3. **Integration test**: Create simple translation examples
4. **TypeScript generation**: Verify TS types are generated correctly

### 9. Documentation

Update README.md:
- Add provider to feature flags section
- Update architecture diagram
- Add usage examples

### 10. Common patterns by provider type

**OpenAPI-based providers** (OpenAI, Anthropic):
- Can use automated generation from specs
- Usually have consistent REST API patterns
- Focus on chat completion endpoints

**SDK-based providers** (Bedrock, Google):
- May need to work with existing SDKs
- Handle SDK type conversion carefully
- Consider optional dependencies for large SDKs

**Custom API providers**:
- Manual type extraction from documentation
- Focus on core chat/completion functionality
- Implement streaming support if available

### 11. Best practices

- **Start minimal**: Implement basic text chat first, add features incrementally
- **Follow existing patterns**: Study OpenAI and Bedrock implementations
- **Test thoroughly**: Verify type compatibility and serialization
- **Document differences**: Note any provider-specific quirks or limitations
- **Consider streaming**: Many providers support streaming responses

This process ensures consistent provider integration while maintaining type safety and zero-runtime overhead.