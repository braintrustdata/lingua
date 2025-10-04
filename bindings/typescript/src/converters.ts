/**
 * Provider conversion functions using WASM
 *
 * These functions handle conversion between provider-specific formats
 * (OpenAI, Anthropic) and LLMIR Message format.
 *
 * Uses direct object passing for maximum efficiency - no JSON serialization!
 * All functions throw ConversionError on failure instead of returning error objects.
 */

// @ts-ignore - WASM module types are generated
import * as wasm from '../wasm/llmir.js';
import type { Message } from './generated/Message';

// ============================================================================
// Error handling
// ============================================================================

export class ConversionError extends Error {
  constructor(
    message: string,
    public readonly provider?: string,
    public readonly direction?: 'to_llmir' | 'from_llmir',
    public readonly cause?: unknown
  ) {
    super(message);
    this.name = 'ConversionError';

    // Maintains proper stack trace for where our error was thrown (only available on V8)
    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, ConversionError);
    }
  }
}

// ============================================================================
// Generic converter factory
// ============================================================================

/**
 * Creates a converter function that transforms provider format to LLMIR
 * @param wasmFn - The WASM function to call
 * @param provider - Provider name for error reporting
 * @returns A function that converts provider format to LLMIR
 */
function createToLLMIRConverter<T, U extends Message | Message[]>(
  wasmFn: (value: unknown) => unknown,
  provider: string
): (input: T) => U {
  return (input: T): U => {
    try {
      const result = wasmFn(input);
      return result as U;
    } catch (error: unknown) {
      throw new ConversionError(
        `Failed to convert ${provider} message to LLMIR`,
        provider,
        'to_llmir',
        error
      );
    }
  };
}

/**
 * Creates a converter function that transforms LLMIR to provider format
 * @param wasmFn - The WASM function to call
 * @param provider - Provider name for error reporting
 * @returns A function that converts LLMIR to provider format
 */
function createFromLLMIRConverter<T extends Message | Message[], U>(
  wasmFn: (value: unknown) => unknown,
  provider: string
): (input: T) => U {
  return (input: T): U => {
    try {
      const result = wasmFn(input);
      return result as U;
    } catch (error: unknown) {
      throw new ConversionError(
        `Failed to convert LLMIR to ${provider} format`,
        provider,
        'from_llmir',
        error
      );
    }
  };
}

// ============================================================================
// OpenAI Conversions
// ============================================================================

/**
 * Convert OpenAI ChatCompletionRequestMessage to LLMIR Message
 * @throws {ConversionError} If conversion fails
 */
export const openAIMessageToLLMIR = createToLLMIRConverter<unknown, Message>(
  wasm.openai_message_to_llmir,
  'OpenAI'
);

/**
 * Convert LLMIR Message to OpenAI ChatCompletionRequestMessage
 * @throws {ConversionError} If conversion fails
 */
export const llmirToOpenAIMessage = createFromLLMIRConverter<Message, unknown>(
  wasm.llmir_to_openai_message,
  'OpenAI'
);

/**
 * Convert array of OpenAI InputItems to LLMIR Messages
 * @throws {ConversionError} If conversion fails
 */
export const openAIInputItemsToLLMIR = createToLLMIRConverter<unknown[], Message[]>(
  wasm.openai_input_items_to_llmir,
  'OpenAI'
);

/**
 * Convert array of LLMIR Messages to OpenAI InputItems
 * @throws {ConversionError} If conversion fails
 */
export const llmirToOpenAIInputItems = createFromLLMIRConverter<Message[], unknown[]>(
  wasm.llmir_to_openai_input_items,
  'OpenAI'
);

// ============================================================================
// Anthropic Conversions
// ============================================================================

/**
 * Convert Anthropic InputMessage to LLMIR Message
 * @throws {ConversionError} If conversion fails
 */
export const anthropicMessageToLLMIR = createToLLMIRConverter<unknown, Message>(
  wasm.anthropic_message_to_llmir,
  'Anthropic'
);

/**
 * Convert LLMIR Message to Anthropic InputMessage
 * @throws {ConversionError} If conversion fails
 */
export const llmirToAnthropicMessage = createFromLLMIRConverter<Message, unknown>(
  wasm.llmir_to_anthropic_message,
  'Anthropic'
);

/**
 * Convert array of LLMIR Messages to Anthropic InputMessages
 * @throws {ConversionError} If conversion fails
 */
export const llmirToAnthropicMessages = createFromLLMIRConverter<Message[], unknown[]>(
  wasm.llmir_to_anthropic_messages,
  'Anthropic'
);

// ============================================================================
// Validation
// ============================================================================

export type ValidationResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { message: string } };

/**
 * Generic validator factory
 * @param wasmFn - The WASM validation function to call
 * @param provider - Provider name for error reporting
 * @returns A function that validates JSON and returns ValidationResult
 */
function createValidator<T>(
  wasmFn: (json: string) => unknown,
  provider: string,
  type: 'request' | 'response'
): (json: string) => ValidationResult<T> {
  return (json: string): ValidationResult<T> => {
    try {
      const data = wasmFn(json) as T;
      return { ok: true, data };
    } catch (error: unknown) {
      return {
        ok: false,
        error: {
          message: `Failed to validate ${provider} ${type}: ${String(error)}`,
        },
      };
    }
  };
}

// ============================================================================
// OpenAI Validation
// ============================================================================

/**
 * Validate a JSON string as an OpenAI request
 * @param json - JSON string to validate
 * @returns ValidationResult with parsed data or error
 */
export const validateOpenAIRequest = createValidator<unknown>(
  wasm.validate_openai_request,
  'OpenAI',
  'request'
);

/**
 * Validate a JSON string as an OpenAI response
 * @param json - JSON string to validate
 * @returns ValidationResult with parsed data or error
 */
export const validateOpenAIResponse = createValidator<unknown>(
  wasm.validate_openai_response,
  'OpenAI',
  'response'
);

// ============================================================================
// Anthropic Validation
// ============================================================================

/**
 * Validate a JSON string as an Anthropic request
 * @param json - JSON string to validate
 * @returns ValidationResult with parsed data or error
 */
export const validateAnthropicRequest = createValidator<unknown>(
  wasm.validate_anthropic_request,
  'Anthropic',
  'request'
);

/**
 * Validate a JSON string as an Anthropic response
 * @param json - JSON string to validate
 * @returns ValidationResult with parsed data or error
 */
export const validateAnthropicResponse = createValidator<unknown>(
  wasm.validate_anthropic_response,
  'Anthropic',
  'response'
);

