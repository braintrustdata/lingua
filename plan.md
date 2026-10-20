# Google media display names

## Root cause

Google media import discards Blob/FileData.display_name, and file export always sets it to None. Model-role file_data is skipped. The approved mapping treats displayName as a portable media filename. Universal Image/Audio variants have no filename, so named media must use the existing File variant with its original MIME type.

## Target files

- `crates/lingua/src/providers/google/convert.rs`: preserve display names on user/model inline and referenced files; share typed file export across both roles.
- `payloads/cases/params.ts`: add `chatCompletionsFileFilenameToGoogleDisplayNameParam` with a named PDF in both native formats.
- `payloads/snapshots/`, `payloads/transforms/`, and transform test snapshots: capture the new case and accept intentional request filename changes.

## Expected behavior

- Google displayName and universal File.filename map bidirectionally, including images and audio represented as named files.
- Raw base64, MIME-bearing data URLs, HTTPS references, and GCS references preserve names.
- Unnamed user images/audio retain their existing universal variants.
- Google-only operational features remain explicitly unsupported across providers.

## Tests and expected diffs

- Add typed converter matrix tests for inline/referenced media, user/model roles, PDF/image/audio MIME types, and absent/empty/nonempty filenames.
- Preserve the existing MIME inference and unnamed media tests.
- Expect Google requests with filename-bearing files to gain displayName. Do not accept filename loss. Remove obsolete Google filename exceptions and classify only explicit unsupported errors or provider billing metadata differences for the new case.

## Validation sequence

1. Add the payload case and run `make capture FILTER=chatCompletionsFileFilenameToGoogleDisplayNameParam` before converter changes; inspect failures and baseline loss.
2. Confirm focused display-name tests fail on the current converter, then implement the typed mapping.
3. Run `make capture FILTER=chatCompletionsFileFilenameToGoogleDisplayNameParam`.
4. Run `cargo test -p lingua --lib providers::google::convert::tests`.
5. Re-run the case capture and run `make test-payloads`.
6. Regenerate failed transforms only if the logic fix leaves stale artifacts; run payload fixture sync checks.
7. Run `cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures`.
8. Run `make typed-boundary-check` and `make typed-boundary-check-branch BASE=main`, plus a branch check against the actual PR base.

## Coverage findings

- Remove the three obsolete Google filename-loss exceptions after the converter fix.
- The new named-PDF case exposes existing Anthropic filename loss. Add a typed guard in `crates/lingua/src/providers/anthropic/convert.rs`, test its unsupported-mapping error, and keep the document-title test focused on its independent title contract. Record only this case's explicit errors in the request expectations and payload error manifest.
- Google captures also contain served_service_tier billing metadata, which Anthropic streams cannot retain. Scope the existing metadata normalization expectation to this new case and provider pair.
