# Route GPT requests through Responses

- Root cause: Lingua upgrades Chat Completions only when `reasoning_effort` and tools are combined. The router separately recognizes only selected GPT versions as requiring Responses, so ordinary GPT requests can still use Chat Completions.
- Target files: `crates/lingua/src/processing/transform.rs`, `crates/braintrust-llm-router/src/catalog/spec.rs`, router tests, and `payloads/cases/params.ts`.
- Expected behavior: The router sends GPT models to Responses regardless of request parameters or model version when the provider supports that transport. Direct Lingua transformations honor their explicit target format.
- Tests: The existing `reasoningEffortNoneParam` payload (GPT with no tools), focused Lingua and router tests for plain GPT requests, versions, and non-GPT targets, plus payload transform checks.
- Expected diff: Remove parameter inspection and GPT version parsing; update related tests and any capture artifacts only after logic is fixed.
- Validation: `make capture FILTER=reasoningEffortNoneParam`; focused Rust tests; repeat capture; `make test-payloads`; regenerate failed transforms only if stale; coverage cross-provider test; typed-boundary checks.
