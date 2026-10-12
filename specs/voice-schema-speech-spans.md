# Voice speech spans in trajectories

> AI-created or modified and not human-reviewed in its current form; treat this artifact as provisional. Updated 2026-10-12.

This spec covers how Lingua builds `Trajectory.voice_calls` when integrations log one `speech` span per utterance with `metadata["voice.speech"]`. The SDK-side contract and the example traces are in braintrust-sdk-javascript `docs/voice-schema/speech-spans/`. The fixtures here use the same rows.

## Input keys

| Key | Where | Read into |
| --- | --- | --- |
| `audio.recordings` | Call span | `SpanVoice.recordings` (unchanged from #486; `timeline.origin_unix_ms` is now also used for `end_of_turn_time`) |
| `audio.selections` | Speech span | `SpanVoice.selections` (unchanged) |
| `voice.speech` | Speech span | `SpanVoice.speech: LoggedSpeech` (new, typed) |

`LoggedSpeech` holds:
- `speaker`, `turn_id?`, `state`, `timing_source`;
- `heard_by[]`, `produced_by?`, `interrupted_by?`;
- `played_ms?`, `provider_played_ms?`, `end_of_turn_ms?`;
- `transcript? {source, final, available_ms?, confidence?}`.

A span keeps its voice header when it logged recordings, selections or `voice.speech`. Building speech therefore stays header-only, as in #486.

## Recorded and inferred

**Recorded as logged:**

| Speech field | Source |
| --- | --- |
| `id` | The speech span's row id |
| `speaker`, `state`, `timing_source` | `voice.speech` |
| `transcript_source` | `voice.speech.transcript.source` |
| `interrupted_by` | The row id of `voice.speech.interrupted_by` |
| `selections` | `audio.selections` (may be empty when audio was omitted) |
| `step_ids` | Row ids of `produced_by` (agent) or `heard_by` (user) |
| `end_of_turn_time` | The call's recording origin + `end_of_turn_ms` |

**Inferred:**
- **The call:** the nearest ancestor, or the span itself, that logged recordings.
- **`interrupted`:** `true` for state `interrupted`, `false` for any other agent speech, absent for user speech.
- **`step_ids` fallback:** when no link was logged, the turn named by `turn_id` and its direct step children, as #486 does for turns.
- **Order:** by the earliest selection start, falling back to the span start. This is unchanged.

**Not used by Lingua:** `played_ms`, `provider_played_ms`, `transcript.final`, `transcript.available_ms` and `confidence`. They stay on the span for the span view.

**Precedence:** when any span in a trace logs `voice.speech`, the name-based LiveKit rules from #486 are skipped for that whole build. Traces without it are unchanged.

## Failures

| Condition | Result |
| --- | --- |
| Malformed `voice.speech`, including an unknown enum value | `Invalid metadata.voice.speech: …` on the span; no speech is built from it |
| A selection naming another call span or an unknown recording | Import failure `Speech refers to a missing recording`; the speech is skipped |
| A link (`heard_by`, `produced_by`, `interrupted_by`, `turn_id`) naming a span missing from the trace | Import failure `Speech refers to a missing span <id>`; that link is dropped and the speech is kept |
| No ancestor call span yet | Speech skipped silently (call still in progress, as in #486) |

## Trajectory changes

`Speech` gains five optional fields:
- `state: SpeechState` (`complete | interrupted | incomplete | ignored`)
- `interrupted_by: string`
- `transcript_source: TranscriptSource` (`stt | provider_transcription | tts_input | model_output | playback_sync`)
- `timing_source: TimingSource` (`audio_samples | vad | provider_vad | playout | span_clock`)
- `end_of_turn_time: string`

All five are optional, so existing app code and older trajectories still parse.

## App consumption

- #22034 and #22036 need no changes. They read `recordings`, `speech[].speaker`, `step_ids` and `selections`, which keep their meaning. Agent `step_ids` now name the producing LLM step directly instead of relying on start-time heuristics.
- The app's zod `trajectory-schema.ts` should add the five optional fields before using them.
- After #486 is rebased onto main, add `voice.speech` to `TRAJECTORY_METADATA_FIELDS`, or the browser stream never fetches it.

## Known gap

A realtime user utterance without a transcript does not start a new Trajectory turn: the turn builder needs user text. `voice-speech-spans-realtime-barge-in.json` records this as current behaviour, with both exchanges in one turn. A follow-up could let user speech with `heard_by` force a turn boundary at the named step.
