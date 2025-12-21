warning: profiles for the non root package will be ignored, specify profiles at the workspace root:
package:   /Users/kenjiang/Development/braintrust/gateway/braintrust-llm-router/lingua/Cargo.toml
workspace: /Users/kenjiang/Development/braintrust/gateway/Cargo.toml
warning: methods `is_valid_request` and `is_valid_response` are never used
  --> braintrust-llm-router/lingua/src/bin/coverage-report/main.rs:65:8
   |
64 | impl SemanticCheck {
   | ------------------ methods in this implementation
65 |     fn is_valid_request(&self) -> bool {
   |        ^^^^^^^^^^^^^^^^
...
69 |     fn is_valid_response(&self) -> bool {
   |        ^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` on by default

warning: `lingua` (bin "generate-coverage-report") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running `/Users/kenjiang/Development/braintrust/gateway/target/debug/generate-coverage-report`
## Cross-Provider Transformation Coverage

**Validation levels:**
- ✅ Full: Schema valid + semantics preserved
- ⚠️ Partial: Schema valid but semantic issues (or mixed results)
- ❌ Failed: Transform error or schema invalid

### Request Transformations

| Source ↓ / Target → | ChatCompletions | Anthropic | Responses | Google | Bedrock |
|---------------------|-------------|-------------|-------------|-------------|-------------|
| ChatCompletions | - | ⚠️ 12/14 | ✅ 14/14 | ✅ 14/14 | ❌ 0/14 |
| Anthropic | ⚠️ 12/14 | - | ⚠️ 12/14 | ✅ 14/14 | ❌ 0/14 |
| Responses | ❌ 0/13 | ❌ 0/13 | - | ❌ 0/13 | ❌ 0/13 |
| Google | ❌ 0/14 | ❌ 0/14 | ❌ 0/14 | - | ❌ 0/14 |
| Bedrock | ❌ 0/13 | ❌ 0/13 | ❌ 0/13 | ❌ 0/13 | - |

### Response Transformations

| Source ↓ / Target → | ChatCompletions | Anthropic | Responses | Google | Bedrock |
|---------------------|-------------|-------------|-------------|-------------|-------------|
| ChatCompletions | - | ❌ 0/7 | ✅ 7/7 | ⚠️ 0/7 | ❌ 0/7 |
| Anthropic | ⚠️ 0/7 | - | ⚠️ 0/7 | ⚠️ 0/7 | ❌ 0/7 |
| Responses | ❌ 0/7 | ❌ 0/7 | - | ❌ 0/7 | ⚠️ 2/7 |
| Google | ⚠️ 0/7 | ❌ 0/7 | ⚠️ 0/7 | - | ❌ 0/7 |
| Bedrock | ❌ 0/6 | ❌ 0/6 | ❌ 0/6 | ❌ 0/6 | - |

### Summary

- **Full Coverage (schema + semantics):** 87/408 (21.3%)
- **Schema Only:** 42 (valid schema but semantic issues)
- **Failed:** 279

**Requests:** 78/272 full, 0 schema-only, 194 failed
**Responses:** 9/136 full, 42 schema-only, 85 failed

### Issues by Source

<details>
<summary>❌ Google (84 issues)</summary>

<details>
<summary>→ Bedrock (21)</summary>

- `complexReasoningRequest (request)` - Unsupported target format: Converse
- `complexReasoningRequest (followup)` - Unsupported target format: Converse
- `multimodalRequest (request)` - Unsupported target format: Converse
- `multimodalRequest (followup)` - Unsupported target format: Converse
- `reasoningRequest (request)` - Unsupported target format: Converse
- `reasoningRequest (followup)` - Unsupported target format: Converse
- `reasoningRequestTruncated (request)` - Unsupported target format: Converse
- `reasoningRequestTruncated (followup)` - Unsupported target format: Converse
- `reasoningWithOutput (request)` - Unsupported target format: Converse
- `reasoningWithOutput (followup)` - Unsupported target format: Converse
- `simpleRequest (request)` - Unsupported target format: Converse
- `simpleRequest (followup)` - Unsupported target format: Converse
- `toolCallRequest (request)` - Unsupported target format: Converse
- `toolCallRequest (followup)` - Unsupported target format: Converse
- `complexReasoningRequest (response)` - Unsupported target format: Converse
- `multimodalRequest (response)` - Unsupported target format: Converse
- `reasoningRequest (response)` - Unsupported target format: Converse
- `reasoningRequestTruncated (response)` - Unsupported target format: Converse
- `reasoningWithOutput (response)` - Unsupported target format: Converse
- `simpleRequest (response)` - Unsupported target format: Converse
- `toolCallRequest (response)` - Unsupported target format: Converse

</details>

<details>
<summary>→ ChatCompletions (21)</summary>

- `complexReasoningRequest (request)` - OpenAI schema: missing field `model`
- `complexReasoningRequest (followup)` - OpenAI schema: missing field `model`
- `multimodalRequest (request)` - OpenAI schema: missing field `model`
- `multimodalRequest (followup)` - OpenAI schema: missing field `model`
- `reasoningRequest (request)` - OpenAI schema: missing field `model`
- `reasoningRequest (followup)` - OpenAI schema: missing field `model`
- `reasoningRequestTruncated (request)` - OpenAI schema: missing field `model`
- `reasoningRequestTruncated (followup)` - OpenAI schema: missing field `model`
- `reasoningWithOutput (request)` - OpenAI schema: missing field `model`
- `reasoningWithOutput (followup)` - OpenAI schema: missing field `model`
- `simpleRequest (request)` - OpenAI schema: missing field `model`
- `simpleRequest (followup)` - OpenAI schema: missing field `model`
- `toolCallRequest (request)` - OpenAI schema: missing field `type`
- `toolCallRequest (followup)` - OpenAI schema: missing field `type`
- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

<details>
<summary>→ Anthropic (21)</summary>

- `complexReasoningRequest (request)` - Anthropic schema: missing field `model`
- `complexReasoningRequest (followup)` - Anthropic schema: missing field `model`
- `multimodalRequest (request)` - Anthropic schema: missing field `model`
- `multimodalRequest (followup)` - Anthropic schema: missing field `model`
- `reasoningRequest (request)` - Anthropic schema: missing field `model`
- `reasoningRequest (followup)` - Anthropic schema: missing field `model`
- `reasoningRequestTruncated (request)` - Anthropic schema: missing field `model`
- `reasoningRequestTruncated (followup)` - Anthropic schema: missing field `model`
- `reasoningWithOutput (request)` - Anthropic schema: missing field `model`
- `reasoningWithOutput (followup)` - Anthropic schema: missing field `model`
- `simpleRequest (request)` - Anthropic schema: missing field `model`
- `simpleRequest (followup)` - Anthropic schema: missing field `model`
- `toolCallRequest (request)` - Anthropic schema: data did not match any variant of untagged enum Tool
- `toolCallRequest (followup)` - Anthropic schema: data did not match any variant of untagged enum Tool
- `complexReasoningRequest (response)` - Anthropic response schema: missing field `usage`
- `multimodalRequest (response)` - Anthropic response schema: missing field `usage`
- `reasoningRequest (response)` - Anthropic response schema: missing field `usage`
- `reasoningRequestTruncated (response)` - Anthropic response schema: missing field `usage`
- `reasoningWithOutput (response)` - Anthropic response schema: missing field `usage`
- `simpleRequest (response)` - Anthropic response schema: missing field `usage`
- `toolCallRequest (response)` - Anthropic response schema: missing field `usage`

</details>

<details>
<summary>→ Responses (21)</summary>

- `complexReasoningRequest (request)` - OpenAI schema: missing field `model`
- `complexReasoningRequest (followup)` - OpenAI schema: missing field `model`
- `multimodalRequest (request)` - OpenAI schema: missing field `model`
- `multimodalRequest (followup)` - OpenAI schema: missing field `model`
- `reasoningRequest (request)` - OpenAI schema: missing field `model`
- `reasoningRequest (followup)` - OpenAI schema: missing field `model`
- `reasoningRequestTruncated (request)` - OpenAI schema: missing field `model`
- `reasoningRequestTruncated (followup)` - OpenAI schema: missing field `model`
- `reasoningWithOutput (request)` - OpenAI schema: missing field `model`
- `reasoningWithOutput (followup)` - OpenAI schema: missing field `model`
- `simpleRequest (request)` - OpenAI schema: missing field `model`
- `simpleRequest (followup)` - OpenAI schema: missing field `model`
- `toolCallRequest (request)` - OpenAI schema: missing field `type`
- `toolCallRequest (followup)` - OpenAI schema: missing field `type`
- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

</details>

<details>
<summary>❌ Responses (78 issues)</summary>

<details>
<summary>→ Google (20)</summary>

- `complexReasoningRequest (request)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (followup)` - Responses API (input[]) not yet supported
- `multimodalRequest (request)` - Responses API (input[]) not yet supported
- `multimodalRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequest (request)` - Responses API (input[]) not yet supported
- `reasoningRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequestTruncated (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (followup)` - Responses API (input[]) not yet supported
- `simpleRequest (request)` - Responses API (input[]) not yet supported
- `simpleRequest (followup)` - Responses API (input[]) not yet supported
- `toolCallRequest (request)` - Responses API (input[]) not yet supported
- `toolCallRequest (followup)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (response)` - Unable to detect source format
- `multimodalRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unable to detect source format
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unable to detect source format
- `simpleRequest (response)` - Unable to detect source format
- `toolCallRequest (response)` - Unable to detect source format

</details>

<details>
<summary>→ ChatCompletions (20)</summary>

- `complexReasoningRequest (request)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (followup)` - Responses API (input[]) not yet supported
- `multimodalRequest (request)` - Responses API (input[]) not yet supported
- `multimodalRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequest (request)` - Responses API (input[]) not yet supported
- `reasoningRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequestTruncated (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (followup)` - Responses API (input[]) not yet supported
- `simpleRequest (request)` - Responses API (input[]) not yet supported
- `simpleRequest (followup)` - Responses API (input[]) not yet supported
- `toolCallRequest (request)` - Responses API (input[]) not yet supported
- `toolCallRequest (followup)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (response)` - Unable to detect source format
- `multimodalRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unable to detect source format
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unable to detect source format
- `simpleRequest (response)` - Unable to detect source format
- `toolCallRequest (response)` - Unable to detect source format

</details>

<details>
<summary>→ Anthropic (20)</summary>

- `complexReasoningRequest (request)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (followup)` - Responses API (input[]) not yet supported
- `multimodalRequest (request)` - Responses API (input[]) not yet supported
- `multimodalRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequest (request)` - Responses API (input[]) not yet supported
- `reasoningRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequestTruncated (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (followup)` - Responses API (input[]) not yet supported
- `simpleRequest (request)` - Responses API (input[]) not yet supported
- `simpleRequest (followup)` - Responses API (input[]) not yet supported
- `toolCallRequest (request)` - Responses API (input[]) not yet supported
- `toolCallRequest (followup)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (response)` - Unable to detect source format
- `multimodalRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unable to detect source format
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unable to detect source format
- `simpleRequest (response)` - Unable to detect source format
- `toolCallRequest (response)` - Unable to detect source format

</details>

<details>
<summary>→ Bedrock (18)</summary>

- `complexReasoningRequest (request)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (followup)` - Responses API (input[]) not yet supported
- `multimodalRequest (request)` - Responses API (input[]) not yet supported
- `multimodalRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequest (request)` - Responses API (input[]) not yet supported
- `reasoningRequest (followup)` - Responses API (input[]) not yet supported
- `reasoningRequestTruncated (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (request)` - Responses API (input[]) not yet supported
- `reasoningWithOutput (followup)` - Responses API (input[]) not yet supported
- `simpleRequest (request)` - Responses API (input[]) not yet supported
- `simpleRequest (followup)` - Responses API (input[]) not yet supported
- `toolCallRequest (request)` - Responses API (input[]) not yet supported
- `toolCallRequest (followup)` - Responses API (input[]) not yet supported
- `complexReasoningRequest (response)` - Unable to detect source format
- `reasoningRequest (response)` - Unable to detect source format
- `reasoningWithOutput (response)` - Unable to detect source format
- `simpleRequest (response)` - Unable to detect source format
- `toolCallRequest (response)` - Unable to detect source format

</details>

</details>

<details>
<summary>❌ Bedrock (76 issues)</summary>

<details>
<summary>→ Google (19)</summary>

- `complexReasoningRequest (request)` - Unable to detect source format
- `complexReasoningRequest (followup)` - Unable to detect source format
- `multimodalRequest (request)` - Unable to detect source format
- `reasoningRequest (request)` - Unable to detect source format
- `reasoningRequest (followup)` - Unable to detect source format
- `reasoningRequestTruncated (request)` - Unable to detect source format
- `reasoningRequestTruncated (followup)` - Unable to detect source format
- `reasoningWithOutput (request)` - Unable to detect source format
- `reasoningWithOutput (followup)` - Unable to detect source format
- `simpleRequest (request)` - Unable to detect source format
- `simpleRequest (followup)` - Unable to detect source format
- `toolCallRequest (request)` - Unable to detect source format
- `toolCallRequest (followup)` - Unable to detect source format
- `complexReasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unsupported source format: Converse
- `simpleRequest (response)` - Unsupported source format: Converse
- `toolCallRequest (response)` - Unsupported source format: Converse

</details>

<details>
<summary>→ Anthropic (19)</summary>

- `complexReasoningRequest (request)` - Unable to detect source format
- `complexReasoningRequest (followup)` - Unable to detect source format
- `multimodalRequest (request)` - Unable to detect source format
- `reasoningRequest (request)` - Unable to detect source format
- `reasoningRequest (followup)` - Unable to detect source format
- `reasoningRequestTruncated (request)` - Unable to detect source format
- `reasoningRequestTruncated (followup)` - Unable to detect source format
- `reasoningWithOutput (request)` - Unable to detect source format
- `reasoningWithOutput (followup)` - Unable to detect source format
- `simpleRequest (request)` - Unable to detect source format
- `simpleRequest (followup)` - Unable to detect source format
- `toolCallRequest (request)` - Unable to detect source format
- `toolCallRequest (followup)` - Unable to detect source format
- `complexReasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unsupported source format: Converse
- `simpleRequest (response)` - Unsupported source format: Converse
- `toolCallRequest (response)` - Unsupported source format: Converse

</details>

<details>
<summary>→ ChatCompletions (19)</summary>

- `complexReasoningRequest (request)` - Unable to detect source format
- `complexReasoningRequest (followup)` - Unable to detect source format
- `multimodalRequest (request)` - Unable to detect source format
- `reasoningRequest (request)` - Unable to detect source format
- `reasoningRequest (followup)` - Unable to detect source format
- `reasoningRequestTruncated (request)` - Unable to detect source format
- `reasoningRequestTruncated (followup)` - Unable to detect source format
- `reasoningWithOutput (request)` - Unable to detect source format
- `reasoningWithOutput (followup)` - Unable to detect source format
- `simpleRequest (request)` - Unable to detect source format
- `simpleRequest (followup)` - Unable to detect source format
- `toolCallRequest (request)` - Unable to detect source format
- `toolCallRequest (followup)` - Unable to detect source format
- `complexReasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unsupported source format: Converse
- `simpleRequest (response)` - Unsupported source format: Converse
- `toolCallRequest (response)` - Unsupported source format: Converse

</details>

<details>
<summary>→ Responses (19)</summary>

- `complexReasoningRequest (request)` - Unable to detect source format
- `complexReasoningRequest (followup)` - Unable to detect source format
- `multimodalRequest (request)` - Unable to detect source format
- `reasoningRequest (request)` - Unable to detect source format
- `reasoningRequest (followup)` - Unable to detect source format
- `reasoningRequestTruncated (request)` - Unable to detect source format
- `reasoningRequestTruncated (followup)` - Unable to detect source format
- `reasoningWithOutput (request)` - Unable to detect source format
- `reasoningWithOutput (followup)` - Unable to detect source format
- `simpleRequest (request)` - Unable to detect source format
- `simpleRequest (followup)` - Unable to detect source format
- `toolCallRequest (request)` - Unable to detect source format
- `toolCallRequest (followup)` - Unable to detect source format
- `complexReasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequest (response)` - Unsupported source format: Converse
- `reasoningRequestTruncated (response)` - Unsupported source format: Converse
- `reasoningWithOutput (response)` - Unsupported source format: Converse
- `simpleRequest (response)` - Unsupported source format: Converse
- `toolCallRequest (response)` - Unsupported source format: Converse

</details>

</details>

<details>
<summary>❌ Anthropic (46 issues)</summary>

<details>
<summary>→ Bedrock (21)</summary>

- `complexReasoningRequest (request)` - Unsupported target format: Converse
- `complexReasoningRequest (followup)` - Unsupported target format: Converse
- `multimodalRequest (request)` - Unsupported target format: Converse
- `multimodalRequest (followup)` - Unsupported target format: Converse
- `reasoningRequest (request)` - Unsupported target format: Converse
- `reasoningRequest (followup)` - Unsupported target format: Converse
- `reasoningRequestTruncated (request)` - Unsupported target format: Converse
- `reasoningRequestTruncated (followup)` - Unsupported target format: Converse
- `reasoningWithOutput (request)` - Unsupported target format: Converse
- `reasoningWithOutput (followup)` - Unsupported target format: Converse
- `simpleRequest (request)` - Unsupported target format: Converse
- `simpleRequest (followup)` - Unsupported target format: Converse
- `toolCallRequest (request)` - Unsupported target format: Converse
- `toolCallRequest (followup)` - Unsupported target format: Converse
- `complexReasoningRequest (response)` - Unsupported target format: Converse
- `multimodalRequest (response)` - Unsupported target format: Converse
- `reasoningRequest (response)` - Unsupported target format: Converse
- `reasoningRequestTruncated (response)` - Unsupported target format: Converse
- `reasoningWithOutput (response)` - Unsupported target format: Converse
- `simpleRequest (response)` - Unsupported target format: Converse
- `toolCallRequest (response)` - Unsupported target format: Converse

</details>

<details>
<summary>→ ChatCompletions (9)</summary>

- `toolCallRequest (request)` - OpenAI schema: data did not match any variant of untagged enum ChatCompletionToolChoiceOption
- `toolCallRequest (followup)` - OpenAI schema: data did not match any variant of untagged enum ChatCompletionToolChoiceOption
- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

<details>
<summary>→ Responses (9)</summary>

- `toolCallRequest (request)` - OpenAI schema: data did not match any variant of untagged enum ChatCompletionToolChoiceOption
- `toolCallRequest (followup)` - OpenAI schema: data did not match any variant of untagged enum ChatCompletionToolChoiceOption
- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

<details>
<summary>→ Google (7)</summary>

- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

</details>

<details>
<summary>❌ ChatCompletions (37 issues)</summary>

<details>
<summary>→ Bedrock (21)</summary>

- `complexReasoningRequest (request)` - Unsupported target format: Converse
- `complexReasoningRequest (followup)` - Unsupported target format: Converse
- `multimodalRequest (request)` - Unsupported target format: Converse
- `multimodalRequest (followup)` - Unsupported target format: Converse
- `reasoningRequest (request)` - Unsupported target format: Converse
- `reasoningRequest (followup)` - Unsupported target format: Converse
- `reasoningRequestTruncated (request)` - Unsupported target format: Converse
- `reasoningRequestTruncated (followup)` - Unsupported target format: Converse
- `reasoningWithOutput (request)` - Unsupported target format: Converse
- `reasoningWithOutput (followup)` - Unsupported target format: Converse
- `simpleRequest (request)` - Unsupported target format: Converse
- `simpleRequest (followup)` - Unsupported target format: Converse
- `toolCallRequest (request)` - Unsupported target format: Converse
- `toolCallRequest (followup)` - Unsupported target format: Converse
- `complexReasoningRequest (response)` - Unsupported target format: Converse
- `multimodalRequest (response)` - Unsupported target format: Converse
- `reasoningRequest (response)` - Unsupported target format: Converse
- `reasoningRequestTruncated (response)` - Unsupported target format: Converse
- `reasoningWithOutput (response)` - Unsupported target format: Converse
- `simpleRequest (response)` - Unsupported target format: Converse
- `toolCallRequest (response)` - Unsupported target format: Converse

</details>

<details>
<summary>→ Anthropic (9)</summary>

- `toolCallRequest (request)` - Anthropic schema: invalid type: string "auto", expected struct ToolChoice
- `toolCallRequest (followup)` - Anthropic schema: invalid type: string "auto", expected struct ToolChoice
- `complexReasoningRequest (response)` - Anthropic response schema: missing field `usage`
- `multimodalRequest (response)` - Anthropic response schema: missing field `usage`
- `reasoningRequest (response)` - Anthropic response schema: missing field `usage`
- `reasoningRequestTruncated (response)` - Anthropic response schema: missing field `usage`
- `reasoningWithOutput (response)` - Anthropic response schema: missing field `usage`
- `simpleRequest (response)` - Anthropic response schema: missing field `usage`
- `toolCallRequest (response)` - Anthropic response schema: missing field `usage`

</details>

<details>
<summary>→ Google (7)</summary>

- `complexReasoningRequest (response)` - semantic: missing usage
- `multimodalRequest (response)` - semantic: missing usage
- `reasoningRequest (response)` - semantic: missing usage
- `reasoningRequestTruncated (response)` - semantic: missing usage
- `reasoningWithOutput (response)` - semantic: missing usage
- `simpleRequest (response)` - semantic: missing usage
- `toolCallRequest (response)` - semantic: missing usage

</details>

</details>


