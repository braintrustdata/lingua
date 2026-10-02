# Provider type update notes

## Google — Discovery revision 20260717 → 20260915

### Root cause of the failed run

The spec update is purely additive, but `Blob` and `FileData` gained a `displayName` field and
`crates/lingua/src/providers/google/convert.rs` built both structs with exhaustive struct literals
(no `..Default::default()`), so `cargo build` failed with eight `E0063` errors. Generation itself
succeeded; nothing was removed from the generated surface.

### Decisions taken

- **`Blob.displayName` / `FileData.displayName` are intentionally left unset.** The spec defines
  them as the identifier the *model* uses to refer to the media when `verbalization_mode` is
  `REFERENCE_ONLY`. That is a Google-specific reference handle, not equivalent to universal
  `UserContentPart::File::filename` (source metadata), so mapping the two would be a surface-shape
  match rather than a semantic one. Emitted Google payloads are byte-identical to the previous
  revision. Not blocking.
- **`FinishReason::PUP_LIMITED_DISABLED` passes through as `FinishReason::Other`.** It describes an
  account-level Prohibited Use Policy state, not a filtered response, so it is not folded into
  `ContentFilter` alongside `SAFETY` / `ESCALATION`. Not blocking.

### Newly generated, not yet mapped to universal types

These fields now deserialize and re-serialize losslessly on the Google-native boundary but have no
universal representation, so they are dropped by `Content` → `Message` conversion:

- `Part.audioTranscription` (and `WordInfo`)
- `Part.mediaProcessing`
- `GenerationConfig.audioTranscriptionConfig`
- `GenerateContentRequest.labels`
- `Part.toolCall.toolName`

These are new provider features, not lost behavior — the previous revision had no such fields, so
there is no compatibility regression. Adding universal mappings requires the payload-case-first
workflow in `AGENTS.md`. Not blocking.

### Pre-existing issues observed (unchanged by this update)

- `GenerationConfig.response_schema` is typed `Box<Option<Schema>>`, which
  `add_serde_skip_if_none` in `crates/generate-types/src/main.rs` does not match, so requests
  always emit `"responseSchema": null`. This predates the update and is baked into 115 payload
  transform snapshots.
- `Part.mediaResolution` now references `V1mainMediaResolution`, which the generator's `V1main`
  prefix stripping normalizes back to the public name `MediaResolution` with an identical `level`
  enum. The hard-coded `google_missing_discovery_schema("MediaResolution")` fallback is therefore
  no longer reachable; it is harmless but now dead for this revision.
