# Google provider type update notes

**Spec revision**: 20260611 -> 20260614

## Changes made

### FinishReason normalization (convert.rs + response.rs)

Three Google `FinishReason` variants were falling through the catch-all to
`FinishReason::Other(...)` instead of mapping to `ContentFilter`:

- `ImageRecitation` — image content flagged for recitation (same class as `Recitation`)
- `ImageProhibitedContent` — prohibited content in image output (same class as `ProhibitedContent`)
- `Language` — language-filtering safety stop

These are content-safety/filtering reasons with the same semantics as their
non-image counterparts that were already mapped. Both the typed
`From<&GoogleFinishReason>` impl (non-streaming path) and the string-based
`from_provider_string` (streaming path) were updated to keep them consistent.

Tests added:
- `test_image_safety_finish_reasons_map_to_content_filter`
- `test_language_finish_reason_maps_to_content_filter`
- Extended `test_google_safety_related_strings_map_to_content_filter`

## No changes needed

The discovery.json update (revision 20260611 -> 20260614) contains only JSON key
reordering and the revision bump. The `schemas` section is semantically identical.
Type regeneration (`cargo run --bin generate-types -- google`) produces the same
`generated.rs`. No new enum variants, fields, or types were introduced.

## Items needing human clarification

The following generated fields exist in the Google types but have no universal
representation. They predate this update but are noted here for completeness:

### Part.tool_call / Part.tool_response (server-side tool execution)

Google's `toolCall` and `toolResponse` represent server-side tool invocations
(the model requests execution that Google runs server-side, distinct from
client-side `functionCall`/`functionResponse`). The universal format has no
concept of server-executed tools. These fields are preserved in round-trip
serialization but silently skipped during conversion to universal messages.

### FunctionResponse.parts / .scheduling / .will_continue (non-blocking calls)

These fields support Google's non-blocking function call pattern (multimedia
responses, scheduling modes). The universal tool result only carries a single
JSON `output` value. These fields survive round-trip serialization but are not
converted.

### Part.part_metadata / Part.video_metadata / Part.media_resolution

Agent/multimedia metadata fields with no universal counterpart. Preserved in
serialization, not converted.

### FinishReason variants remaining in Other catch-all

These variants map to `FinishReason::Other(...)` because they represent
non-safety, non-standard stop conditions without a clear universal equivalent:

- `FinishReasonUnspecified` — unset/unknown
- `MalformedFunctionCall` — malformed tool call output
- `MalformedResponse` — malformed model response
- `MissingThoughtSignature` — thought signature validation failure
- `TooManyToolCalls` — exceeded tool call limit
- `UnexpectedToolCall` — tool call when none expected
- `NoImage` — image generation failed (not a safety filter)
- `ImageOther` — other image generation issue (not clearly safety-related)

These pass through as `Other("MALFORMED_FUNCTION_CALL")` etc., preserving the
provider-specific reason string for downstream consumers.
