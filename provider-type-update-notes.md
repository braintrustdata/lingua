# Anthropic provider type update notes

## Summary

The generated type update for the Anthropic provider introduces:
1. A new `CodeExecutionTool20260521` struct and `Tool::CodeExecution20260521` variant
2. A new `AllowedCaller` enum (used in tool `allowed_callers` fields)
3. A rename of the old `AllowedCaller` enum to `CallerType` (used in `Caller.caller_type`)
4. A rename of `Category` to `RefusalCategory` with a new `MilitaryWeapons` variant
5. A new `ToolType::CodeExecution20260521` variant
6. Doc-only changes to `CacheControlEphemeral` and `Ttl`

## No hand-written code changes required

All generated changes are either additive or renames that do not affect serde wire format:

- **New `Tool::CodeExecution20260521`**: Handled by the existing catch-all arm in `From<&Tool> for UniversalTool` (convert.rs:2368) which serializes non-custom tools to builtin config. The reverse path in `TryFrom<&UniversalTool> for Tool` (convert.rs:2445) deserializes builtin configs back to `Tool` via serde.

- **`AllowedCaller` -> `CallerType` rename**: The hand-written code references `generated::Caller` (the struct), never the inner enum type directly. Serde serialization wire values are unchanged (`rename_all = "snake_case"` with the same variants).

- **`Category` -> `RefusalCategory` + `MilitaryWeapons`**: Not referenced in hand-written code. Refusal handling uses the `stop_reason` string field, where `"refusal"` already maps to `FinishReason::ContentFilter` (response.rs:119, 256).

- **New `AllowedCaller` enum**: Only used in generated tool struct `allowed_callers` fields. Not referenced in hand-written code.

## Potential spec/generation gap: `CallerType` missing `CodeExecution20260521`

The generated `CallerType` enum (used in `Caller.caller_type`) has 3 variants:
- `CodeExecution20250825`
- `CodeExecution20260120`
- `Direct`

The generated `AllowedCaller` enum (used in tool `allowed_callers`) has 4 variants — the 3 above plus `CodeExecution20260521`.

If a tool is configured with `allowed_callers: ["code_execution_20260521"]` and the model invokes another tool from that code execution environment, the API response would include a `Caller` block with `type: "code_execution_20260521"`. Since `CallerType` lacks this variant, deserialization of that content block would fail.

**Action needed**: This is a spec or generation-pipeline issue. The `CallerType` enum likely needs `CodeExecution20260521` added, but this requires either updating the OpenAPI spec or adjusting the generation pipeline. Do not edit `generated.rs` directly.
