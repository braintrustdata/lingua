# Build guide

This document describes how to build Lingua and generate language bindings.

## Prerequisites

- Rust 1.70+ with Cargo
- Node.js 20+ with pnpm (for TypeScript bindings)
- Python 3.11+ with uv (for Python bindings)
- Protocol Buffers compiler (for Google provider)

## Quick start

```bash
# Install git hooks (recommended for development)
make install-hooks

# Build all bindings
make all

# Run tests
make test
```

## Development setup

After cloning the repository, install git hooks to ensure code quality:

```bash
make install-hooks
```

This installs a pre-commit hook that:
- Automatically runs `cargo fmt` to format Rust code
- Runs TypeScript linting/formatting checks (if applicable)
- Aborts the commit if any files are changed by formatting

If the commit is aborted, review the formatting changes and re-commit:

```bash
git add -A
git commit
```

To temporarily bypass the hook (not recommended):

```bash
git commit --no-verify
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

TypeScript types for the universal `Message` format are automatically generated from Rust types using [ts-rs](https://github.com/Aleph-Alpha/ts-rs):

```bash
# Generate TypeScript types from Rust
make generate-types
```

This creates TypeScript files in `bindings/typescript/src/generated/`:

- Individual type files (e.g., `Message.ts`, `UserContentPart.ts`)
- Automatic exports for all types marked with `#[ts(export)]`

**Important**: After modifying Rust types in `src/universal/`, always run `make generate-types` and commit the updated TypeScript files. CI will verify that generated types are up to date.

To build the full TypeScript bindings (WASM):

```bash
make typescript
```

### Browser trace entry point

The optional `@braintrust/lingua/browser/trace` entry point includes message
conversion for all providers, message imports and deduplication, span imports,
and trajectory streaming and collection. It uses a smaller WASM artifact that
omits request/response validation and request/response/stream transformation.
The existing Node and browser entry points retain the full API.

`make lingua-wasm` builds both variants with the release profile (`opt-level = 3`).
To build only the scoped browser artifact:

```bash
pnpm --dir bindings/lingua-wasm run build:web:trace
```

Serve `@braintrust/lingua-wasm/browser/trace/lingua_bg.wasm` from your app's
static assets, then initialize the matching entry point:

```typescript
import { init, importMessagesFromSpans, getWasm } from "@braintrust/lingua/browser/trace";

await init("/wasm/lingua-trace.wasm");
const messages = importMessagesFromSpans(spans);
```

`getWasm()` exposes `import_span`, `trajectory_metadata_fields`,
`TrajectoryStream`, and `TrajectoryCollector`. The scoped entry point has its
own runtime, so it can coexist with the full entry points. Raw browser bindings
are available at `@braintrust/lingua-wasm/browser/trace`. Direct Rust builds use
`--features bedrock,wasm-trace`; this limits WASM exports without changing the
native Rust API.

## Provider type generation

Generate provider-specific types from OpenAPI specs:

```bash
cargo run --bin generate-types openai
cargo run --bin generate-types anthropic
```

## Testing

Run all tests (Rust, TypeScript, and Python):

```bash
make test
```

Run tests for specific languages:

```bash
make test-rust         # Rust tests only
make test-typescript   # TypeScript tests only
make test-python       # Python tests only
```

Run tests for specific features:

```bash
cargo test --features="openai"
```

## Code quality

Format code:

```bash
make fmt              # Format all code (Rust + TypeScript)
cargo fmt             # Format Rust code only
```

Run linter:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Check all code compiles:

```bash
make check            # Check Rust and TypeScript
```

## Available features

- `openai` - OpenAI API types and translators
- `anthropic` - Anthropic API types and translators
- `google` - Google Gemini API types and translators
- `bedrock` - AWS Bedrock API types and translators

Default features: `["openai", "anthropic", "google", "bedrock"]`
