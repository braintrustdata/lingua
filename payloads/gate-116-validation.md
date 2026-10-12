# Gemini lowercase thinking level validation

Validated on 2026-10-12 for [GATE-116](https://linear.app/braintrustdata/issue/GATE-116/lingua-native-gemini-lowercase-thinkinglevel-rejected-with-source) and [PR #488](https://github.com/braintrustdata/lingua/pull/488), starting from commit `ea8c76d133d3e5e7beb41795b51194925ffad7ad`.

## Capture results

- Added `thinkingLevelLowercase{Low,Medium,High,Minimal}Param` cases using `gemini-3-flash-preview` and the prompt `Say ok.`.
- Google accepted all four lowercase values for non-streaming and streaming requests, including follow-up turns. Saved 24 native fixture files. Native capture durations were 2,256–4,518 ms per case, including those calls.
- Captured all 12 Google → Anthropic, Chat Completions, and Responses transformations with no capture failures.
- Generated 24 request/response Vitest snapshots for those transformations.
- The fixture request type models lowercase REST enum aliases alongside the SDK's uppercase enum values.

## Validation results

| Command | Result |
| --- | --- |
| `pnpm --dir payloads capture --filter thinkingLevelLowercase --snapshots-only` | Four native captures succeeded |
| `make capture FILTER=thinkingLevelLowercase` | 12 transformations captured; subsequent run found all captures present |
| `cargo test -p lingua providers::google` | 206 passed |
| `make test-payloads` | 1,879 passed, 106 skipped; 13 test files passed |
| `pnpm --dir payloads exec vitest run scripts/transforms/google-thinking-level.test.ts scripts/sync.test.ts` | 882 passed, 74 skipped |
| `pnpm --dir payloads typecheck` | Passed |
| Prettier and oxlint on changed TypeScript files | Passed |
| `make typed-boundary-check` | Passed |
| `make typed-boundary-check-branch BASE=main` | Passed |
| `git diff --check` | Passed |
| `cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures` | Failed: 24 streaming roundtrip metadata losses, detailed below |

The existing WASM artifact, before rebuilding the PR, rejected all four lowercase values with a source-format detection error: eight regression checks failed and five uppercase/invalid-value checks passed. After rebuilding, all 13 regression checks pass. They exercise native bodies without a model field, preserve native lowercase payloads through the passthrough path, compare lowercase and uppercase reasoning-effort mappings, and reject an unknown value. The pre-rebuild artifact was already on this box; it was not a fresh build of `main`.

The first capture run's automatic snapshot test found that the invalid-value assertion expected an older error message. The assertion now checks the source-format detection category, and the subsequent full payload test run passes.

The box initially had Rust 1.92 and stale JS/WASM dependencies. Validation used Rust 1.95 with the WASM target, dependencies installed from the frozen pnpm lockfile, and rebuilt Node/browser WASM bindings.

## Outstanding coverage findings

The required cross-provider guard reports exactly 24 failures, all involving the four new native Google captures:

- Google → Anthropic: eight streaming roundtrips.
- Google → Bedrock Anthropic: eight streaming roundtrips.
- Google → Vertex Anthropic: eight streaming roundtrips.

Each pair covers the four thinking levels for the initial and follow-up streams. Every failure reports loss of `served_service_tier` on event 0. The captured Google responses include `usageMetadata.serviceTier: "standard"`; the roundtrip through these Anthropic-family stream formats loses that metadata. Request and non-streaming response checks for the guard's required providers pass under the existing expectations.

The broader diagnostic command `cargo run --bin coverage-report -- -t 'thinkingLevelLowercase*'` reports 56/56 request checks, 24/28 response checks, and 32/56 streaming checks passing under existing expectations. In addition to the 24 failures above, four Google → Bedrock Converse response roundtrips change `model` to null and add `usage.prompt_tokens_exclude_cache`; three also lose `usage.completion_reasoning_tokens`. Bedrock Converse is outside the required guard's provider list.

These findings concern response metadata interoperability, rather than rejection of lowercase request values. The captures retain the original fields. No converter changes or new coverage exceptions were made to conceal these failures; the cross-provider guard remains failing on this branch.
