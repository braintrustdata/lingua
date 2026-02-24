# Converter test porting todo

This tracks porting from `app/ui/trace/converters/*.test.ts` into `payloads/import-cases`.

Status snapshot (February 24, 2026):

- OpenAI Responses importer coverage in `payloads/import-cases` is now broad enough to cover the main import compatibility shims used by `import_messages_from_spans`.
- OpenAI Responses import-only compat/normalization logic now lives in `crates/lingua/src/processing/import_openai_responses.rs` (not `providers/openai/convert.rs`).
- Generic OpenAI Responses converters in `crates/lingua/src/providers/openai/convert.rs` remain roundtrip-safe (standalone reasoning items are preserved there).
- Recent validation used both importer fixtures and provider roundtrip tests:
- `cargo test -p lingua --test import_fixtures`
- `cargo test -p lingua test_responses --lib`

Notes:

- `import-cases` only exercises `import_messages_from_spans`, so detector tests and metadata-transform-only tests are not directly portable.
- When a fixture expectation differs from the old converter tests, the corresponding `*.assertions.json` includes `_migrationNote`.
- Import-specific OpenAI Responses behavior (compat aliases/coercions/reasoning merge) should be tested via `payloads/import-cases`, not by changing generic provider roundtrip behavior.

## Ported (high confidence / direct importer coverage)

- `lingua-converter.test.ts`
  - simple chat messages
  - tool calls / tool results
  - developer role variants
  - multi-message conversations
  - anthropic-style content block output
  - try prompt (input-only)
- `anthropic-tools-converter.test.ts`
  - tool_use blocks
  - tool_result blocks
  - multiple tool_use blocks
  - plus existing anthropic fixtures already in this folder
- `openai-response-converter.test.ts`
  - mixed responses ordering (input/output)
  - function_call_output input case
  - real-world tool loop
  - image attachments / image generation / web search / reasoning blocks / reasoning-only output
  - input_text/output_text message arrays
  - wrapped responses under `output` field (`responses-output-field`)
- `mastra-response-converter.test.ts`
  - llm_generation conversation/tool loop
  - legacy tool message
  - tool_call span
  - agent_run variants

## Ported (representative unsupported/raw wrapper coverage)

- `gemini-converter.test.ts`
  - basic raw `contents/parts` request shape
- `adk-converter.test.ts`
  - basic raw ADK input/output shape
- `langchain-converter.test.ts`
  - basic human/ai wrapper + `generations`
- `pydantic-ai-converter.test.ts`
  - basic wrapper `user_prompt` + `response.parts`
- `ai-sdk-converter.test.ts`
  - OpenAI Responses `steps` wrapper
  - legacy AI SDK `messages` + output object
- `anthropic-tools-converter.test.ts` + adjacent importer coverage
  - role-based `system`/`developer` import fixture (`anthropic_system`)

## Still to port (maximalist backlog)

- `ai-sdk-converter.test.ts` (many scenarios; highest volume)
  - v3/v4 output shapes
  - streaming/steps variants
  - tool call/result extraction across steps
  - attachments (image/document)
  - reasoning/thinking variants
  - doGenerate/doStream/provider-level formats
  - streamObject/object output variants
- `langchain-converter.test.ts` (many scenarios)
  - tool_call transformations
  - tool messages/tool_call_id
  - metadata extraction variations
  - multimodal image content
  - Anthropic image conversions
  - batch/multiple generation shapes
- `pydantic-ai-converter.test.ts` (many scenarios)
  - message_history and internal message formats
  - tool calls/returns and grouping
  - multipart image/document attachments
  - toolset/tool definition extraction
  - reasoning/thinking parts
- `gemini-converter.test.ts` (more scenarios)
  - thinking tokens
  - image inputs
  - function calls and snake_case variants
- `adk-converter.test.ts` (more scenarios)
  - function calls and snake_case variants
  - error responses and finishReason/usageMetadata edge cases
  - Go library PascalCase format
- `openai-response-converter.test.ts` (remaining non-portable unit tests)
  - `isOpenAIResponse` detection/rejection cases
  - `transformMetadataForChatCompletions`
  - metadata-driven system message / response-format transformation assertions
  - converter-only roundtrip assertions that intentionally depend on provider converter behavior (distinct from importer fixture expectations)
- `anthropic-tools-converter.test.ts` (remaining non-portable unit tests)
  - tool metadata detection/transformation assertions
- `mastra-response-converter.test.ts` (remaining non-portable)
  - `isMastraSpan` detector assertions
