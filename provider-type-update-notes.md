## OpenAI provider type update notes

### New `reasoning.context` field (needs human clarification)

The OpenAI Responses API `Reasoning` struct now includes a `context` field
(`Context` enum: `auto`, `current_turn`, `all_turns`). This controls which
reasoning items from previous turns are rendered back to the model.

**Current status:**

- The field is captured during deserialization via the hand-written
  `OpenAIReasoning` param struct (`params.rs`).
- Same-provider roundtrip (Responses -> universal -> Responses) preserves
  `context` through the extras mechanism.
- Cross-provider conversion drops `context` because `ReasoningConfig` has
  no universal equivalent. No other provider supports this concept today.

**Needs human decision:**

Should `context` be added to `ReasoningConfig` as a universal field (like
`summary`/`SummaryMode`)? If so, cross-provider adapters would need a
policy for providers that don't support it (drop silently, or map to a
provider-specific default).

### New `tunnel_id` field on MCP tool structs

A `tunnel_id: Option<String>` field was added to `InputItemTool`, `MCPTool`,
and `OutputItemTool` generated structs. This is a new option for connecting
to MCP servers via a secure tunnel instead of a direct URL.

No adapter changes required. All hand-written code constructs these structs
via JSON deserialization, and the new field is optional with
`skip_serializing_if`. It serializes and deserializes correctly with no
code changes.

### Field reordering in `CreateResponseClass` and `TheResponseObject`

The `reasoning` and `truncation` fields moved positions within
`CreateResponseClass` and `TheResponseObject`. This is cosmetic only; serde
is order-independent. No adapter changes needed.

### `truncation` deprecated in OpenAPI spec

The OpenAPI spec marks `truncation` as `deprecated: true` on the
`ResponseProperties` schema. The generated Rust code does not carry a
`#[deprecated]` attribute, so this is a spec-only change with no build
impact. No adapter changes needed.
