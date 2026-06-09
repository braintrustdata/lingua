# OpenAI provider type update notes

## Changes reviewed

Spec update generated the following semantic changes in `crates/lingua/src/providers/openai/generated.rs`:

### New types (no adapter changes needed)

- **`ModerationParam`**: Optional request-side moderation config on `CreateChatCompletionRequestClass`, `CreateResponseClass`. Passthrough — not part of universal representation.
- **`ChatCompletionModeration` / `Moderation` (response)**: Optional moderation results on chat completion and Responses API responses. Passthrough — not converted to/from universal.
- **`ModerationResult`, `InputClass`, `OutputClass`, `StickyType`, `FriskyType`, `ResultType`, `TType`**: Supporting moderation result types. No adapter impact.
- **`PromptCacheRetention` doc update**: Updated docs for gpt-5.5 models. No code impact.
- **`FinishReason` doc update**: Added Model Spec link. No enum variant changes.

### Renames (no hand-written code impact)

- `SearchContentType` → `TType` (Image, Text variants): No hand-written references existed.
- `Moderation` (image gen level) → `ModerationEnum`: No hand-written references.
- Quicktype name shifts (`StickyType` → `IndigoType`, `IndigoType` → `IndecentType`, `IndecentType` → `HilariousType`, etc.): Compatibility alias `InputItemContentListType` updated dynamically by `find_enum_with_variants` in generate-types.

### Adapter changes made

1. **`RoleEnum` replacing `MessageRole` for `OutputItem.role`**
   - `OutputItem.role` changed from `Option<MessageRole>` (only `Assistant`) to `Option<RoleEnum>` (Assistant, Critic, Developer, Discriminator, System, Tool, Unknown, User).
   - **Fix**: Changed OutputItem→InputItem conversion to return `ConvertError::UnsupportedMapping` for `Critic`, `Discriminator`, `Tool`, `Unknown` instead of silently coercing to `User`.
   - InputItem→OutputItem conversion already maps all four `InputItemRole` variants correctly.
   - 4 focused tests added.

2. **`AdditionalTools` item type**
   - New `InputItemType::AdditionalTools` and `OutputItemType::AdditionalTools` for mid-conversation tool definitions.
   - **Fix**: Added explicit type mapping in OutputItem↔InputItem conversions. Added explicit `ConvertError::UnsupportedMapping` error in InputItem→universal and OutputItem→universal conversions (universal `Message` has no representation for mid-conversation tool definitions).
   - 3 focused tests added.

3. **`moderation` field on `CreateChatCompletionRequestClass`**
   - New required `Option<ModerationParam>` field. Added `moderation: None` to 3 struct literals in `detect.rs` tests.

## Items needing human clarification

### `RoleEnum` variants without universal mapping

The new `RoleEnum` includes `Critic`, `Discriminator`, `Tool`, and `Unknown` variants. These do not map to any existing `InputItemRole` variant (which only has `Assistant`, `Developer`, `System`, `User`). The converter now returns an explicit error for these.

**Decision needed**: Should these roles be:
- Added to `InputItemRole` (if OpenAI intends them as first-class input roles)?
- Mapped to existing universal roles with specific semantics (e.g., `Tool` → tool-result-like handling)?
- Left as errors (current behavior) until OpenAI documents their intended usage?

### `AdditionalTools` without universal representation

`InputItemType::AdditionalTools` / `OutputItemType::AdditionalTools` carries mid-conversation tool definitions (with a `tools` array and `role: developer`). Universal `Message` has no representation for injecting tool definitions mid-conversation — tools are currently only in `UniversalParams.tools`.

**Decision needed**: Should Lingua:
- Add a universal message variant for mid-conversation tool injection?
- Treat `additional_tools` as a provider-specific passthrough (preserved in extras)?
- Keep the current explicit error until the feature is better understood?
