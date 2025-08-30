# Provider type generation pipeline

This document outlines the process for keeping LLMIR's provider types in sync with the latest provider SDKs. This pipeline will eventually cover all providers (OpenAI, Anthropic, Google, etc.), but starts with OpenAI as the reference implementation.

## Summary

**Automated generation**: Not feasible with current tooling  
**Recommended approach**: Manual conversion with structured pipeline  
**Update frequency**: Check monthly or when OpenAI releases major SDK updates  

## Pipeline overview

```
OpenAI SDK Update → Type Extraction → Rust Type Generation → Validation → Integration
```

## Step-by-step process

### 1. Check for provider SDK updates

**OpenAI Location**: `/tests/typescript/openai/`  
**Future providers**: `/tests/typescript/anthropic/`, `/tests/typescript/google/`, etc.

```bash
cd tests/typescript/openai/
npm outdated openai
# If outdated, update:
pnpm add -D openai@latest
```

**What to check**:
- Current version vs latest version
- Release notes for breaking changes
- New features in chat completions API

### 2. Extract type definitions

**Method**: Manual inspection of TypeScript interfaces

**Key files to examine** (in `node_modules/openai/`):
- `resources/chat/completions.d.ts` - Main exports
- `src/resources/chat/completions/` - Core type definitions  
- Look for interfaces prefixed with `ChatCompletion`

**Target interfaces**:
- `ChatCompletionCreateParams` - Request parameters
- `ChatCompletionCreateParamsNonStreaming` - Non-streaming request  
- `ChatCompletion` - Response interface
- `ChatCompletionMessage` - Individual messages
- `ChatCompletionChoice` - Response choices
- `CompletionUsage` - Token usage info

### 3. Convert TypeScript to Rust

**Manual conversion rules**:

| TypeScript | Rust |
|------------|------|
| `string` | `String` |
| `number` | `u64` (for counts), `f64` (for floats) |
| `boolean` | `bool` |
| `T \| undefined` | `Option<T>` |
| `T \| U` | `enum` or separate structs |
| `Array<T>` | `Vec<T>` |
| `object` | `struct` with `serde_json::Value` for unknown |

**Process**:
1. Create new files in `src/providers/{provider}/` (e.g., `openai/`, `anthropic/`)
2. Define request types in `request.rs`  
3. Define response types in `response.rs`
4. Update `mod.rs` to export both
5. Add `#[derive(Serialize, Deserialize)]` to all types
6. Use `#[serde(rename_all = "snake_case")]` where needed

### 4. Validation process

**Step 4a: Type Compatibility Test**
```bash
cd tests/typescript/{provider}/  # e.g., openai/, anthropic/
pnpm add -D @types/node typescript
# Create test that imports both LLMIR types and provider SDK types
# Verify fields match between Rust-generated types and provider SDK
```

**Step 4b: Round-trip Testing**
```rust
// In tests/
#[test]
fn test_{provider}_request_roundtrip() {
    // Create provider request using LLMIR types
    // Serialize to JSON
    // Deserialize using provider SDK types (via Node.js)
    // Verify no data loss
}
```

**Step 4c: Real API Testing**
```rust 
// Integration test with actual provider API
// Send request using LLMIR types
// Verify response can be parsed into LLMIR response types
```

### 5. Integration steps

**Update translators**:
- `src/translators/{provider}.rs` - Update to use new request/response types
- Ensure bidirectional conversion still works
- Update any field mappings that may have changed

**Update examples**:
- `examples/simple_{provider}.rs` - Test with new types
- Verify TypeScript bindings still generate correctly

## Why not automated?

**Technical challenges**:

1. **Direction Mismatch**: Tools like `ts-rs` generate TypeScript FROM Rust, not the reverse
2. **Type System Differences**: 
   - TypeScript's union types (`string | number`) don't map cleanly to Rust enums
   - Optional properties (`field?: string`) vs Rust's `Option<T>`
   - TypeScript's flexible object types vs Rust's strict structs

3. **Complex OpenAI Types**: 
   - Heavy use of discriminated unions
   - Nested optional properties
   - Function overloads that don't exist in Rust

4. **Maintenance Overhead**: 
   - Custom tooling would need updates as TypeScript/OpenAI evolves
   - Manual conversion gives us control over type design decisions
