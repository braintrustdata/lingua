## root cause

`transform_request(..., ProviderFormat::Responses, Some("gpt-5-mini"))` forces a Responses -> Universal -> Responses round-trip for `gpt-5-*` models. During reconstruction, URL-backed `input_file` parts get a synthesized `filename`, which changes `{ file_url }` into `{ file_url, filename }`.

## target files

- `crates/lingua/src/providers/openai/convert.rs`
- `payloads/cases/params.ts`

## expected behavior after fix

- Responses requests that already contain `input_file.file_url` should not gain a synthetic `filename` during forced translation.
- Inline/base64 file payloads can still receive a synthesized filename when needed.

## tests to add or update

- Add a unit test in `convert.rs` covering URL-backed file parts.
- Add a payload case in `payloads/cases/params.ts` for Responses `input_file.file_url`.

## expected-diff impact

- New transform fixture(s) for the added payload case.
- No expected-differences file changes.

## command sequence to validate

1. `cargo test -p lingua responses_input_file_url_roundtrip_preserves_absent_filename`
2. `make capture FILTER=responsesInputFileUrlParam`
3. `make test-payloads`
