## root cause

- `transform_request(..., ProviderFormat::Responses, Some("gpt-5-mini"))` forces a Responses -> Universal -> Responses round-trip for `gpt-5-*` models. During reconstruction, URL-backed `input_file` parts were getting a synthesized `filename`, which changed `{ file_url }` into `{ file_url, filename }`.
- `Responses input_file.file_url` imports into universal `UserContentPart::File`, but universal URL-backed files do not currently round-trip to Anthropic or Google correctly:
  - Anthropic skips regular `File` parts unless they carry an Anthropic-specific marker.
  - Google does not emit `UserContentPart::File` parts at all on the outbound request path.

## target files

- `crates/lingua/src/providers/openai/convert.rs`
- `crates/lingua/src/providers/anthropic/convert.rs`
- `crates/lingua/src/providers/google/convert.rs`
- `payloads/cases/params.ts`

## expected behavior after fix

- Responses requests that already contain `input_file.file_url` should not gain a synthetic `filename` during forced translation.
- Inline/base64 file payloads can still receive a synthesized filename when needed.
- Universal URL-backed files should translate to:
  - Anthropic document blocks with `source.type = "url"`
  - Google parts with `file_data.file_uri`

## tests to add or update

- Add a unit test in `convert.rs` covering URL-backed file parts.
- Add a payload case in `payloads/cases/params.ts` for Responses `input_file.file_url`.
- Add Anthropic converter tests for regular URL-backed `File` parts.
- Add Google converter tests for URL-backed `File` parts.

## expected-diff impact

- New transform fixture(s) for the added payload case.
- No expected-differences file changes.

## command sequence to validate

1. `cargo test -p lingua user_url_backed_file_does_not_synthesize_filename -- --nocapture`
2. `cargo test -p lingua test_reasoning_responses_file_url_preserves_absent_filename -- --nocapture`
3. `cargo test -p lingua test_regular_url_file_converts_to_anthropic_document -- --nocapture`
4. `cargo test -p lingua test_file_url_to_google_file_data -- --nocapture`
5. `make capture FILTER=responsesInputFileUrlParam`
6. `make test-payloads`
