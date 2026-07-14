# Provider Type Update Notes (2026-07 Anthropic Spec)

Items below require human clarification before the update is complete.

---

## 1. Tool search tools removed from generated `Tool` enum

**What changed:** Four `Tool` enum variants were removed from `generated.rs`:
- `ToolSearchToolBm25`
- `ToolSearchToolBm2520251119`
- `ToolSearchToolRegex`
- `ToolSearchToolRegex20251119`

The corresponding `Type` enum variants were also removed.

**Impact on hand-written code:**

- **Outbound (adapter.rs):** `anthropic_tool_value()` still emits tool_search
  tools as raw JSON via `is_anthropic_tool_search_builtin()`, bypassing the
  `Tool` enum entirely. This path continues to work because it never touches
  `Tool::try_from()`.

- **Inbound (tool_discovery.rs):** `anthropic_tool_search_tool()` still creates
  a `UniversalTool` with `builtin_type: "tool_search_tool_regex_20251119"` and
  `is_anthropic_tool_search_builtin()` still matches all four removed variant
  names. These functions operate on `UniversalTool`/string matching, not on the
  generated `Tool` enum, so they compile. However, any code path that attempts to
  **deserialize** a tool_search tool from JSON into the generated `Tool` enum
  will now fail.

- **Detection (detect.rs):** Parsing a request containing tool_search tools in
  the `tools` array now fails because `serde_json::from_str::<CreateMessageParams>`
  cannot deserialize the removed variants. Test updated to expect `is_err()`.

- **Validation (anthropic.rs):** Same as detection — test updated to expect
  `is_err()`.

**Needs clarification:**
1. Should the outbound tool_search emission in `anthropic_tool_value()` and
   `anthropic_tool_search_tool()` be updated to use the new tool types
   (`web_search_20260318` / `web_fetch_20260318`), or should tool_search
   emission be removed entirely?
2. Should `is_anthropic_tool_search_builtin()` be updated to match the new
   type names?
3. Does the Anthropic API still accept the old `tool_search_tool_regex_20251119`
   type, or has it been replaced?

---

## 2. Two generated roundtrip test failures

**Tests:**
- `test_roundtrip_responsesToolSearchInputParam_first_turn`
- `test_roundtrip_responsesToolSearchInputParam_followup_turn`

**Root cause:** The snapshot at
`payloads/snapshots/responsesToolSearchInputParam/anthropic/request.json`
contains `{"type": "tool_search_tool_regex_20251119"}` in the tools array.
This no longer deserializes into `CreateMessageParams` because the `Tool` enum
no longer has that variant. The test infrastructure discovers test cases at
runtime by parsing snapshot JSON, so the test case is "not found."

**Needs clarification:**
- These snapshots need to be recaptured against the current API spec. The
  `make capture` workflow (see `payloads/README.md`) can regenerate them, but
  the input parameters may need to be updated to use the replacement tool types.
- Alternatively, if tool_search is fully deprecated, the snapshot directory
  `responsesToolSearchInputParam` may need to be removed.

---

## 3. New tool variants: WebFetch20260318 and WebSearch20260318

**What changed:** Two new variants were added to the generated `Tool` enum:
- `Tool::WebFetch20260318` (struct `WebFetchTool20260318`)
- `Tool::WebSearch20260318` (struct `WebSearchTool20260318`)

**Current handling:** The generic `From<&Tool> for UniversalTool` impl handles
unknown/new tool variants via the `Custom` catch-all. These new tools will
round-trip through the universal format as custom tools. No hand-written code
changes are needed unless first-class universal support is desired.

**Needs clarification:**
- Should these new web tool types get explicit mappings in the converter
  (similar to how `computer_20250124` gets special handling), or is the
  catch-all sufficient?
- Are these the replacements for the removed tool_search variants?

---

## 4. Media type default change for unknown document types

**What changed:** In `convert.rs`, the fallback for unknown file extensions in
`extension_to_document_media_type()` changed from
`Some(generated::FluffyMediaType::TextPlain)` to `None`.

**Current behavior:** Unknown file types now return `None` instead of defaulting
to `text/plain`. This is a behavior change: previously, a document with an
unrecognized extension would be sent as `text/plain`; now it will have no media
type set.

**Needs clarification:**
- Is this intentional? The generated type rename from `FluffyMediaType` to
  `Base64ImageSourceMediaType` suggests the type's scope narrowed. Verify that
  omitting the media type for unknown extensions is acceptable to the API.
