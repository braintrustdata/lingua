# Voice schema option: call timeline (Lingua mapping)

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

The SDK contract is in braintrust-sdk-javascript `docs/voice-schema/call-timeline/README.md`. In this option, integrations log an append-only event log in `voice.timeline` child rows. Lingua builds `VoiceCall.speech` from that log and copies the normalized log into `VoiceCall.events`.

**Implemented in this PR:**
- The `VoiceEvent` types with `VoiceCall.events` (`universal/trajectory.rs`).
- Regenerated TypeScript bindings and their exports.
- A test that reads the SDK's example event logs into `VoiceEvent` without dropping fields.

**Sketched below, not implemented:**
- Import of `voice.timeline` rows.
- Building speech from events.
- Linking speech to steps.
- Warnings.

`VoiceCall.events` is always `None` today.

## Input keys

| Key | Row | Read as |
| --- | --- | --- |
| `audio.recordings` | call span | unchanged from #486 |
| `voice.timeline` | `voice.timeline` task rows | `{call_span_id, chunk_index, final}` |
| `voice.events` | same rows | `Vec<VoiceEvent>` |
| `voice.timeline_origin_unix_ms` | call span, only when there is no recording | `f64` |

Add `voice.timeline`, `voice.events` and `voice.timeline_origin_unix_ms` to `TRAJECTORY_METADATA_FIELDS`. The app then fetches them with headers, and `voice_calls` stays header-only.

`audio.selections`, `turn.id` and `contrib.livekit.interrupted` are not read for timeline calls. A trace that logs both formats gets its speech from the timeline; turn-span selections are ignored and a warning is reported.

## Import

1. `import/span.rs`: add typed `Metadata` fields. Parse `voice.events` with `metadata_field::<Vec<VoiceEvent>>` and keep them on `SpanContext.voice` as `timeline: Option<LoggedTimeline>`. A malformed chunk becomes an import failure naming the key, following the #486 pattern.
2. `trajectory/voice.rs`: group chunks by `(root_span_id, call_span_id)`. Sort by `chunk_index`, concatenate, then sort by `seq`.

## Recorded (copied)

- `VoiceCall.events`: the concatenated events, deduplicated by `seq` (identical duplicates are dropped; conflicting ones fail).
- `VoiceCall.recordings`: as in #486.

## Inferred

1. **Speech.** There is one `Speech` per `utterance_id` that has a `speech_start`.
   - `id`: the `utterance_id`.
   - `speaker`: from the event.
   - `interrupted`: from `speech_end`.
   - Interval: `[start.at_ms, end.at_ms]`.
2. **Selections.** The interval is intersected with each ready recording: `[recording_start_offset_ms, + duration_ms]` on the call clock. This yields recording-relative offsets, with `channel_index` 0 for the user and 1 for the agent. An interval that spans segments yields several selections; a gap yields none for that stretch.
3. **`step_ids`:**
   - Agent speech: the `span_id` of the `response_start` with the same `response_id`.
   - User speech: the `span_id` of every `response_start` whose `in_reply_to` contains the utterance.
   - When no event names the utterance, use the first `response_start` after the `end_of_turn` that lists it, and add a warning `inferred reply for <utterance_id>`.
   - Ids map to row ids through `(root_span_id, span_id)` like recordings do, so `step_ids` name `WorkStep` or `Turn` ids exactly as #486 does.
4. **Ordering.** Speech is ordered by start `at_ms`, then `seq`.

Not inferred: transcripts, latencies and overlaps. These stay in `events`, where the app computes what it shows. That keeps derived numbers out of the Trajectory contract until a consumer needs them in Lingua.

## Failures and warnings

| Case | Result |
| --- | --- |
| `voice.events` does not parse | Import failure on that row |
| `seq` gap between chunks | Warning `timeline gap after seq N` (lost or not-yet-flushed chunk) |
| No chunk with `final: true` | Warning `call timeline is still open`; open speech ends at the last event |
| `speech_end` without `speech_start` | Warning; ignored |
| `speech_start` without `speech_end` | Warning; speech ends at the next event for that speaker, or at `call_end` |
| `response_start.span_id` not in trace | Failure `Timeline refers to a missing span <id>` |
| Two timelines for one call span | Failure |
| `basis: audio` with no recording | Treated as `local`, with a warning |

## Trajectory changes

```rust
pub struct VoiceCall { pub id: String, pub recordings: Vec<AudioRecording>, pub speech: Vec<Speech>,
                       pub events: Option<Vec<VoiceEvent>> }
pub struct VoiceEvent { pub seq: u32, pub at_ms: f64, pub basis: TimeBasis,
                        #[serde(flatten)] pub detail: VoiceEventDetail }  // tagged by `kind`
```

`Speech.id` becomes an utterance id rather than a span id. The app uses it only as a key (#22036 `getSpeechKeys` keys agent speech by `step_ids`), so this changes nothing there. The doc comment on `Speech.id` would change to "span id or utterance id".

## How #22034 and #22036 consume it

- **Unchanged:** the voice bar (`trajectory-voice-call-bar.tsx`) reads `recordings`, `speech[].selections` and `speech[].step_ids`. Play-from-message, highlighting and overlap preference (`findSpanAtAudioTime`) need no changes.
- **New, optional:** a latency and overlap strip reads `events`:
  - caller `speech_end` → `end_of_turn` → `response_start` → agent `speech_start`
  - overlap shading
  - `missing` badges
  - `truncate` vs `played_ms` difference markers

  The zod schema in `app/utils/preprocessors/trajectory-schema.ts` must accept `voice_calls[].events`.

## Implementation order

1. Typed import of `voice.events` and the allow-list keys.
2. Speech and selections from events, with fixtures `call-timeline-cascaded-tool-barge-in.json` and `call-timeline-realtime-missing-transcript.json` built from the SDK examples.
3. `step_ids`, then warnings.
4. App: schema plus the optional strip.
