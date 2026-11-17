/**
 * Re-export message conversion and import functions from converters module
 */

import { getWasm } from "./wasm-runtime";

export function supportsForcedToolChoice(model: string): boolean {
  return getWasm().supports_forced_tool_choice(model);
}

export {
  // Error handling
  ConversionError,

  // Chat Completions API conversions
  chatCompletionsMessagesToLingua,
  linguaToChatCompletionsMessages,

  // Responses API conversions
  responsesMessagesToLingua,
  linguaToResponsesMessages,

  // Anthropic conversions
  anthropicMessagesToLingua,
  linguaToAnthropicMessages,

  // Processing functions
  deduplicateMessages,
  importMessagesFromSpans,
  importAndDeduplicateMessages,
} from "./converters";
