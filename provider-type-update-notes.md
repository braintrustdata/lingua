## OpenAI provider type update notes

### Changes in generated types

1. **`Reasoning.context`** (new field + new `Context` enum)
   - The `Reasoning` struct gains `context: Option<Context>` where `Context` is `AllTurns | Auto | CurrentTurn`.
   - Controls which reasoning items are rendered back to the model on later turns.

2. **`tunnel_id`** (new field on MCP tool types)
   - `InputItemTool`, `MCPTool`, and `OutputItemTool` each gain `tunnel_id: Option<String>`.
   - A third option alongside `server_url` and `connector_id` for Secure MCP Tunnels.

3. **Field reordering** in `CreateResponseClass` and `TheResponseObject`
   - `reasoning` and `truncation` moved to different positions. No semantic change.

### Hand-written adapter changes made

- **`params.rs`**: Added `context: Option<ReasoningContext>` to `OpenAIReasoning` so the field is captured during deserialization instead of being silently dropped.
- **`responses_adapter.rs`**: Added `reasoning` to the extras preservation block in `request_to_universal`. This enables same-provider roundtrip (Responses -> Universal -> Responses) to preserve `reasoning.context` through the extras path, matching the existing pattern for `text`, `truncation`, and other Responses-specific fields.

### Items preserved without adapter changes

- **`tunnel_id`**: The existing builtin tool roundtrip path in `tool_discovery.rs` serializes/deserializes entire `InputItemTool`/`OutputItemTool` structs via `serde_json::Value`, so `tunnel_id` roundtrips correctly without hand-written changes.

### Items needing human clarification

- **`reasoning.context` has no universal representation for cross-provider transforms.** When a request originates in Responses format, `context` is preserved via extras for same-provider roundtrip. However, cross-provider transforms (e.g. Anthropic -> Responses) reconstruct reasoning from `ReasoningConfig` which does not model `context`. If `context` should be settable cross-provider, `ReasoningConfig` needs a new field and each provider's `to_provider` / `from` conversions need updating. This is an OpenAI-specific concept with no current equivalent in Anthropic, Google, or Bedrock.
