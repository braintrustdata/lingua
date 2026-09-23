# Google provider type update plan

## Root cause

Google `Part.audioTranscription` is omitted by content conversion and streaming conversion. `GenerationConfig.audioTranscriptionConfig` and request `labels` are retained only in Google extras, then lost on cross-provider conversion. These fields have no approved lossless universal mapping. The PR's native type additions also need compilation, generation, and payload validation.

## Target files

- `crates/lingua/src/providers/google/convert.rs`: reject transcript parts in both content roles.
- `crates/lingua/src/providers/google/adapter.rs` and `params.rs`: reject transcription settings and labels on request import; reject transcript parts in streams.
- `crates/lingua/src/processing/transform.rs`: perform native stream passthrough before universal conversion so Google-only stream fields remain intact.
- `crates/lingua/tests/google_provider_only_fields.rs`: cover same-format byte preservation and cross-provider errors.
- `payloads/cases/params.ts` and `types.ts`: describe the affected request behaviors for capture.
- `payloads/transforms/transform_errors.json` and the two cases' captured transform errors: classify each unsupported target pair narrowly.
- `payloads/scripts/transforms/__snapshots__/transforms.test.ts.snap`: refresh Google file display names emitted by the PR's existing file conversion change.
- Generator, bindings, and expected transform artifacts only if validation shows they need repair or regeneration.

## Expected behavior

Google requests, responses, and streams continue to preserve native fields through same-format passthrough. A cross-provider transform carrying either transcript data/settings or request labels returns a field-specific unsupported-mapping error. No universal fields are added.

## Tests and expected diffs

Add focused Rust tests for transcript request/response/stream and request configuration/labels, including error identity and native passthrough. Captured cross-provider artifacts may change from silently lossy output to explicit unsupported errors. Keep differences narrow to the new cases.

## Validation commands

1. `make capture FILTER=googleRequestLabelsParam` and `make capture FILTER=googleAudioTranscriptionConfigParam` to document baseline behavior.
2. `cargo test -p lingua --lib providers::google::convert` and `cargo test -p lingua --lib providers::google::adapter`.
3. `cargo test -p lingua --test google_provider_only_fields`.
4. Re-run both captures, then `make test-payloads` and `make regenerate-failed-transforms` only if artifacts are stale.
5. `cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures`.
6. `make typed-boundary-check` and `make typed-boundary-check-branch BASE=main`.
7. Run formatting, type generation checks, and broader build checks for the full PR.
