# Provider Type Update Notes

## OpenAI spec update (openapi.yml)

### 1. `Reasoning.context` field — needs human clarification

**What changed:** The generated `Reasoning` struct gained a new `context: Option<Context>` field.
The `Context` enum has three variants: `AllTurns`, `Auto`, `CurrentTurn`.

**What it controls:** Per the OpenAI spec, `context` determines which reasoning items are
rendered back to the model on later turns in a multi-turn conversation.

**What was done:**
- Added `context: Option<Context>` to `OpenAIReasoning` in `params.rs` so the field is no longer
  silently dropped during typed deserialization.
- Stashed the full `reasoning` object (including `context`) in Responses extras during
  `request_to_universal`, so it roundtrips through the extras-first path in
  `request_from_universal`.
- Added two focused tests verifying roundtrip preservation and absence when unset.

**What needs human decision:** Should `context` become a field on the universal `ReasoningConfig`?

- **Current behavior:** `context` survives same-provider (Responses-to-Responses) roundtrips via
  extras. Cross-provider conversions (e.g. Anthropic-to-Responses) will not include `context`.
- **Argument for universal field:** If other providers add similar multi-turn reasoning context
  controls, a universal field avoids per-provider extras plumbing.
- **Argument against:** This concept is currently OpenAI-specific with no equivalent in Anthropic,
  Google, or Bedrock. Adding it to `ReasoningConfig` would be speculative and violates the
  "don't invent workarounds for non-lossy mapping" guideline.

### 2. `tunnel_id` on MCP tool types — no action needed

**What changed:** `InputItemTool`, `MCPTool`, and `OutputItemTool` gained a new
`tunnel_id: Option<String>` field.

**Why no action:** MCP tools are OpenAI Responses API-specific. No adapter code constructs
these types during cross-provider transformation. The new optional field deserializes as `None`
when absent, and the generated types handle serialization correctly.

### 3. Field reordering in `CreateResponseClass` / `TheResponseObject` — no action needed

**What changed:** `reasoning` and `truncation` fields were reordered within the structs.

**Why no action:** serde matches fields by name, not position. Pure cosmetic change with
no semantic impact on serialization or deserialization.
