/**
 * Re-export conversion functions from converters module
 */

export {
  // Error handling
  ConversionError,

  // OpenAI conversions
  openAIMessageToLLMIR,
  llmirToOpenAIMessage,
  openAIInputItemsToLLMIR,

  // Anthropic conversions
  anthropicMessageToLLMIR,
  llmirToAnthropicMessage,

  // Validation functions
  validateOpenAIRequest,
  validateOpenAIResponse,
  validateAnthropicRequest,
  validateAnthropicResponse,
} from './converters';

// Re-export types
export type { ValidationResult } from './converters';