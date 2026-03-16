# Import fixtures follow-up TODO

## Failing new cases

- [x] openai-responses-function-call-output-input
- [x] openai-responses-image-attachments
- [x] openai-responses-image-generation-call
- [x] openai-responses-mixed-input-order
- [x] openai-responses-mixed-output-order
- [x] openai-responses-real-world-tool-loop
- [x] openai-responses-reasoning-blocks
- [x] openai-responses-reasoning-only-output
- [x] openai-responses-web-search

## Work items

- [x] Loosen importer pre-check for Responses item arrays
  - Current gap: `has_message_structure` rejects arrays that do not have `role` or nested `message.role`.
  - Goal: allow Responses-only arrays (`reasoning`, `function_call_output`, `web_search_call`, etc.) to reach typed OpenAI conversion logic.
  - Acceptance: mixed/output-only Responses fixtures are not dropped at import pre-check.

- [x] Handle raw string span input as user messages
  - Current gap: string-valued `span.input` is ignored.
  - Goal: map string input to `Message::User` with string content.
  - Acceptance: string-input fixtures (for example image generation and web search) include the expected leading user message.

- [x] Expand lenient text-type parsing for content blocks
  - Current gap: lenient parser only accepts `type: "text"`.
  - Goal: also accept OpenAI Responses block types `input_text` and `output_text`.
  - Acceptance: fixtures containing these block types parse into expected user/assistant text messages.

- [x] Add typed compatibility for `callId` aliasing
  - Current gap: some fixtures use `callId` while generated OpenAI types expect `call_id`.
  - Goal: normalize or alias `callId` at import boundary before typed conversion.
  - Acceptance: tool call and tool result linkage is preserved for both `call_id` and `callId`.

- [x] Add typed compatibility for `function_call_result`
  - Current gap: fixtures include `type: "function_call_result"` which is not represented in generated enums.
  - Goal: normalize this to the canonical supported shape before conversion, without raw fallback parsing.
  - Acceptance: output/input-order and tool-loop fixtures parse tool result messages correctly.

- [x] Add typed compatibility for non-string tool output payloads
  - Current gap: fixtures include object-valued `output`, while generated OpenAI types model output as string.
  - Goal: normalize object payloads to canonical representation for strict typed conversion.
  - Acceptance: function/tool-result fixtures preserve structured output content in imported tool messages.

- [x] Decide and implement reasoning message aggregation behavior
  - Current gap: reasoning output items become standalone assistant messages; some fixtures expect reasoning merged with adjacent assistant text.
  - Goal: define canonical import behavior for reasoning-plus-message sequences and implement consistently.
  - Acceptance: `openai-responses-reasoning-blocks` and `openai-responses-reasoning-only-output` match expected message counts and roles.

- [x] Re-run and verify fixture suite after each fix
  - Command: `cargo test -p lingua --test import_fixtures -- --nocapture`
  - Process: update one behavior at a time and confirm no regressions in previously passing fixtures.
