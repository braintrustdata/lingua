# Build guide

This document describes how to build Elmir and generate language bindings.

## Prerequisites

- Rust 1.70+ with Cargo
- For development: Git hooks (optional but recommended)

## Quick start

```bash
# Build the library
cargo build

# Generate TypeScript bindings
cargo run --bin ts-gen

# Run tests
cargo test
```

## Development setup

Install git hooks for consistent formatting:

```bash
./scripts/install-hooks.sh
```

## Building the library

Build in development mode:
```bash
cargo build
```

Build optimized release version:
```bash
cargo build --release
```

Build with specific provider features:
```bash
cargo build --no-default-features --features="openai,anthropic"
```

## Generating bindings

### TypeScript bindings

Generate TypeScript type definitions:
```bash
cargo run --bin ts-gen
```

This creates TypeScript files in `bindings/typescript/`:
- Individual type files (e.g., `Message.ts`, `UserContentPart.ts`)
- `index.ts` - exports all types for easy importing

## Provider type generation

Generate provider-specific types from OpenAPI specs:
```bash
cargo run --bin generate-types openai
cargo run --bin generate-types anthropic
```

## Testing

Run all tests:
```bash
cargo test
```

Run tests for specific features:
```bash
cargo test --features="openai"
```

## Code quality

Format code:
```bash
cargo fmt
```

Run linter:
```bash
cargo clippy
```

## Available features

- `openai` - OpenAI API types and translators
- `anthropic` - Anthropic API types and translators  
- `google` - Google Gemini API types and translators
- `bedrock` - AWS Bedrock API types and translators

Default features: `["openai", "anthropic", "google", "bedrock"]`