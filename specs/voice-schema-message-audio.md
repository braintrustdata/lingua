# Voice schema option: message audio, Lingua mapping

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

This spec maps the `message-audio` SDK contract (braintrust-sdk-javascript `docs/voice-schema/message-audio/README.md`) into `Trajectory.voice_calls`.

**Implemented here:** the Trajectory types (`VoiceCall.messages`, `VoiceMessage` and its enums) and the regenerated TypeScript bindings. **Sketched:** import and building. Nothing populates `messages` yet, and existing #486 behavior is unchanged.

## Input keys

All keys are span `metadata`. Every key below must be added to `TRAJECTORY_METADATA_FIELDS` so headers carry them and the app's header query fetches them.

| Key | Span | Parsed into |
| --- | --- | --- |
| `audio.recordings` | call span | existing `LoggedRecording` (#486) |
| `voice.origin_unix_ms` | call span | `f64`, required when `voice.messages` is present |
| `voice.messages` | call span | `BTreeMap<String, LoggedVoiceMessage>` |
| `voice.context_message_ids` | LLM span | `Vec<Option<String>>`, optional |
| `voice.output_message_id` | LLM span | `String`, optional |

**Merge semantics.** The SDK writes each message's whole current entry on every update, and Braintrust's `mergeDicts` deep-merges objects and replaces arrays. Lingua therefore sees one merged entry per message, and **validates only that merged row**. It never sees or checks intermediate writes, and it does not check forward-only transitions; those are the SDK's responsibility.

Typed import structs, sketched for `processing/import/voice.rs`:

```rust
#[derive(Deserialize)]
struct LoggedVoiceMessage {
    role: LoggedRole,                       // "user" | "assistant"
    seq: u64,
    committed: bool,
    state: VoiceMessageState,
    text: Option<String>,
    text_status: VoiceTextStatus,
    text_source: Option<VoiceTextSource>,   // may be present without text
    generated_text: Option<String>,
    start_ms: Option<f64>,
    end_ms: Option<f64>,
    timing_source: Option<VoiceTimingSource>,
    end_of_turn_ms: Option<f64>,
    transcript_final_ms: Option<f64>,
    interrupted_at_ms: Option<f64>,
    provider_audio_end_ms: Option<f64>,
    selections: Vec<LoggedSelection>,       // existing #486 struct
    produced_by_span_id: Option<String>,
    // `turn_span_id` and `source_ids` are SDK debugging aids: accepted, not parsed or stored.
}
```

These extend `SpanVoice` with `origin_unix_ms`, `messages`, `context_message_ids` and `output_message_id`. `SpanVoice` is kept on the header whenever any of them is present.

## Building `VoiceCall.messages`

Sketch for `processing/trajectory/voice.rs`, run from the same `initialize` hook #486 uses:

1. **Call.** The call is the span with `audio.recordings` (as today) or with `voice.messages`. A call with messages but no recordings still gets a `VoiceCall` with empty `recordings`.
2. **Messages.** For each `(id, message)`:
   - `speaker`: `user` stays `User`, `assistant` becomes `Agent`.
   - Times: `*_time = origin + *_ms`, using `voice.origin_unix_ms`.
   - Copy `text`, `text_status`, `text_source`, `generated_text`, `state`, `committed`, `seq`, `timing_source` and `provider_audio_end_ms` unchanged.
   - Copy selections into `AudioSelection` (#486 validation applies: known recording and the same `recording_span_id`).
   - A user message may have several selections (merged speech fragments); the times are their outer bounds as logged.
3. **`step_ids`:**
   - **Agent:** `[produced_by_span_id]` when that span is a Conversation step in this trace. Otherwise empty, with a warning.
   - **User:** the earliest-starting Conversation LLM span whose `voice.context_message_ids` contains `id`. Ties go by `(start, exec_counter, id)`, which is the stream's order. If no span contains the id, `step_ids` is empty: uncommitted, still open, or never heard.
4. **`request_id`:** the `Turn.request_id` of the turn whose request or work contains the user's step. It is unset for agent messages and unheard user messages. This needs the collected turns, so it runs in `TrajectoryCollector::snapshot` rather than in `initialize`.
5. **Order:** sort by `start_time`, then `seq`. Messages without a `start_time` sort by `seq` after the latest earlier-`seq` message that has one.
6. **`speech`:** derived for backward compatibility. Each message with at least one selection becomes `Speech { id: message id, speaker, step_ids, selections, interrupted: state == Interrupted }`. #486's span-name and `contrib.livekit.interrupted` rules run only when a call has no `voice.messages`.

## Recorded vs inferred

- **Recorded:** everything the SDK logged in `voice.messages` except `turn_span_id` and `source_ids`, which are intentionally omitted. Times are converted to wall clock.
- **Inferred:**
  - user `step_ids` and `request_id`;
  - message order;
  - the derived `speech`.
- **Not stored; computed by the app:** overlaps, latencies (response latency, end-of-turn delay, transcript lag), and "transcript after reply" (`transcript_final_time` later than the user step's start). Lingua stores no derived flags.

## Failures and warnings

Errors go through `import_failures`, warnings through `import_warnings`, following the existing #486 style. Nothing falls back silently.

- **Failure** (on the merged row):
  - `voice.messages` present without `voice.origin_unix_ms`;
  - a malformed message: wrong types, unknown enum value, `text` present while `text_status` is `missing`/`not_captured`, `text` absent while `text_status` is `final`/`partial`, a selection on the wrong channel for the role, or a `committed: false` entry that is not complete user speech without text;
  - a selection that refers to a missing recording.
- **Warning:**
  - a call has `voice.messages` but no LLM span in the trace carries `voice.context_message_ids`, so user messages cannot be linked;
  - `produced_by_span_id` is not a Conversation span in the trace;
  - `voice.output_message_id` and `produced_by_span_id` disagree;
  - `voice.context_message_ids` length differs from the parsed input message count. This is checked only when the span has `input`; with content capture off there is none;
  - recording `timeline.origin_unix_ms` differs from `voice.origin_unix_ms` by more than 1 ms (unknown clock offset);
  - a committed, non-open user message is heard by no LLM span.

## How #22034 and #22036 consume it

- **No app change needed:** #22034 keeps reading `voice_calls[].recordings` and `speech`. #22036 keeps `speech.step_ids` and `getSpeechKeys` through the derived `speech`.
- **Optional app follow-ups:**
  - key play buttons by `VoiceMessage.id` with `request_id`, which removes the "user speech maps to the turn owning the step" lookup in `getSpeechKeys`;
  - show `generated_text` vs `text` for interrupted agent speech;
  - badge `text_status` `missing`/`not_captured` and `committed: false`;
  - compute and show the derived latencies.
- The app zod schema `app/utils/preprocessors/trajectory-schema.ts` needs `messages` added to its voice-call schema once the app validates `voice_calls`.

## Expected output for the cascaded example (abridged)

```json
{
  "id": "c-root",
  "recordings": [{ "id": "call-0000", "attachment_span_id": "c-root", "attachment_pointer": "/input/audio/call-0000", "start_time": "2026-10-12T…", "duration_ms": 13000 }],
  "messages": [
    { "id": "item_u1", "speaker": "user", "seq": 0, "committed": true, "state": "complete", "text_status": "final", "text_source": "stt",
      "timing_source": "vad_events", "step_ids": ["c-llm1"], "request_id": "c-ut1",
      "selections": [
        { "recording_id": "call-0000", "channel_index": 0, "start_offset_ms": 300, "end_offset_ms": 1250 },
        { "recording_id": "call-0000", "channel_index": 0, "start_offset_ms": 1650, "end_offset_ms": 2900 }
      ] },
    { "id": "item_a1", "speaker": "agent", "seq": 1, "state": "interrupted", "generated_text": "I have nine thirty or eleven on Friday. Which would you like?",
      "text": "I have nine thirty or eleven on Fri", "timing_source": "playout_events", "step_ids": ["c-llm2"],
      "selections": [{ "recording_id": "call-0000", "channel_index": 1, "start_offset_ms": 5300, "end_offset_ms": 7900 }] }
  ],
  "speech": ["derived: item_u1 → [c-llm1], item_a1 → [c-llm2] interrupted, item_u2 → [c-llm3], item_a2 → [c-llm4]"]
}
```

`request_id` depends on the turn builder. With #486's user-word takeover, `c-llm1` attaches to the `c-ut1` turn. In the realtime example, which has no user turn spans, it becomes `r-llm1`'s own turn.

## Implementation order

1. Parse the keys into `SpanVoice`, and add them to `TRAJECTORY_METADATA_FIELDS`.
2. Build messages and `step_ids` in `trajectory/voice.rs`, and derive `speech`.
3. Fill `request_id` in `snapshot`.
4. Add fixtures `voice-message-audio-cascaded.json` and `voice-message-audio-realtime.json`, built from the SDK examples, to `trajectory/fixtures`, and register them with `import_fixture!`.
