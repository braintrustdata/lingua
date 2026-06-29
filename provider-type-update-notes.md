# Google provider type update notes

## Changes in this update

### `ComputerUse` struct: two new optional fields

- `disabled_safety_policies: Option<Vec<DisabledSafetyPolicy>>` — allows disabling specific safety policies for computer use sessions.
- `enable_prompt_injection_detection: Option<bool>` — enables prompt injection detection on computer-use requests.

### New `DisabledSafetyPolicy` enum

Eight variants: `AccountCreation`, `CommunicationTool`, `DataModification`, `FinancialTransactions`, `LegalTermsAndAgreements`, `SafetyPolicyUnspecified`, `SensitiveDataModification`, `UserConsentManagement`.

### `Environment` enum: two new variants

- `EnvironmentDesktop` — desktop environment for computer use.
- `EnvironmentMobile` — mobile environment for computer use.

## Adapter changes made

Added `computer_use` as a builtin tool type in the `GoogleTool` <-> `UniversalTool` conversion (`convert.rs`). Previously, `computer_use` was silently dropped during tool conversion. It now roundtrips through `UniversalTool::builtin` with its full typed config (including the new fields), matching the existing pattern for `google_search`, `code_execution`, `google_search_retrieval`, and `url_context`.

## No changes needed

- No new `FinishReason` variants were added, so no finish-reason mapping updates are required.
- No new roles or content part types were added.
- All new fields are optional and serialize/deserialize correctly via serde without adapter changes.

## Pre-existing gaps not addressed

- `file_search` and `google_maps` tool types on the `GoogleTool` struct are not yet handled in the builtin tool conversion (same as before this update — they were not modified in this diff).
