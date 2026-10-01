import { getWasm } from "./wasm-runtime";
import type { VoiceCall } from "./generated/VoiceCall";

/** Read the full rows of one LiveKit trace. Times and recording ranges are in milliseconds. */
export function importVoiceCall(
  rows: readonly { span_id: string; [key: string]: unknown }[]
): VoiceCall {
  return getWasm().import_voice_call(rows) as VoiceCall;
}
