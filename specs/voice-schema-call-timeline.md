# Voice schema option: call timeline (Lingua mapping)

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

The SDK contract is in braintrust-sdk-javascript `docs/voice-schema/call-timeline/README.md`. In this option, integrations log an append-only event log in `voice.timeline` child rows. Lingua groups caller utterances into turns, builds `VoiceCall.speech` and links speech to steps. It copies the log into `VoiceCall.events`.

**Implemented in this PR:**

- the `VoiceEvent` types, including `message` and `missing{timeline}`, and `VoiceCall.events`
- regenerated TypeScript bindings
- a test that reads the SDK example logs into `VoiceEvent` without dropping fields
- the builder: typed import of `voice.timeline` rows and call-span keys, `VoiceCall.events`, derived `speech` (selections, `interrupted`, `step_ids`) and the failures and warnings below, with fixtures built from the SDK examples. It was written from this spec by a second engineer; see "Implementation decisions".

**Not implemented:** the app side (`TRAJECTORY_METADATA_FIELDS` lives in the app and needs the new keys after #486 is rebased).

## Schema at a glance

**Input keys read** (full contract and field meanings: SDK README "Schema at a glance"):

```ts
// call span metadata
"voice.timeline_version": 1                                  // required; any other value fails
"voice.pipeline": "cascaded" | "realtime" | "half_cascade"   // required; validated, not carried
"voice.timeline_origin_unix_ms"?: number                     // unix ms; parsed, unused
"voice.server_vad_silence_ms"?: number                       // ms; parsed, unused
"audio.recordings"?: { id; state?; attachment: { span_id; ref }; duration_ms?;
  sources?: { channel_index; boundary }[]; timeline?: { origin_unix_ms; recording_start_offset_ms } }[]
// voice.timeline row metadata
"voice.timeline": { call_span_id: string; chunk_index: number; final: boolean }
"voice.events": VoiceEvent[]                                 // same shape as the SDK zod, parsed verbatim
```

**Trajectory output** (`crates/lingua/src/universal/trajectory.rs`, bindings in `bindings/typescript/src/generated/`):

```ts
type VoiceCall = {
  id: string                    // recorded: call span id
  recordings: AudioRecording[]  // recorded: call span's audio.recordings (as #486)
  speech: Speech[]              // inferred from the timeline
  events?: VoiceEvent[]         // recorded: the log in seq order; absent for #486 turn-span calls
}
type Speech = {
  id: string                    // recorded: utterance_id
  speaker: "user" | "agent"     // recorded: speech_start.speaker
  step_ids: string[]            // inferred: turn windows / in_reply_to, response_id -> response_start.span_id row id
  selections: AudioSelection[]  // inferred: [speech_start.at_ms, end] clipped to each ready recording
  interrupted?: boolean         // recorded: speech_end.interrupted
}
type AudioRecording = { id: string; attachment_span_id: string; attachment_pointer: string; start_time?: string; duration_ms?: number }
  // recorded: attachment.span_id -> row id, attachment.ref; start_time = origin_unix_ms + recording_start_offset_ms (ISO)
type AudioSelection = { recording_id: string; channel_index?: number; start_offset_ms: number; end_offset_ms: number }
  // inferred: offsets relative to the recording; channel from recordings[].sources
type VoiceEvent = { seq: number; at_ms: number; basis: TimeBasis } & ( /* 11 kinds, tagged by "kind" */ )
type TimeBasis = "audio" | "local" | "provider"
type TranscriptSource = "stt" | "provider" | "playout"
type ToolOutcome = "ok" | "error" | "cancelled"
type MissingEvidence = "transcript" | "speech_end" | "audio" | "tool_result" | "timeline"
```

Speech end is inferred when `speech_end` is absent (next same-speaker start, `call_end`, or the last `at_ms`). Turns, latencies and overlaps are not stored on `VoiceCall`.

## Input keys

| Key | Row | Read as |
| --- | --- | --- |
| `voice.timeline_version` | call span | `u32`. Lingua takes the timeline path only when it is `1`. Any other value fails as unsupported. |
| `voice.pipeline` | call span | `cascaded \| realtime \| half_cascade`. Validated only; not yet carried on `VoiceCall` (see Trajectory changes). `half_cascade` uses realtime caller events and cascaded agent playout. |
| `voice.timeline_origin_unix_ms` | call span, only when there is no recording | `f64` |
| `voice.server_vad_silence_ms` | call span, optional | `f64`. Not used in derivation, because the SDK already subtracted it from caller `speech_end`; it explains `local` caller end times. |
| `audio.recordings` | call span | As in #486, plus `sources[{channel_index, boundary}]` |
| `voice.timeline` | `voice.timeline` rows | Object `{call_span_id, chunk_index, final}` |
| `voice.events` | same rows | `Vec<VoiceEvent>` |

All seven keys belong in the app's `TRAJECTORY_METADATA_FIELDS` (the constant is not in Lingua), so `voice_calls` stays header-only. For a versioned call, `audio.selections`, `turn.id` and `contrib.livekit.interrupted` on its turn spans are ignored, with one warning.

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
- **Responses:** for each `response_start`:
  - **With `in_reply_to`:** `in_reply_to` wins. The response belongs to the turn holding those utterances, wherever it falls in `seq`. The SDK must set it on preemptive generations, which start before their turn's `end_of_turn`. If it names utterances from more than one turn, the response links to each of them, with a warning.
  - **Without `in_reply_to`:** the response belongs to the window that contains its `seq`. This is only a fallback.

### 4. `step_ids`

- **Caller speech:** the row ids of the `span_id`s of its turn's responses.
- **Agent speech:** the row id of the `span_id` of the latest `response_start` with the same `response_id` whose `at_ms` is not after the speech start (the first one when all are later). Without a `response_id`, it gets the turn window's responses.
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
- **Transcripts:** they stay in `events`. Text without a transcript stays absent. Lingua takes each transcript's `utterance_id` as given, because the SDK already attached it. STT finals attach to the open or most recently ended utterance of the uncommitted turn; provider (realtime) finals carry `message_id` and attach through the `message` that mapped it, or FIFO to the oldest committed `end_of_turn` group without a final when there is no item id, assuming one provider input item per group. The text goes on the group's first utterance, and the other utterances in the group are covered, not missing. Lingua does not re-attach.
- **Message matching:** a `message` event maps utterances to a chat item id for matching conversation messages. Realtime caller `message` events are emitted at the entry of the answering generation (LiveKit decision E), so an utterance with no final still has one (`item_u2` in the realtime fixture). Lingua copies `message` events and derives nothing from them; a missing transcript adds no warning beyond the `missing` event already in the log.

## Failures and warnings

| Case | Result |
| --- | --- |
| `voice.events` does not parse | Import failure on that row |
| `voice.timeline_version` is not 1 | Failure `unsupported voice timeline version` |
| `voice.pipeline` missing or not a known value | Warning; field ignored |
| `voice.timeline` row whose call span lacks `voice.timeline_version` | Warning `timeline rows without a versioned call span`; rows ignored |
| Conflicting duplicate `chunk_index` | Failure |
| `seq` gap | Warning `timeline gap after seq N` |
| No chunk with `final: true` | Warning `call timeline is in progress or the session crashed`; open speech closes by the rules above |
| `speech_end` without a start | Warning; ignored |
| Unclosed speech | Warning; closed by the rules above |
| Unknown `span_id` | Warning; link dropped |
| `in_reply_to` spans several turns | Warning |
| `response_start` before the first `end_of_turn` and without `in_reply_to` | Warning; unlinked |
| `basis: audio` with no recording | Treated as `local`, with a warning |

## Trajectory changes

```rust
pub struct VoiceCall { pub id: String, pub recordings: Vec<AudioRecording>, pub speech: Vec<Speech>,
                       pub events: Option<Vec<VoiceEvent>> }
pub struct VoiceEvent { pub seq: u32, pub at_ms: f64, pub basis: TimeBasis,
                        #[serde(flatten)] pub detail: VoiceEventDetail }  // tagged by `kind`
```

- **Not in this PR:** `VoiceCall.pipeline` (an enum from `voice.pipeline`). The builder validates `voice.pipeline` but does not carry it.
- **`Speech.id`:** this becomes an utterance id for timeline calls. #22036 uses it only as a key; the doc comment would read "span id or utterance id".

## How #22034 and #22036 consume it

- **Unchanged:** the voice bar reads `recordings`, `speech[].selections` and `speech[].step_ids`. Play-from-message, highlighting and overlap preference work as before.
- **New, optional:** a latency and overlap strip computed from `events` as in §5, with `missing` badges and truncate markers. A truncate value that differs from `played_ms` appears only from integrations that observe the provider independently (Pipecat); with LiveKit the two values are equal. `trajectory-schema.ts` must accept `voice_calls[].events`.

## Implementation order

1. Typed import, the allow-list keys, and version gating.
2. Speech, selections and channel resolution. Add fixtures `call-timeline-cascaded-tool-barge-in.json` and `call-timeline-realtime-missing-transcript.json`, built from the SDK examples.
3. Turn windows and `step_ids`, then warnings.
4. In the app: the schema, then the strip.

## LiveKit 1.9.2 alignment (2026-10-12)

The SDK capture table was checked against `@livekit/agents` 1.9.2. That review changed the inputs Lingua sees; the builder rules themselves are unchanged.

- **Agent `utterance_id`:** `<SpeechHandle.id>/<n>` in both pipelines, and `response_id` is `SpeechHandle.id`. Tool continuations get their own handle (`speech_2/0` follows the `speech_1` tool call). The realtime output item id arrives through a later agent `message`. Agent `Speech.id` in both fixtures is therefore `speech_2/0` / `speech_4/0`.
- **Realtime `end_of_turn`:** LiveKit has no commit event when server turn detection is on, so the SDK emits it at `_realtimeGenerationTaskImpl` entry. In the realtime fixture that is 3600 for `u1` (it was 3400). The turn windows and `step_ids` are unchanged.
- **`truncate`:** LiveKit derives `audio_end_ms` from the same `playbackPosition`, so the realtime fixture's truncate is at 7900 with 2900, equal to `played_ms`.
- **Preemptive generations (cascaded):** these carry `in_reply_to` from the `onPreemptiveGeneration` snapshot. §3 already prefers it over the window.
- **Cascaded `end_of_turn`:** emitted when the reply is scheduled (the user commit), dated at the `onEndOfTurn` resolve time. That is why it follows the turn-1 transcript in `seq` in the cascaded fixture. Turns that LiveKit drops after `onEndOfTurn` (AMD `skipReply`, non-interruptible speech, `StopResponse`) emit no `end_of_turn`, and their utterances fall back to "the turn open at their start" (§3).
- **Realtime `end_of_turn`:** excludes caller utterances still open at `_realtimeGenerationTaskImpl` entry.
- **Realtime caller `message` (decision E):** emitted at `_realtimeGenerationTaskImpl` entry from the `chatCtx` diff, after `end_of_turn`, even without a transcript. The realtime fixture now maps `u1` → `item_u1` at 3600 and `u2` → `item_u2` at 9000, and the late `u1` final carries `message_id: item_u1`. Speech and `step_ids` are unchanged.
- **Realtime caller speech end:** the realtime fixture gives no `voice.server_vad_silence_ms`, so caller `speech_end` is the receipt time including the 500 ms server-VAD padding (`u1` 3400, `u2` 9000), and turn 2 runs `end_of_turn`/generation 9000, `reschedule` 9300–9700, final audio 9700–12500. Caller selections end at the padded times.
- **Handoff:** `voice.pipeline` is the pipeline at call start. A handoff that changes the LLM type is out of MVP scope.

## Implementation decisions

The builder in this PR was written from this spec by an engineer who did not write it. These are the points the spec left open and how the builder settles them. They are now part of the contract; "Essential" ones change what customers see.

### Essential (changes customer-visible output)

1. **Input keys / Import (call span).** Does a versioned call span without `audio.recordings` produce a `VoiceCall`? The SDK README says the call span is "the session span when there is no recording", but the Lingua spec doesn't say. **Choice:** yes. It gets a call with `recordings: []`, speech with no selections, and `events`.
2. **Failures (`timeline_version` not 1, conflicting chunk).** After the failure, what happens to the call? **Choice:** the call is still emitted with its recordings, `speech: []` and no `events`. Its turn spans are still ignored, so there is no fallback to the #486 path.
3. **Failures (conflicting duplicate `chunk_index`).** Should the whole timeline fail, or only that chunk? **Choice:** the whole timeline is dropped, with failure `Conflicting voice timeline chunk N` on the later row in (start, id) order.
4. **Input keys ("turn spans are ignored, with one warning").** Does "ignored" cover `turn.id` for trajectory turn grouping too? Which spans belong to the call? What does the warning say? **Choice:** turn spans are ignored only in `voice_calls`, and `turn.id` still shapes trajectory turns. A turn span belongs to the call when its first selection's `recording_span_id` points at the call. One warning per call: `turn span voice metadata is ignored for a timeline call`.
5. **§1 Speech interval ("same speaker's next speech_start after it").** Is "after" measured by `seq` or by `at_ms`? **Choice:** the next same-speaker start in speech order (`at_ms`, then `seq`). The `unclosed speech <id>` warning fires for every unclosed speech, including speech closed by `call_end` or the last `at_ms`.
6. **§1 Speech.** What about duplicate `speech_start` or `speech_end` events for one `utterance_id`? **Choice:** the first in `seq` order wins, and later ones are ignored without a warning.
7. **§2 Selections ("each ready recording").** Readiness depends on `state`, which #486 didn't parse, and the possible values aren't listed. **Choice:** `state` is parsed as `ready | <other>`. A missing `state` counts as ready. A recording with no `timeline` or no `duration_ms` yields no selections. Zero-length intersections yield nothing.
8. **§2 Channel.** What if `sources` exists but has no boundary for the speaker, or has several? What does the warning say? **Choice:** the first matching source wins. With no match, the 0/1 fallback applies, as when `sources` is missing. Warning: `recording <id> has no <user|agent> source; using channel N`, deduplicated. Unknown boundary values parse as `other` and are ignored, not treated as an import failure.
9. **§3 Turns, unlisted caller utterance before the first `end_of_turn`.** The warning text isn't given. **Choice:** `speech <id> is before the first end of turn`.
10. **§3 Responses.** `in_reply_to` can be empty, or name utterances that are in no turn (unknown or pre-turn). **Choice:** an empty array counts as absent, so the window fallback applies. Unknown ids are skipped. If no turn resolves, the response is unlinked with warning `response <id> is not linked to a turn`. The same text covers the spec's "before the first `end_of_turn` and without `in_reply_to`" case. Multi-turn warning: `response <id> replies to several turns`.
11. **§4 Agent speech.** What if `speech_start.response_id` matches no `response_start`? What is the "turn window" for agent speech without a `response_id`? **Choice:** with no match, `step_ids` is empty and there is no warning. Without a `response_id`, the window containing the agent `speech_start.seq` is used, with that turn's responses as resolved by `in_reply_to` or window. For several `response_start` events with one `response_id`, the first won; the span-points branch changed this to the latest one started before playback, because LiveKit cascaded tool replies reuse the speech handle (see `voice-schema-span-points.md`).
12. **Failures: `seq` gap.** A missing first chunk (first `seq` > 0) isn't covered by "gap after seq N". **Choice:** added warning `timeline gap before seq N`.
13. **Failures: `basis: audio` with no recording.** Does "treated as local" rewrite the events, and how often does it warn? **Choice:** events stay verbatim, with one warning per call: `audio basis without a recording is treated as local`. "No recording" means `audio.recordings` is empty.
14. **Failures: `voice.pipeline`.** The warning text isn't given. **Choice:** `Missing metadata.voice.pipeline`, only on spans with `voice.timeline_version`, and `Invalid metadata.voice.pipeline: <serde error>` on any span. Both are import warnings on the call span.
15. **Failures: rows without a versioned call span.** Which span gets the warning? **Choice:** once per (root, `call_span_id`), on the first row in (start, id) order.
16. **Failures: unknown `span_id`.** Which `span_id`s are checked? **Choice:** only `response_start.span_id`, since that is the only link Lingua makes. `tool_call` and `tool_result` span ids aren't checked. One warning per span id.
17. **Warnings: placement and wire format.** Where do derivation warnings go? **Choice:** into the existing `metadata.import_warnings` on the call span's row, joined with `; ` alongside that span's other warnings, with identical messages deduplicated. This needed a new warnings channel in `TrajectoryStream`.

### Minor (no customer-visible change)

1. **Input keys: `TRAJECTORY_METADATA_FIELDS`.** The constant doesn't exist in lingua on this base; it lives in the consuming app. **Choice:** nothing added in lingua. The keys reach the typed header (`SpanVoice`), so `voice_calls` stays header-only.
2. **Input keys: `voice.timeline_origin_unix_ms` and `voice.server_vad_silence_ms`.** These are parsed (bad types are import failures) but never used, because derivation runs on the call clock.
3. **Input keys: wrong JSON type for `voice.timeline_version` (for example `"1"`).** **Choice:** import failure `Invalid metadata.voice.timeline_version`, like other typed metadata, rather than "unsupported voice timeline version".
4. **Import: timeline rows.** Rows are recognized by `metadata["voice.timeline"]` alone. Span name, type and parent aren't checked.
5. **Import: identical retries.** "Identical" means equal `voice.timeline` objects and equal `voice.events`. Duplicate `seq` values across different chunks are kept, in stable chunk order, with no warning.
6. **§2 Selections: recordings with different `origin_unix_ms`.** Assumed to share one call clock, with no check.
7. **Recorded: a versioned call with no rows.** `events: []` and the in-progress warning.
8. **Speech.id doc comment.** Updated to "or its utterance id in a call timeline", and bindings regenerated (`Speech.ts`).
