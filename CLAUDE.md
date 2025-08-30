# LLMIR project guide for Claude

This guide helps AI assistants understand and work with the LLMIR codebase effectively.

## Project overview

LLMIR (LLM Intermediate Representation) is a universal message format that compiles to provider-specific formats with zero runtime overhead. It's designed to allow seamless interoperability between different LLM providers without runtime penalties.

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

## Future enhancements

- **OpenAPI specification approach**: Generate types from OpenAPI specs rather than SDKs
- **Automated validation**: CI/CD integration for type compatibility checking  
- **Extended provider support**: Anthropic, Google, Cohere, etc.
- **Streaming support**: Handle provider-specific streaming patterns