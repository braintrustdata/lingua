# Google provider type update notes

## Changes in this update

### `ComputerUse` struct (additive, optional fields)

Two new optional fields added:
- `disabled_safety_policies: Option<Vec<DisabledSafetyPolicy>>` -- safety policies to disable during computer use
- `enable_prompt_injection_detection: Option<bool>` -- flag for prompt injection detection

### New `DisabledSafetyPolicy` enum

Eight variants: `AccountCreation`, `CommunicationTool`, `DataModification`, `FinancialTransactions`, `LegalTermsAndAgreements`, `SafetyPolicyUnspecified`, `SensitiveDataModification`, `UserConsentManagement`.

### `Environment` enum (additive variants)

Two new variants: `EnvironmentDesktop`, `EnvironmentMobile` (joining existing `EnvironmentBrowser`, `EnvironmentUnspecified`).

### Generator improvement

New `strip_quicktype_preamble()` function strips non-Rust diagnostic lines that CI environments or quicktype may emit before generated code.

## Adapter/converter impact

No hand-written code changes required. All changes are within the `ComputerUse` type hierarchy, which is a field on the `Tool` struct. The tool converter already does not convert `computer_use` to/from `UniversalTool` (pre-existing gap -- it handles `function_declarations`, `google_search`, `code_execution`, `google_search_retrieval`, and `url_context` only). The new fields and variants are all optional/additive, so serialization and deserialization remain backward-compatible.

## Items for human review

**`computer_use` tool conversion**: If cross-provider computer-use support is planned, the `Tool.computer_use` field needs a `UniversalTool` representation (likely a new builtin tool type). The new `DisabledSafetyPolicy` and expanded `Environment` variants would need to be modeled as part of that builtin tool's config. This is a pre-existing gap, not caused by this update.
