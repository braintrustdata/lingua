# Google provider type follow-up plan

## Root cause

- Google Discovery schema ids can move between public and internal `V1main` names, allowing quicktype collision resolution to silently swap the public `MediaResolution` struct and enum names.
- `Part.toolCall` and `Part.toolResponse` are omitted by the Google content converter. The universal tool-call/result shapes require a function name and have no typed provider-executed builtin identity, so preserving Google’s optional `toolName` and `toolType` is impossible today.
- The Google adapter lifts a small canonical subset of `generationConfig` and discards every other typed field, including `audioTranscriptionConfig` and request-level `mediaResolution`.
- Google streaming conversion only inspects `Part.functionCall`, so native `Part.toolCall` chunks are silently reduced to empty assistant deltas and may receive the wrong finish reason.
- Google declares built-in `ToolCall.id` and `ToolResponse.id` optional, but the universal built-in parts currently require a string and the converter rejects valid provider payloads with no ID.

## Target files

- `crates/generate-types/src/main.rs`
- `crates/lingua/src/universal/message.rs`
- Provider adapters and import helpers that construct or consume universal tool calls/results
- `crates/lingua/src/providers/google/convert.rs`
- `crates/lingua/src/providers/google/adapter.rs`
- `crates/lingua/src/providers/google/params.rs`
- `payloads/cases/advanced.ts`
- `payloads/cases/params.ts`
- Generated TypeScript universal bindings produced by `make generate-types`
- Narrow transform expectations only if an intentional cross-provider limitation remains

## Expected behavior

- `V1main` Discovery ids normalize to stable public names, while the GenerationConfig scalar enum remains `MediaResolutionEnum`; generation fails loudly on real normalized-name collisions.
- Dedicated universal builtin-tool call and result parts carry an optional free-form name plus a typed identity (`provider` and `builtin_type`). Google server-side tool calls/results round-trip with `provider_executed: true` without fabricating a function name, while ordinary function-tool parts remain source-compatible.
- Built-in call/result correlation IDs are optional so a missing Google ID round-trips as absent rather than being rejected or synthesized. Real IDs remain unchanged.
- Native Google streaming `toolCall` parts become typed universal built-in tool-call deltas, set the `tool_calls` finish reason, and round-trip back to Google. Streaming targets that cannot represent the built-in identity fail explicitly instead of treating it as a function call.
- Providers that cannot represent a provider-executed builtin return an explicit unsupported-mapping error instead of silently dropping it.
- Google-to-universal preserves only the unmapped, typed remainder of `generationConfig` in Google-scoped extras. Universal-to-Google starts from that typed remainder and lets canonical fields override it, avoiding duplicate sources of truth.
- The accepted REST `audioTranscriptionConfig` subtree and request-level `mediaResolution` survive Google round trips byte-for-byte at the semantic JSON level.

## Tests to add or update

- Keep the existing generator normalization/collision tests and media-resolution compile-time serialization guards.
- Add Google converter tests for named and unnamed provider-executed builtin calls and responses.
- Add Google converter tests for built-in calls and responses with absent IDs.
- Add Google streaming tests for typed built-in call conversion, absent-ID preservation, Google roundtrip, finish-reason handling, and explicit rejection by non-Google targets.
- Add Google params/adapter tests proving unmapped `generationConfig` fields survive while canonical temperature/reasoning/response-format values take precedence.
- Keep payload cases `googleProviderExecutedToolRoundtrip` and `audioTranscriptionConfigParam`; recapture after the logic fix.
- Update existing universal/provider tests for optional tool names and builtin identities.

## Expected-diff impact

- Google same-provider request coverage should stop reporting loss of `generationConfig.audioTranscriptionConfig` and `generationConfig.mediaResolution`.
- Google provider-executed tool call/response parts should stop disappearing. Cross-provider transforms may produce explicit unsupported errors where no equivalent builtin exists; any expectation entry must be case-specific.
- Generated Google provider files should change only through the generator. Generated universal TypeScript bindings will reflect optional tool names and builtin identity fields.

## Validation commands

1. `make capture FILTER=audioTranscriptionConfigParam`
2. `make capture FILTER=googleProviderExecutedToolRoundtrip`
3. `cargo test -p generate-types google_post_process_tests`
4. `cargo test -p generate-types google_schema_name_tests`
5. `cargo test -p lingua providers::google::convert::tests`
6. `cargo test -p lingua providers::google::params::tests`
7. `cargo test -p lingua providers::google::`
8. `make capture FILTER=audioTranscriptionConfigParam`
9. `make capture FILTER=googleProviderExecutedToolRoundtrip`
10. `make test-payloads`
11. `cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures`
12. `make typed-boundary-check`
13. `make typed-boundary-check-branch BASE=main`
14. `make generate-types PROVIDER=google`
15. `git diff --exit-code crates/lingua/src/providers/google/generated.rs bindings/typescript/src/generated/google`
16. `cargo check -p lingua`
17. `cd bindings/typescript && pnpm run typecheck`
