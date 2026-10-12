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
5. **Order:** sort by `start_time`, then `seq`. Gaps in `seq` are expected: the SDK never writes tool-only speech, which has no text and no audio. Messages without a `start_time` sort by `seq` after the latest earlier-`seq` message that has one.
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

The SDK's zod schema rejects every per-message field rule below, at both failure and warning level. Lingua is more lenient: it keeps messages that have only warning-level problems, so one bad field does not hide the rest of a call.

- **Failure** (on the merged row; the message is dropped):
  - `voice.messages` present without `voice.origin_unix_ms`;
  - a message that does not parse: wrong types or an unknown enum value;
  - a contradictory text state: `text` present while `text_status` is `missing`/`not_captured`, or absent while it is `final`/`partial`;
  - a selection on the wrong channel for the role, or one that refers to a missing recording;
  - commit and playback contradictions:
    - a `committed: false` user entry that is not complete or has text;
    - a `committed: false` agent entry that is not `unplayed`/`interrupted`;
    - an `unplayed` entry with selections or `interrupted_at_ms`, or one with no generated text (no `text` while `text_status` is not `not_captured`).
- **Warning** (the message is kept as logged):
  - a role-mismatched field: `generated_text`, `interrupted_at_ms`, `provider_audio_end_ms` or `produced_by_span_id` on a user message, or `end_of_turn_ms`/`transcript_final_ms` on an agent message;
  - a user message with state `interrupted` or `unplayed`;
  - an `interrupted` agent message without `interrupted_at_ms`;
  - `end_ms` earlier than `start_ms`;
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
    { "id": "item_a1", "speaker": "agent", "seq": 2, "committed": true, "state": "interrupted", "text_status": "final", "text_source": "playout_aligned", "generated_text": "I have nine thirty or eleven on Friday. Which would you like?",
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

## Implementation decisions

The builder in this PR was written from this spec by an engineer who did not write it. These are the points the spec left open and how the builder settles them. They are now part of the contract; "Essential" ones change what customers see.

### Essential (changes customer-visible output)

- **Input keys / `TRAJECTORY_METADATA_FIELDS`.** Q: the spec says to add the keys to `TRAJECTORY_METADATA_FIELDS`, but that constant does not exist on this base. Chose: parse the keys in `import/span.rs` `Metadata` only, like `audio.recordings`. If the app's header query uses an allow-list elsewhere, it must add the four keys, or `voice_calls` will lack messages in production.
- **Failures: missing `voice.origin_unix_ms`.** Q: "the message is dropped" is per message; what happens to the call? Chose: one failure on the call span, every message dropped, and the call keeps `messages: []` and empty `speech`. The #486 speech rules still do not run, because `voice.messages` is present.
- **Failures: wrong channel.** Q: which channel is right for each role? The Lingua spec does not say. Chose the SDK README rule: user is channel 0 and assistant is channel 1. `audio.recordings[].sources[].boundary` is not consulted. A selection with no `channel_index` counts as the wrong channel, so its message is dropped.
- **Failures: missing recording.** Q: what does "#486 validation, same `recording_span_id`" compare against? Chose: `recording_span_id` must equal the call span's own `span_id`, and `recording_id` must be in that span's `audio.recordings`. A call with messages but no recordings therefore drops every message that has selections.
- **`speech.interrupted` for user messages.** Q: is `interrupted` `false` or omitted for user speech? Chose the literal rule `interrupted: state == Interrupted` for every message, so derived user speech has `interrupted: false`. #486 omitted the field for user speech.
- **`speech` order.** Q: does derived `speech` keep #486's audio-start sort? Chose: `speech` follows the `messages` order (start time, then seq), with no re-sort by first selection.
- **`request_id` lookup.** Q: does "request or work contains the step" include `Turn.response_id`? Chose: match `request_id`, `response_id`, or any `work[].id`. With the example traces, both hearing steps are in `work`, so the fixtures do not exercise the response-only case.
- **Agent `step_ids` without `produced_by_span_id`.** Q: does "otherwise empty, with a warning" also warn when the field is absent, for example on cancelled or unplayed speech? Chose: absent gives empty with no warning. Present but not a Conversation span gives empty with a warning.
- **`step_ids` value.** Q: is a step id the span's `span_id` or its row `id`? Chose the row `id`, matching `Turn`/`WorkStep` ids. `produced_by_span_id` and `voice.context_message_ids` are resolved through `(root_span_id, span_id)`.

### Minor (diagnostics or internals only)

- **Where validation runs.** Per-message failures and warnings are produced at import (`import/span.rs`) and reported on the call span. Cross-span warnings are produced in `trajectory/voice.rs`. Warnings for one span are joined with "; " into one `import_warnings` entry, as existing span warnings are.
- **Unknown fields in a message.** Ignored, not rejected. The spec only names `turn_span_id` and `source_ids`.
- **Failure precedence.** A message that breaks several rules reports only the first: parse, text state, selections, commit, then unplayed.
- **No-hearer warning.** It fires only when the call has at least one message, not for an empty `voice.messages` map. When it fires, the per-message "heard by no LLM span" warning is suppressed, to avoid repeating the same cause.
- **Which spans can hear.** Only spans with the Conversation role count as hearers. An LLM span with `voice.context_message_ids` in another role, such as analysis, is ignored.
- **Unheard user warning.** It applies to `committed && state != open` only, so uncommitted `vad_*` entries never warn.
- **`output_message_id` vs `produced_by_span_id`.** This warns only when both sides exist and disagree. Both directions are checked, and the warning is reported on the LLM span. An `output_message_id` naming a message that is not in any call of the trace does not warn. A Conversation producer with no `output_message_id` does not warn.
- **`context_message_ids` length.** "Parsed input message count" is the number of `Message`s that import produced from `input`, system messages included. The check runs only when `input` is present on the imported row, so header-only imports skip it, and the stream reports it from the full row.
- **Clock-origin warning.** This is checked on the call span for every recording with a `timeline`, whenever `voice.origin_unix_ms` is present, even without `voice.messages`.
- **Time conversion.** `origin + ms` is truncated to microseconds, like #486's recording `start_time`.
- **Order tiebreak.** The sort key is `(start_time or the latest start among earlier-seq messages, seq)`, so untimed messages with no earlier timed message sort first.
