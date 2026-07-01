# Anthropic provider type update notes

## Changes applied

### Generator fix: tool search variant injection pattern

The `add_anthropic_tool_search_tool_variants` function in `crates/generate-types/src/main.rs`
used a fragile string anchor (`WebSearch20260209` immediately before `#[serde(untagged)]`) to
inject tool search tool variants into the `Tool` enum. The new `WebSearch20260318` variant
broke this pattern, silently dropping all four `ToolSearchTool*` variants from the `Tool` enum.

Fixed by anchoring on `#[serde(untagged)]\n    Custom(CustomTool),` instead, which is stable
across future tool variant additions.

### Tests added

- `test_web_search_20260318_roundtrips_through_universal` - verifies new `WebSearchTool20260318` serializes to a builtin universal tool and round-trips back with fields preserved.
- `test_web_fetch_20260318_roundtrips_through_universal` - same for `WebFetchTool20260318` including `max_content_tokens`.
- `test_tool_search_variants_still_deserialize` - confirms all four tool search type strings (`tool_search_tool_bm25`, `tool_search_tool_bm25_20251119`, `tool_search_tool_regex`, `tool_search_tool_regex_20251119`) still deserialize into `Tool` variants.

## No hand-written converter changes needed

The new `Tool` variants (`WebFetch20260318`, `WebSearch20260318`) are handled by the existing
catch-all arm in `From<&Tool> for UniversalTool` (convert.rs) which serializes any non-Custom
variant to JSON and extracts the `type` field to create a builtin universal tool. The reverse
path (`TryFrom<&UniversalTool> for Tool`) deserializes Anthropic builtins from their stored
config via `serde_json::from_value::<Tool>()`. Both paths work without changes.

## Observations for human review

### `ResponseInclusion` enum is generated but unused by tool structs

The new `ResponseInclusion` enum (`Excluded`, `Full`) was generated, but the `response_inclusion`
field on `WebFetchTool20260318` and `WebSearchTool20260318` is typed as `Option<String>` rather
than `Option<ResponseInclusion>`. This is a generation quality issue — the enum exists but isn't
wired into the structs. Not blocking (string deserialization works), but the generator could be
improved to use the typed enum instead.
