## Top-k OpenAI model regression

Root cause:
- Google GenerateContent requests can carry `generationConfig.topK`, including when routed to an OpenAI model through the gateway.
- OpenAI Chat Completions and Responses do not support `top_k`, so the OpenAI adapters must drop canonical `top_k` when emitting requests.
- Existing `topKParam` covers top-k dropping, but not the reported Google SDK shape with an OpenAI model and `maxOutputTokens`.

Target files:
- `payloads/cases/params.ts`

Expected behavior after fix:
- A Google request using an OpenAI model with `generationConfig.topK` transforms to OpenAI Chat Completions and Responses without any `top_k` field.
- `maxOutputTokens` still maps to the OpenAI output token budget field.

Tests to add/update:
- Add a focused payload case for Google SDK top-k on an OpenAI model.
- Validate with coverage-report and targeted transform tests where local fixtures exist.

Expected-diff impact:
- `params.top_k` remains an expected lost field for OpenAI targets.
- No generated provider files or broad expected-difference changes.

Command sequence to validate:
- `cargo run -p coverage-report -- -f compact -t googleOpenAIModelTopKParam`
- `pnpm --dir payloads exec vitest run scripts/transforms/transforms.test.ts -t "topKParam"`
