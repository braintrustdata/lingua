# Remove display-only Chat Completions conversion

## Root cause

The new display conversion creates a second, lossy public Chat Completions API solely to drop replay signatures. Display policy belongs at the presentation boundary, not in the provider conversion API.

## Target files

- `crates/lingua/src/providers/openai/convert.rs`
- `crates/lingua/src/wasm.rs`
- `bindings/typescript/src/converters.ts`
- `bindings/typescript/src/wasm.ts`
- TypeScript export and type tests

## Expected behavior

The normal Chat Completions conversion remains the only export. Imported Responses reasoning followed by a function call remains covered by the import fixture.

## Expected diff impact

- Remove the display-only Rust, WASM, and TypeScript APIs and their tests.
- Keep the import fixture and add coverage to the normal replay conversion.

## Validation

- `cargo test -p lingua chat_messages_keep_reasoning_signatures_and_tool_calls`
- `pnpm --dir bindings/typescript test -- node-exports.test.ts browser-exports.test.ts`
- `make typed-boundary-check`
