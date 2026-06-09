# OpenAI provider type update notes

Generated from spec update on `alex/embiggen-automations` branch.

## Changes applied in hand-written code

### 1. Role mapping expanded (OutputItemRoleEnum <-> InputItemRole)

`OutputItem.role` changed from `MessageRole` (only `Assistant`) to `RoleEnum` (Assistant, Critic, Developer, Discriminator, System, Tool, Unknown, User). The alias `OutputItemRoleEnum = RoleEnum` preserves compile compatibility.

**Fixed in `convert.rs`:**
- OutputItem -> InputItem: `Developer`, `System`, `User` now map to their `InputItemRole` counterparts instead of falling through to `User`.
- InputItem -> OutputItem: `Developer`, `System`, `User` now map to their `RoleEnum` counterparts instead of being dropped to `None`.
- Roles without an `InputItemRole` equivalent (`Critic`, `Discriminator`, `Tool`, `Unknown`) still fall through to `User` for output-to-input.

### 2. AdditionalTools item type

New `InputItemType::AdditionalTools` and `OutputItemType::AdditionalTools` variants added.

**Fixed in `convert.rs`:**
- Output-to-input and input-to-output type maps now include the `AdditionalTools` variant.
- InputItem -> universal Message conversion skips `AdditionalTools` items (they carry tool definitions, not message content, and have no universal equivalent).
- OutputItem -> universal Message conversion already skips unknown types via the `_ => continue` wildcard.

## Items that need no hand-written changes

### ModerationParam (request field)

New optional `moderation` field on `CreateChatCompletionRequestClass` and `CreateResponseClass`. This is a pass-through request parameter. The `OpenAIChatParams` struct uses `#[serde(flatten)] extras` which captures it automatically. The `OpenAIResponsesParams` struct similarly uses extras. No adapter change needed.

### ChatCompletionModeration / Moderation (response types)

New optional `moderation` field on `CreateChatCompletionResponse`, `CreateChatCompletionStreamResponse`, and `TheResponseObject`. The chat completions adapter works with raw `Value` for response construction, so these fields pass through without loss. The responses adapter similarly constructs responses from `Value`. No adapter change needed.

### Renamed type enums (cosmetic)

Several generated type enums were renamed during codegen due to name-collision resolution (`StickyType` -> `IndigoType`, etc.). The compatibility aliases in `generated.rs` were updated accordingly. No hand-written impact.

### SearchContentType removed, replaced by TType

`SearchContentType` (Image, Text) was removed; `TType` (Image, Text) is used instead for `search_content_types` and moderation `category_applied_input_types`. Same variants, different name. No hand-written code referenced `SearchContentType`.

### FinishReason doc-only change

The `finish_reason` field gained a doc link to the Model Spec. No semantic change; `content_filter` was already handled in universal `FinishReason::ContentFilter`.

### PromptCacheRetention doc-only change

Extended documentation about `gpt-5.5` behavior and ZDR defaults. No code impact.

## Items needing human clarification

### RoleEnum variants Critic, Discriminator, Unknown

These new `OutputItem` roles (`Critic`, `Discriminator`, `Unknown`) have no `InputItemRole` equivalent and no obvious universal message mapping. They currently fall through to `InputItemRole::User` in the output-to-input path. If these represent distinct conversation roles that should be preserved losslessly, the universal `Message` enum may need new variants or these roles need explicit `InputItemRole` support from OpenAI's spec.

### AdditionalTools universal representation

`AdditionalTools` items (mid-conversation tool injection) are skipped during universal conversion. If cross-provider translation of dynamic tool injection is desired, a universal representation would need to be designed.
