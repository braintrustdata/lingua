# Voice schema option: call timeline (Lingua mapping)

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

The SDK contract is in braintrust-sdk-javascript `docs/voice-schema/call-timeline/README.md`. In this option, integrations log an append-only event log in `voice.timeline` child rows. Lingua groups caller utterances into turns, builds `VoiceCall.speech` and links speech to steps. It copies the log into `VoiceCall.events`.

**Implemented in this PR:**
- the `VoiceEvent` types, including `message` and `missing{timeline}`, and `VoiceCall.events`
- regenerated TypeScript bindings
- a test that reads the SDK example logs into `VoiceEvent` without dropping fields

**Sketched below:** everything else. `VoiceCall.events` is always `None` today.

## Input keys

| Key | Row | Read as |
| --- | --- | --- |
| `voice.timeline_version` | call span | `u32`. Lingua takes the timeline path only when it is `1`. Any other value fails as unsupported. |
| `voice.pipeline` | call span | `cascaded \| realtime \| half_cascade`. Recorded only, in `VoiceCall`. |
| `voice.timeline_origin_unix_ms` | call span, only when there is no recording | `f64` |
| `audio.recordings` | call span | As in #486, plus `sources[{channel_index, boundary}]` |
| `voice.timeline` | `voice.timeline` rows | Object `{call_span_id, chunk_index, final}` |
| `voice.events` | same rows | `Vec<VoiceEvent>` |

All six keys go into `TRAJECTORY_METADATA_FIELDS`, so `voice_calls` stays header-only. For a versioned call, `audio.selections`, `turn.id` and `contrib.livekit.interrupted` on its turn spans are ignored, with one warning.

## Import

1. `import/span.rs`: add typed `Metadata` fields, and parse with `metadata_field` as in #486. A row whose `voice.events` fails to parse is an import failure naming the key.
2. `trajectory/voice.rs`: collect chunks by `(root_span_id, call_span_id)`. Deduplicate by `chunk_index`: identical retries are dropped, and differing duplicates are a failure. Concatenate in `chunk_index` order, then sort by `seq`.

## Recorded

- `VoiceCall.events`: the log in `seq` order.
- `VoiceCall.recordings`: only the call span's own `audio.recordings`, as in #486.

## Inferred

### 1. Speech

- **One `Speech` per `utterance_id` with a `speech_start`:**
  - `id`: the `utterance_id`.
  - `speaker`: from `speech_start`.
  - `interrupted`: from `speech_end`.
- **Interval** is `[speech_start.at_ms, end]`, where `end` is the first of:
  - the matching `speech_end.at_ms` (for the agent this is the played end; `truncate` is only a marker);
  - the same speaker's next `speech_start.at_ms` after it (warning `unclosed speech <id>`);
  - `call_end.at_ms`;
  - the largest `at_ms` in the log.
- **Order:** speech is ordered by start `at_ms`, then `seq`.

### 2. Selections

- **Channel:** resolved from `audio.recordings[].sources`: `session_input` / `transport_input` / `model_input` is the user, and `local_audio_output` / `transport_output` is the agent. A recording without `sources` falls back to 0 for the user and 1 for the agent, with a warning.
- **Clipping:** the interval is intersected with each ready recording's `[recording_start_offset_ms, + duration_ms]`. This gives one `AudioSelection` per overlapping recording, with recording-relative offsets and that channel. Gaps yield nothing.

### 3. Turns

- **Turn `k`** is the window of events with `seq` after `end_of_turn[k].seq` and before `end_of_turn[k+1].seq`; the last window runs to the end of the log.
- **Utterances:** `end_of_turn[k].utterance_ids`. If a caller utterance appears in no `end_of_turn`, it joins the turn window that contains its `speech_start.seq`. Before the first `end_of_turn`, there is no turn; such utterances keep empty `step_ids` and get a warning.
- **Responses:** every `response_start` in the window, whether or not `in_reply_to` is set. If `in_reply_to` names utterances outside the turn, Lingua warns `in_reply_to disagrees with turn window`.

### 4. `step_ids`

- **Caller speech:** the row ids of the `span_id`s of its turn's responses.
- **Agent speech:** the row id of the `span_id` of the `response_start` with the same `response_id`. Without a `response_id`, it gets the turn window's responses.
- **Row ids:** mapped through `(root_span_id, span_id)`, like recordings. A `span_id` missing from the trace is dropped with warning `timeline refers to a missing span <id>`.

### 5. Not inferred

- **Latencies and overlaps:** Lingua does not compute these. The app derives them from `events`:
  - **Turn latency:** first agent `speech_start.at_ms` in window `k`, minus the latest caller `speech_end.at_ms` among `end_of_turn[k].utterance_ids`.
  - **Stages:**
    - caller end → `end_of_turn.at_ms`
    - → first `response_start.at_ms`
    - → each `tool_call` / `tool_result` pair
    - → first agent `speech_start`
  - **Overlap:** the intersection of caller and agent speech intervals.
- **Transcripts:** they stay in `events`. Text without a transcript stays absent.

## Failures and warnings

| Case | Result |
| --- | --- |
| `voice.events` does not parse | Import failure on that row |
| `voice.timeline_version` is not 1 | Failure `unsupported voice timeline version` |
| Conflicting duplicate `chunk_index` | Failure |
| `seq` gap | Warning `timeline gap after seq N` |
| No chunk with `final: true` | Warning `call timeline is in progress or the session crashed`; open speech closes by the rules above |
| `speech_end` without a start | Warning; ignored |
| Unclosed speech | Warning; closed by the rules above |
| Unknown `span_id` | Warning; link dropped |
| `in_reply_to` disagrees with the window | Warning |
| `basis: audio` with no recording | Treated as `local`, with a warning |

## Trajectory changes

```rust
pub struct VoiceCall { pub id: String, pub recordings: Vec<AudioRecording>, pub speech: Vec<Speech>,
                       pub events: Option<Vec<VoiceEvent>> }
pub struct VoiceEvent { pub seq: u32, pub at_ms: f64, pub basis: TimeBasis,
                        #[serde(flatten)] pub detail: VoiceEventDetail }  // tagged by `kind`
```

- **Later, not in this PR:** `VoiceCall.pipeline` would carry `voice.pipeline` once the builder exists.
- **`Speech.id`:** this becomes an utterance id for timeline calls. #22036 uses it only as a key; the doc comment would read "span id or utterance id".

## How #22034 and #22036 consume it

- **Unchanged:** the voice bar reads `recordings`, `speech[].selections` and `speech[].step_ids`. Play-from-message, highlighting and overlap preference work as before.
- **New, optional:** a latency and overlap strip computed from `events` as in §5, with `missing` badges and truncate-versus-played markers. `trajectory-schema.ts` must accept `voice_calls[].events`.

## Implementation order

1. Typed import, the allow-list keys, and version gating.
2. Speech, selections and channel resolution. Add fixtures `call-timeline-cascaded-tool-barge-in.json` and `call-timeline-realtime-missing-transcript.json`, built from the SDK examples.
3. Turn windows and `step_ids`, then warnings.
4. In the app: the schema, then the strip.
