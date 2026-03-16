# Converter test porting todo

This tracks porting from `app/ui/trace/converters/*.test.ts` into `payloads/import-cases`.

Notes:

- `import-cases` only exercises `import_messages_from_spans`, so detector tests and metadata-transform-only tests are not directly portable.
- When a fixture expectation differs from the old converter tests, the corresponding `*.assertions.json` includes `_migrationNote`.

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
  - system/human/ai mixed inputs
  - message chunk role mapping (`SystemMessageChunk` / `HumanMessageChunk` / `AIMessageChunk`)
  - streaming output chunks
  - tool_call transformations (`args/name/id/type: tool_call` -> function tool call shape)
  - tool message `id`/`tool_call_id` handling
  - output `{ messages: [...] }` tool loop wrappers
  - multimodal image_url content
  - anthropic-style image content conversion target behavior
  - multiple generations in a single batch
  - js `ToolMessage` output with non-message input
- `pydantic-ai-converter.test.ts`
  - basic wrapper `user_prompt` + `response.parts`
- `ai-sdk-converter.test.ts`
  - OpenAI Responses `steps` wrapper
  - legacy AI SDK `messages` + output object

## Still to port (maximalist backlog)

- `ai-sdk-converter.test.ts` (many scenarios; highest volume)
  - v3/v4 output shapes
  - streaming/steps variants
  - tool call/result extraction across steps
  - attachments (image/document)
  - reasoning/thinking variants
  - doGenerate/doStream/provider-level formats
  - streamObject/object output variants
- `langchain-converter.test.ts` (remaining non-portable unit tests)
  - `isLangChainSpan` detection/rejection cases
  - metadata extraction variations (`model`, `provider`, `max_tokens`, `tools`, etc.)
  - `toolDefinitions` map assertions
  - prompt schema (`promptDataSchema`) validation-specific assertions
  - invalid output passthrough behavior assertions
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
- `anthropic-tools-converter.test.ts` (remaining non-portable unit tests)
  - tool metadata detection/transformation assertions
- `mastra-response-converter.test.ts` (remaining non-portable)
  - `isMastraSpan` detector assertions
