# Anthropic Opus 5 compatibility plan

## Root cause

- Anthropic now uses the fixed model ID `claude-opus-5`, but the Opus 4.7-or-later capability regex only recognizes Opus 5 IDs with a minor-version suffix such as `claude-opus-5-0` or `claude-opus-5.0`.
- As a result, universal reasoning effort converts to legacy manual thinking for `claude-opus-5` instead of adaptive thinking with `output_config.effort`.
- Claude Opus 5 rejects `thinking: {type: "disabled"}` when effort is `xhigh` or `max`; the universal-to-Anthropic adapter does not currently validate that cross-field combination.

## Target files

- `crates/lingua/src/providers/anthropic/capabilities.rs`
- `crates/lingua/src/providers/anthropic/adapter.rs`
- `payloads/cases/params.ts`
- Generated capture and transform snapshots for `anthropicOpus5AdaptiveThinkingMaxEffortParam`
- Regenerated Anthropic specification and generated types already produced by the provider pipeline

## Expected behavior

- Recognize direct, Bedrock, and Vertex-style `claude-opus-5` model IDs as adaptive-thinking models that support `output_config.effort` and reject deprecated sampling parameters.
- Convert universal `max` effort to `thinking: {type: "adaptive"}` plus `output_config: {effort: "max"}` for Claude Opus 5.
- Preserve explicit thinking opt-out for effort `high` or below.
- Return a clear conversion error for Claude Opus 5 when thinking is disabled with effort `xhigh` or `max` instead of emitting a request Anthropic rejects.
- Leave Fable 5 and Mythos 5 always-on-thinking behavior unchanged.

## Tests

- Extend Anthropic capability tests for the fixed Claude Opus 5 model ID and provider-wrapped forms.
- Add adapter tests for Opus 5 adaptive/max conversion, default thinking omission, valid disabled/high conversion, and invalid disabled/`xhigh|max` errors.
- Re-capture `anthropicOpus5AdaptiveThinkingMaxEffortParam` and verify the request snapshot changes from legacy manual thinking to adaptive thinking plus max effort.
- Run payload, cross-provider, and typed-boundary checks.

## Expected diff impact

- Anthropic generated types add the new refusal category and context-window stop reason from the current OpenAPI specification.
- The targeted Responses-to-Anthropic transform snapshot changes intentionally for Claude Opus 5 reasoning.
- No broad expected-difference exceptions should be needed.

## Validation commands

```bash
make capture FILTER=anthropicOpus5AdaptiveThinkingMaxEffortParam
cargo test -p lingua providers::anthropic::
make capture FILTER=anthropicOpus5AdaptiveThinkingMaxEffortParam
make test-payloads
make regenerate-failed-transforms
cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures
make typed-boundary-check
make typed-boundary-check-branch BASE=main
```
