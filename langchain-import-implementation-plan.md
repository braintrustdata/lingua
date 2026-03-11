# LangChain import implementation plan

## Scope

Implement LangChain span-wrapper import support for `import_messages_from_spans` in Lingua.

This work targets third-party trace shapes (LangChain Python/JS wrappers), not provider request/response validation APIs.

## Goals

- Parse LangChain input/output shapes into universal `Message` values.
- Keep typed boundaries (no semantic branching from raw map access).
- Make new `payloads/import-cases/langchain-*` fixtures pass.
- Keep existing OpenAI/Anthropic/Google/Bedrock import behavior stable.

## Non-goals

- Recreate frontend metadata normalization behavior (`model`, `max_tokens`, tool definitions, etc.).
- Add LangChain request/response validators in `validation/*`.
- Add provider wire-format support under `providers/*` for LangChain.

## Why this path

`import_messages_from_spans` is the right home because LangChain data here is a tracing wrapper format, not a provider API format. Existing provider converters should remain focused on canonical provider schemas.

## Proposed file layout

- Add: `crates/lingua/src/processing/import/langchain.rs`
- Update: `crates/lingua/src/processing/import.rs`
  - add `mod langchain;`
  - call `try_parse_langchain_for_import` in import parser flow
- Optional helper module split if needed:
  - `crates/lingua/src/processing/import/langchain_types.rs`
  - `crates/lingua/src/processing/import/langchain_convert.rs`

## Parser integration order

In `try_converting_to_messages`:

1. existing role-array fast path
2. existing provider parser path
3. **new LangChain parser path**
4. existing lenient parsing path
5. existing choices-array path

Rationale: LangChain wrappers can be mistaken for generic/lenient shapes; parse them before lenient fallback.

## Typed compatibility model

Define typed compatibility enums/structs for the key wrappers:

- LangChain message variants:
  - Python-like message object: `{ type, content, ... }`
  - JS `lc/id/kwargs` object
  - chunk message types (`HumanMessageChunk`, `AIMessageChunk`, etc.)
- Input wrappers:
  - nested array format `[[...messages]]`
  - object wrapper `{ messages: [...] }`
- Output wrappers:
  - `LLMResult` style `{ generations: [[{ message, generation_info, ... }]] }`
  - object wrapper `{ messages: [...] }`
  - single standalone message object (tool output spans)
- Tool call compatibility:
  - OpenAI-like tool call shape (`function.arguments` may be object/string)
  - LangChain shape (`args`, `name`, `id`, `type: "tool_call"`)

Use serde aliases and tagged/untagged enums where required.

## Mapping policy (first pass)

- Role mapping:
  - `human` / `HumanMessageChunk` -> `user`
  - `ai` / `AIMessageChunk` -> `assistant`
  - `system` / `SystemMessageChunk` -> `system`
  - `tool` / `ToolMessageChunk` -> `tool`
  - `function` / `FunctionMessageChunk` -> treated consistently with current universal support (likely assistant/tool-result depending content shape)
- Assistant with tool calls:
  - preserve tool call ids and names
  - normalize arguments:
    - object -> `ToolCallArguments::Valid`
    - string/other -> `ToolCallArguments::Invalid(...)`
- Tool messages:
  - map `id` or `tool_call_id` compatibly to universal tool result part `tool_call_id`
  - keep tool name when available
- Content:
  - text string -> string content
  - number content in tool results -> preserve as JSON number in tool result output
  - multimodal arrays:
    - preserve known text/image parts where representable in universal structures
    - anthropic-style image source shape should convert to the same canonical image part shape used elsewhere in universal content

## Implementation phases

1. Add typed LangChain compatibility structs/enums and parser skeleton.
2. Support basic message extraction:
   - human/ai/system
   - `generations` output
   - `{ messages: [...] }` wrappers
3. Add tool-call and tool-result handling:
   - LangChain `tool_call` shape
   - OpenAI-like `function.arguments` object/string normalization
4. Add JS `lc/id/kwargs` message compatibility.
5. Add chunk-type mapping (`*MessageChunk`).
6. Add multimodal image/text array handling.
7. Hardening:
   - ensure non-message input is preserved as no-op for input side when unparseable
   - avoid false positives versus provider parsers

## Testing plan

Primary acceptance:

- `CASE_FILTER=langchain cargo test -p lingua --test import_fixtures -- --nocapture`

Then full fixture confidence:

- `cargo test -p lingua --test import_fixtures -- --nocapture`

Optional targeted unit tests to add in Rust:

- parser detects and parses `generations` shape
- parser handles JS wrapper `lc/id/kwargs`
- parser normalizes LangChain `tool_call` args object/string
- parser handles anthropic-style image part conversion

## Current fixture targets

Recently added import cases under `payloads/import-cases/langchain-*` define expected behavior for:

- mixed system/human/ai input
- streaming/chunk outputs
- tool call normalization
- tool message id/tool_call_id handling
- output `{ messages: [...] }` tool loop
- multiple generations in one batch
- multimodal image inputs (OpenAI-style and Anthropic-style)
- LangChain JS tool message output with non-message input

## Open questions to resolve before finalizing behavior

1. Function-role LangChain messages: should they map to assistant tool calls, tool results, or be dropped when ambiguous?
2. Image part canonicalization: exact universal representation target for Anthropic-style image source payloads.
3. Numeric tool content handling: whether preserving numeric JSON value vs stringifying is preferred in universal import.
4. Strictness policy: return no messages on partially malformed wrappers vs best-effort partial extraction.

## Risks

- Overly broad LangChain detection can steal inputs meant for provider parsers or lenient parser.
- Insufficient typed modeling can regress into ad-hoc JSON branching (forbidden by project guardrails).
- Wrapper variants in the wild may require iterative expansion after first fixture pass.

## Suggested first implementation diff

- Add parser module and wire it into `processing/import.rs`.
- Make the smallest set of new `langchain-*` fixtures pass in this order:
  1. basic wrapper + generations
  2. chunk role mapping
  3. tool calls / tool results
  4. image content shapes
  5. JS wrapper

This sequence reduces blast radius and makes regressions easier to pinpoint.
