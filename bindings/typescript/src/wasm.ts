/**
 * Re-export conversion and validation functions from converters module
 */

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

  // Chat Completions validation
  validateChatCompletionsRequest,
  validateChatCompletionsResponse,

  // Responses API validation
  validateResponsesRequest,
  validateResponsesResponse,

  // OpenAI validation (deprecated - use Chat Completions or Responses instead)
  validateOpenAIRequest,
  validateOpenAIResponse,

  // Anthropic validation
  validateAnthropicRequest,
  validateAnthropicResponse,
} from "./converters";

// Re-export types
export type { ValidationResult } from "./converters";
