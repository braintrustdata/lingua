# Voice speech spans in trajectories

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

This spec covers how Lingua builds `Trajectory.voice_calls` when integrations log one `speech` span per utterance with `metadata["voice.speech"]` under a call span with `metadata["voice.call"]`. The SDK-side contract and the example traces are in braintrust-sdk-javascript `docs/voice-schema/speech-spans/`. The fixtures here use the same rows.

## Input keys

| Key | Where | Read into |
| --- | --- | --- |
| `voice.call` | Call span | `SpanVoice.call: LoggedCall {framework, pipeline, origin_unix_ms}` (new) |
| `audio.recordings` | Call span, when audio is captured | `SpanVoice.recordings` (unchanged from #486) |
| `audio.selections` | Speech span, when audio is captured | `SpanVoice.selections` (unchanged; one range per fragment) |
| `voice.speech` | Speech span | `SpanVoice.speech: LoggedSpeech` (new, typed) |

`LoggedSpeech` holds:

- `speaker`, `turn_id?`, `state`, `timing_source`;
- `heard_by[]`, `produced_by?`, `interrupted_by?`;
- `played_ms?`, `provider_played_ms?`, `end_of_turn_ms?`;
- `transcript? {source, final, available_ms?, confidence?}`.

A span keeps its voice header when it logged any of these keys. Building speech therefore stays header-only, as in #486.

## Calls

A call is any span that logged `voice.call` or `audio.recordings`.

- **Clock:** `voice.call.origin_unix_ms` is the call clock, falling back to the first recording's `timeline.origin_unix_ms`.
- **No audio:** a call without audio has an empty `recordings` list.

## Speech

Each `voice.speech` span is one `Speech`.

**Utterance grain:** a user speech is one turn-level utterance whose VAD fragments are separate selections. An agent speech is one playout of a speech handle. Lingua does not merge or split speech spans.

**Recorded as logged:**

| Speech field | Source |
| --- | --- |
| `id` | The speech span's row id |
| `speaker`, `state`, `timing_source` | `voice.speech` |
| `transcript_source` | `voice.speech.transcript.source` |
| `interrupted_by` | The row id of `voice.speech.interrupted_by` |
| `selections` | `audio.selections`; empty without audio |
| `start_time`, `end_time` | The span bounds; the only timing when no audio was captured |
| `step_ids` | Row ids of `produced_by` (agent) or `heard_by` (user) |
| `end_of_turn_time` | Call origin + `end_of_turn_ms` |

**Inferred:**

- **The call:** the nearest ancestor, or the span itself, that is a call. The parent may be a turn or, for ignored speech and agent speech with no turn (greetings), the call span.
- **`interrupted`:** `true` for state `interrupted`, `false` for any other agent speech, absent for user speech.
- **`step_ids` fallback:** when no link was logged, the turn named by `turn_id` and its direct step children. Ignored speech without `turn_id` gets no steps.
- **Order:** by the earliest selection start, falling back to the span start.

**Agent text:** Lingua does not read text from speech spans. The SDK contract puts spoken text (`playback_sync`) in `output.text` and generated text in `planned_text`, so `transcript_source` on agent speech is `playback_sync` when spoken text exists, and `tts_input`/`model_output` when only the planned text was logged.

**Not used by Lingua:** `played_ms`, `provider_played_ms`, `transcript.final`, `transcript.available_ms` and `confidence`. They stay on the span.

## Precedence during rollout

`voice.speech` replaces #486's LiveKit name rules **per trace and speaker**:

- If a trace has any user speech span, LiveKit `user_turn` speech is skipped in that trace.
- If it has any agent speech span, `assistant_turn` speech is skipped.
- Traces without `voice.speech` are unchanged.

A `voice.speech` key that fails to parse is reported on its span and does not count toward precedence, so one malformed span cannot hide the LiveKit speech of that speaker. An integration can therefore emit speech spans for one speaker first. Once one speech span for a speaker is emitted, though, every utterance of that speaker in the trace must be a speech span.

## Failures

| Condition | Result |
| --- | --- |
| Malformed `voice.speech` or `voice.call`, including an unknown enum value | `Invalid metadata.voice.speech: …` (or `voice.call`) on the span; nothing is built from that key |
| A selection naming another call span or an unknown recording | Import failure `Speech refers to a missing recording`; the speech is skipped |
| A link (`heard_by`, `produced_by`, `interrupted_by`, `turn_id`) naming a span missing from the trace | Import failure `Speech refers to a missing span <id>`; that link is dropped and the speech is kept |
| No ancestor call span yet | Speech skipped silently (call still in progress, as in #486) |

Lingua does not check `timing_source: audio_samples` without selections, or `played_ms` ≤ 0. The SDK schema rejects both.

## Trajectory changes

`Speech` gains seven optional fields:

- `state: SpeechState` (`complete | interrupted | incomplete | ignored`)
- `interrupted_by: string`
- `transcript_source: TranscriptSource` (`stt | provider_transcription | tts_input | model_output | playback_sync`)
- `timing_source: TimingSource` (`audio_samples | vad | provider_vad | playout | span_clock`)
- `end_of_turn_time`
- `start_time`
- `end_time`

`Speaker` now derives `Hash`. All new fields are optional, so existing app code and older trajectories still parse.

## Fixtures and tests

| Fixture or test | Covers |
| --- | --- |
| `voice-speech-spans-cascaded-barge-in.json` | Cascaded call: tool calls, a two-fragment user utterance, a barge-in and an ignored backchannel parented to the call span |
| `voice-speech-spans-realtime-barge-in.json` | The realtime equivalent, with a missing transcript |
| `voice-speech-spans-cascaded-no-audio.json` | The cascaded call with `audio.*` removed: calls come from `voice.call`, speech has bounds and no selections |
| `speech_spans_replace_livekit_speech_per_speaker` | Adds one user speech span to a LiveKit fixture: user speech comes only from it, agent speech still from LiveKit names |

## App consumption

- #22034 and #22036 need no changes for calls with audio. They read `recordings`, `speech[].speaker`, `step_ids` and `selections`, which keep their meaning. Agent `step_ids` now name the producing LLM step directly.
- Calls without audio have empty `recordings`. The voice bar should hide itself; a timeline would use `start_time`/`end_time`.
- The app's zod `trajectory-schema.ts` should add the new optional fields before using them.
- After #486 is rebased onto main, add `voice.speech` and `voice.call` to `TRAJECTORY_METADATA_FIELDS`, or the browser stream never fetches them.

## Known gap

A realtime user utterance without a transcript does not start a new Trajectory turn: the turn builder needs user text. The realtime fixture records this as current behaviour, with both exchanges in one turn. A follow-up could let user speech with `heard_by` force a turn boundary at the named step.
