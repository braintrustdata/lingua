# Responses multi-message conversion plan

## Root cause

Responses can emit multiple assistant `message` output items in one response. The streaming adapter maps every `response.output_text.delta` to Chat Completions choice 0, without retaining the item boundary. The full-response path also merges adjacent Responses assistant messages. Two JSON messages therefore become `{...}{...}`. A Chat Completions answer has no faithful representation for two sequential assistant messages, and selecting one would discard provider output.

## Target files

- `payloads/cases/params.ts`: add a GPT-5.4 strict JSON schema request to exercise the routed request path. A live capture may emit only one message, so the deterministic regression uses synthetic response events.
- `payloads/snapshots/`, `payloads/transforms/`, and Vitest snapshots: capture real provider responses and transformed responses for the new case.
- `payloads/transforms/transform_errors.json`: classify the existing real Responses capture with separate commentary and final-answer message items as unsupported when converting back to Chat Completions.
- `payloads/scripts/transforms/transforms-chat-completions.test.ts`: skip SDK parsing for cases already classified as expected transform errors.
- `crates/lingua/src/providers/openai/responses_adapter.rs`: parse `response.output_item.added` with a typed view to identify assistant message items.
- `crates/lingua/src/processing/transform.rs`: carry the typed message-item index through stream transformation, and reject full Responses payloads with multiple text-bearing assistant messages when converting to Chat Completions or Google.
- `crates/lingua/src/processing/stream.rs`: track the first Responses message item per session and reject a second one before emitting its text.

The separate TypeScript proxy converter in the Braintrust monorepo has the same concatenation behavior. It is outside this Lingua checkout and needs a matching change in that repository.

## Expected behavior

- One message, with or without a preceding reasoning or tool item, converts as before.
- A second Responses assistant message produces a clear unsupported-mapping error. Its text is never appended to the first Chat Completions answer.
- Same-format Responses passthrough preserves all output items.
- Full-response fallback and direct response conversion reject the same ambiguous multi-message case.

## Tests

- Streaming session regression with two identical JSON message items, both with and without `phase`, plus a single-message control and reasoning-first case.
- Full-response conversion test for two message items and a reasoning-plus-one-message control.
- Assert a stable unsupported-mapping error category and the Responses-to-target context.
- Capture a real GPT-5.4 strict JSON schema request and its transformed provider responses in a provisioned environment, then put those artifacts in the same PR as the converter fix.

## Expected-diff impact

- Existing one-message responses should remain unchanged. The new payload case adds two provider snapshot sets, seven transform captures, and Vitest snapshots. One narrow expected-unsupported entry is needed for an existing real capture that contains both commentary and final-answer message items.

## Validation sequence

1. `make capture FILTER=responsesRoutedStrictJsonSchemaParam` in the provisioned capture environment.
2. `cargo test -p lingua <focused test filter>` with Rust 1.97.1.
3. Re-run `make capture FILTER=responsesRoutedStrictJsonSchemaParam`.
4. `make test-payloads`.
5. If logic changes stale artifacts, `make regenerate-failed-transforms`.
6. `cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures`.
7. `make typed-boundary-check` and `make typed-boundary-check-branch BASE=main`.

The local ARM environment lacks compatible capture tooling and provider credentials. Run live captures in the provisioned Linux environment; do not manufacture or hand-edit captured payloads.
