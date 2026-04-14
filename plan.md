# Fix strict response schema normalization for array-valued type

## Root cause

`crates/lingua/src/universal/response_format.rs` normalizes strict-target JSON Schema nodes through `StrictTargetSchemaNodeView`, which currently models `type` as `Option<String>`. Schemas that use valid array-valued JSON Schema unions such as `["string", "null"]` fail deserialization before normalization runs.

## Target files

- `crates/lingua/src/universal/response_format.rs`
- `payloads/transforms/transform_errors.json`
- `crates/coverage-report/src/requests_expected_differences.json`

## Expected behavior after fix

- Strict-target normalization accepts schema nodes whose `type` is either a string or an array of strings.
- Existing behavior is preserved:
  - object schemas still default `additionalProperties` to `false`
  - non-Google targets still strip `propertyOrdering`
  - Google still preserves `propertyOrdering`
  - Anthropic still strips unsupported array and numeric constraints
- The repro payload `textFormatJsonSchemaNullableUnionTypeGpt54NanoParam` succeeds without temporary whitelists.

## Tests to add or update

- Add unit tests in `response_format.rs` covering array-valued `type` for:
  - string/null leaf schemas
  - object/null object schemas
  - Anthropic array/numeric stripping
  - Google and non-Google `propertyOrdering` behavior

## Validation commands

```bash
cargo test -p lingua response_format
make capture FILTER=textFormatJsonSchemaNullableUnionTypeGpt54NanoParam
make test-payloads
cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures
```
