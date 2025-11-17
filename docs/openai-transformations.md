<!-- Tracking document for OpenAI payload transformations migrated from proxy.ts -->
# OpenAI payload transformations

## Status legend
- Not started – logic still lives exclusively in `proxy.ts`
- In progress – partially ported to Lingua
- Complete – implemented in Lingua with tests
- N/A – not applicable to Lingua (documented for completeness)

## Request transformation matrix

| Case | `proxy.ts` reference | Behavior | Lingua owner | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| Vertex routing & model rewrite | `fetchOpenAI` 1871-1898 | Rewrite Vertex URLs and strip `publishers/*/models/` prefixes based on endpoint | `transformations::apply_vertex_routing` | Not started | Requires model spec metadata and stream flag awareness |
| Azure deployment URL & gpt-35 rename | `fetchOpenAI` 1917-1935 | Resolve deployment base URL, rename `gpt-3.5` → `gpt-35` when deriving deployment ID | `transformations::apply_azure_routing` | Not started | Needs to honor `no_named_deployment` flag |
| Lepton base URL templating | `fetchOpenAI` 1936-1938 | Replace `<model>` placeholder in base URL with request model | `transformations::apply_lepton_routing` | Not started | Confirm whether lingua should expose provider templating |
| Remove `stream_options` for unsupported providers | `fetchOpenAI` 1975-1981 | Drop `stream_options` for Mistral, Fireworks, Databricks | `transformations::sanitize_stream_options` | Not started | Prevents provider errors |
| Remove `parallel_tool_calls` for unsupported providers | `fetchOpenAI` 1983-1990 | Drop parallel tool calls for Mistral, Databricks, Azure | `transformations::sanitize_parallel_tool_calls` | Not started | Keeps compatibility with provider limits |
| Delete `seed` in Azure Entra flow | `fetchOpenAI` 1995-1999 | Remove unsupported `seed` when Azure API version is set | `transformations::sanitize_seed_for_azure` | Not started | Required for Azure REST validation |
| Reasoning model token handling | `fetchOpenAI` 2015-2024 | Map `max_tokens` → `max_completion_tokens` for reasoning models | `transformations::OpenAIRequestTransformer::apply_reasoning_model_transforms` | Complete | Applies when `modelProviderHasReasoning` matches |
| Reasoning model temperature & tool cleanup | `fetchOpenAI` 2026-2028 | Drop `temperature` and `parallel_tool_calls` for reasoning models | `transformations::OpenAIRequestTransformer::apply_reasoning_model_transforms` | Complete | Avoids unsupported fields on o-series |
| Old o1 system message downgrade | `fetchOpenAI` 2029-2039 | Convert `system` roles to `user` for legacy o1 models | `transformations::OpenAIRequestTransformer::apply_reasoning_model_transforms` | Complete | Scoped to `o1-preview`, `o1-mini`, `o1-preview-2024-09-12` |
| Message content normalization | `fetchOpenAI` 2043-2045` + `providers/openai.ts` | Normalize multimodal content, strip `reasoning` field | `transformations::normalize_messages` | Complete | Remote media conversion deferred; base64 handling implemented |
| Force fake streaming when provider lacks streaming | `fetchOpenAI` 2047-2055 | Route through fake stream helper when `supportsStreaming === false` | `transformations::apply_stream_fallback` | Not started | Should remain request-side decision |
| Responses API auto-pivot for pro models | `fetchOpenAI` 2058-2068 | Send specific pro models via Responses API | `transformations::route_to_responses_api` | Not started | Applies to `o1-pro`, `o3-pro`, `gpt-5-pro`, `gpt-5-codex` |
| Remove explicit `text` response_format | `fetchOpenAI` 2076-2081 | Drop redundant `response_format: { type: \"text\" }` | `transformations::apply_response_format_transforms` | Complete | Avoids Together failures |
| Managed structured output tooling | `fetchOpenAI` 2082-2110` | Convert JSON schema response_format into tool call | `transformations::apply_response_format_transforms` | Complete | Enforces no tools + json schema conflict |
| Structured output stream rewrite | `fetchOpenAI` 2134-2178 | Rewrites streaming & non-streaming responses to expose JSON content | `responses::structured_output_stream` | Not started | Response-path transformation—documented for parity |

## Tracking notes
- Focus initial implementation on request-side transformations (rows marked with Lingua owner in `transformations::*` and `message_normalization::*`).
- Response-path adjustments are listed for awareness; Lingua may handle them in a dedicated response module once request transformations are complete.
- Update the Status column and Lingua owner references as modules are implemented.


