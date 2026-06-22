# Google provider type update notes

## Generated changes (discovery.json update)

### `ComputerUse` struct
- Added `enable_prompt_injection_detection: Option<bool>` field

### `Environment` enum
- Added `EnvironmentDesktop` variant (`ENVIRONMENT_DESKTOP`)
- Added `EnvironmentMobile` variant (`ENVIRONMENT_MOBILE`)

## Hand-written code changes

### `computer_use` builtin tool conversion (convert.rs)

The `computer_use` field on `Tool` was previously silently dropped during
Google-to-universal tool conversion. Added handling to convert it as a
builtin tool (matching the existing pattern for `code_execution`,
`google_search`, `google_search_retrieval`, and `url_context`).

The `ComputerUse` struct config is serialized/deserialized as a JSON value
in the builtin tool's `config` field, preserving all fields including the
new `enable_prompt_injection_detection` and `Environment` variants.

### Tests added

- `test_computer_use_builtin_roundtrip`: Verifies roundtrip with
  `EnvironmentDesktop` and `enable_prompt_injection_detection: true`
- `test_computer_use_with_mobile_environment_roundtrip`: Verifies roundtrip
  with `EnvironmentMobile` and `excluded_predefined_functions`

## No changes needed

- **FinishReason mapping**: No new finish reason variants were added in this
  update.
- **Request/response shape**: No changes to `GenerateContentRequest`,
  `GenerateContentResponse`, `Content`, `Part`, or `GenerationConfig`.
- **Streaming**: No streaming-related changes.
