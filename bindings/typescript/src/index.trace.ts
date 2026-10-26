import initWasm, * as wasmModule from "@braintrust/lingua-wasm/browser/trace";
import { type InitInput } from "@braintrust/lingua-wasm/browser/trace";
import { createWasmRuntime } from "./create-wasm-runtime";
import { createMessageConverters } from "./message-converters";

const { getWasm, ensureOnce } = createWasmRuntime<typeof wasmModule>();

export async function init(module: InitInput): Promise<void> {
  return ensureOnce(async () => {
    await initWasm({ module_or_path: module });
    return wasmModule;
  });
}

export default init;
export { getWasm, ensureOnce };
export { ConversionError } from "./message-converters";
export const {
  chatCompletionsMessagesToLingua,
  linguaToChatCompletionsMessages,
  responsesMessagesToLingua,
  linguaToResponsesMessages,
  anthropicMessagesToLingua,
  linguaToAnthropicMessages,
  googleContentsToLingua,
  linguaToGoogleContents,
  deduplicateMessages,
  importMessagesFromSpans,
  importAndDeduplicateMessages,
} = createMessageConverters(getWasm);
export type * from "./types";
export { VERSION } from "./version";
