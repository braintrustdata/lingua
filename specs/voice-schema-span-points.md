# Voice schema option: span points (Lingua)

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-13.

This branch is stacked on the call-timeline branch (#490). The SDK contract is in braintrust-sdk-javascript, `docs/voice-schema/span-points/README.md`, on branch `kevchoi/voice-schema-span-points`.

Integrations log #490's events as **points** on the spans that observed them, instead of in `voice.timeline` rows:

- `metadata["voice.points"]` is a map from `seq` (decimal string) to an event without its `seq`.
- Owners are `user_turn`, `assistant_turn`, `llm_response`, tool spans and the call span.
- The call span sets `voice.points_version: 1`.

Lingua turns the points back into #490 events and reuses #490's speech builder.

## Status

- **Implemented:**
  - `voice.points_version` and `voice.points` import on every span;
  - `build_points` in `processing/trajectory/voice.rs`;
  - two changes to the shared `timeline_speech`;
  - `VoiceEventDetail::SpeechEnd.interrupted_by`;
  - fixtures built from the SDK examples (`span-points-cascaded-backchannel.json`, `span-points-realtime-missing-transcript.json`);
  - `span_points_diagnostics`.
- **Not done:** app allow-list keys (after #486 is rebased); wasm and TS tests. `voice.origin_unix_ms` is not parsed: like #490's `voice.timeline_origin_unix_ms`, nothing reads it yet.

## Input

<!-- prettier-ignore -->
```ts
// Any span. Kept on SpanVoice; the points are kept as JSON until their owner is known.
metadata["voice.points_version"]?: 1                 // call span only; selects this path
metadata["voice.points"]?: Record<Seq, PointJson>    // Seq = decimal int >= 0, unique per call
// PointJson = #490 VoiceEvent without `seq`, and without `span_id` on response_start / tool_call /
// tool_result and `name` on tool_call (the owner supplies them), plus optional
// speech_end.interrupted_by.
```

## Build

1. **Call.** The call is a span with `voice.points_version`. Each span with points belongs to its nearest such ancestor, including itself. Points with no versioned ancestor get the warning `voice points without a versioned call span`. Version 1 builds; any other version fails with `unsupported voice points version N`.
2. **Events.** For each owner, in (start, id) order, and each key:
   - **Key:** it must be a canonical decimal `u32`, otherwise warning `voice point key K is not a seq`.
   - **Duplicate `seq`:** identical content is skipped. Different content fails the call: `Conflicting voice point N`.
   - **Owner role:** `call` (the call span itself), `user_turn` / `assistant_turn` (task spans with those names), or the span type (`llm`, `tool`, …). A point on a role that cannot own its kind is kept, with warning `voice point N (kind) on a ROLE span`:

     | Kind | Allowed roles |
     | --- | --- |
     | `end_of_turn` | `user_turn` |
     | `response_start` | `llm` |
     | `tool_call`, `tool_result` | `tool` |
     | `truncate` | `assistant_turn` |
     | `call_end` | `call` |
     | everything else | `user_turn`, `assistant_turn`, `call` |

   - **Fields added:** `seq` from the key. `span_id` = the owner's `span_id` for `response_start`, `tool_call` and `tool_result`. `name` = the owner's name for `tool_call`.
   - **Parse:** the result is parsed as a `VoiceEvent`; a parse error is a warning, `invalid voice point N: …`.
3. **Order and completeness.** Events are sorted by `seq`. Gaps warn with `points gap before seq N` / `points gap after seq N`. No `call_end` warns `call points are in progress or the session crashed`; points with a higher `seq` than `call_end` warn `voice points after call_end`.
4. **Uncommitted caller speech.** A caller utterance whose `speech_start` is on a `user_turn` span that holds no `end_of_turn` was never committed. LiveKit skipped that turn, for example under non-interruptible speech.
5. **Speech.** `timeline_speech` from #490 builds `Speech`, with two changes that apply to both paths:
   - **Uncommitted utterances** from step 4 join no turn window and get empty `step_ids`, with no warning. The timeline path passes an empty set, so it is unchanged.
   - **Agent speech** links to the `response_start` with its `response_id` that started latest at or before the speech start, or to the first one when all started later. #490 took the first. That matters only when one handle runs several generations: LiveKit 1.9.2 cascaded tool replies rerun the reply task on the same handle (`agent_activity.ts:4391-4421`). #490's fixtures have one generation per handle and are unchanged.
6. **Output.** `VoiceCall.events` (sorted) and `VoiceCall.speech`. `user_turn` / `assistant_turn` selections are ignored for a points call, as for a timeline call.

## Fixture results

**Cascaded:** `ut-3` is talk-over under a non-interruptible reply.

| Speech | Steps | Selection (ch, ms) | `interrupted` |
| --- | --- | --- | --- |
| `u1` | llm-1, llm-2 | 0, 300–2900 | |
| `speech_1/0` | llm-2 | 1, 5300–7900 | true |
| `u2` | llm-3, llm-4 | 0, 7600–8500 | |
| `speech_2/0` | llm-4 | 1, 9900–12600 | false |
| `u3` | none | 0, 10800–11500 | |

Trajectory turns: `ut-1` → `llm-2`, `ut-2` → `llm-4`, and `ut-3` with no response. The events keep `interrupted_by: "u2"` on `speech_1/0` and none on `speech_2/0`.

**Realtime:**

- **Steps:**
  - `u1` → rt-1, rt-2;
  - `speech_2/0` → rt-2;
  - `u2` → rt-3, rt-4;
  - `speech_4/0` → rt-4.
- **Late points:** the late `u1` transcript and the close-time `missing` for `u2` sit on `user_turn` spans that had already ended. The import is the same for every batch size and span order tested (`check_fixture`, plus random span orders in `check_fixture_variants`). Those runs load complete rows: point-by-point merges are not exercised here, and they rely on Braintrust's keyed merge.

## Diagnostics test

`span_points_diagnostics` mutates the cascaded fixture:

| Mutation | Result |
| --- | --- |
| Drop the call span's points | Warning: `call points are in progress or the session crashed` |
| Drop `tool-1`'s points | Warning: `points gap after seq 5` |
| Move `response_start` 5 onto `at-1` | Warning: `voice point 5 (response_start) on a assistant_turn span` |
| Write `seq` 23 with a different `at_ms` on the call span | Failure: `Conflicting voice point 23` |
