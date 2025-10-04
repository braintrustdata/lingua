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
 * Convert Map objects to plain objects recursively.
 * This is needed because serde-wasm-bindgen serializes serde_json::Map to JS Map
 * instead of plain objects.
 */
function convertMapsToObjects(value: unknown): unknown {
  if (value instanceof Map) {
    const obj: Record<string, unknown> = {};
    for (const [key, val] of value.entries()) {
      obj[key] = convertMapsToObjects(val);
    }
    return obj;
  }

  if (Array.isArray(value)) {
    return value.map(item => convertMapsToObjects(item));
  }

  if (value !== null && typeof value === 'object') {
    const obj: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      obj[key] = convertMapsToObjects(val);
    }
    return obj;
  }

  return value;
}

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
      // Convert any Map objects to plain objects
      return convertMapsToObjects(result) as U;
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
      // Convert any Map objects to plain objects
      return convertMapsToObjects(result) as U;
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
// Chat Completions API Conversions
// ============================================================================

/**
 * Convert array of Chat Completions messages to LLMIR Messages
 * @throws {ConversionError} If conversion fails
 */
export const chatCompletionsMessagesToLLMIR = createToLLMIRConverter<unknown[], Message[]>(
  wasm.chat_completions_messages_to_llmir,
  'Chat Completions'
);

/**
 * Convert array of LLMIR Messages to Chat Completions messages
 * @throws {ConversionError} If conversion fails
 */
export const llmirToChatCompletionsMessages = createFromLLMIRConverter<Message[], unknown[]>(
  wasm.llmir_to_chat_completions_messages,
  'Chat Completions'
);

// ============================================================================
// Responses API Conversions
// ============================================================================

/**
 * Convert array of Responses API messages to LLMIR Messages
 * @throws {ConversionError} If conversion fails
 */
export const responsesMessagesToLLMIR = createToLLMIRConverter<unknown[], Message[]>(
  wasm.responses_messages_to_llmir,
  'Responses'
);

/**
 * Convert array of LLMIR Messages to Responses API messages
 * @throws {ConversionError} If conversion fails
 */
export const llmirToResponsesMessages = createFromLLMIRConverter<Message[], unknown[]>(
  wasm.llmir_to_responses_messages,
  'Responses'
);

// ============================================================================
// Anthropic Conversions
// ============================================================================

/**
 * Convert array of Anthropic messages to LLMIR Messages
 * @throws {ConversionError} If conversion fails
 */
export const anthropicMessagesToLLMIR = createToLLMIRConverter<unknown[], Message[]>(
  wasm.anthropic_messages_to_llmir,
  'Anthropic'
);

/**
 * Convert array of LLMIR Messages to Anthropic messages
 * @throws {ConversionError} If conversion fails
 */
export const llmirToAnthropicMessages = createFromLLMIRConverter<Message[], unknown[]>(
  wasm.llmir_to_anthropic_messages,
  'Anthropic'
);

// ============================================================================
// Validation functions (Zod-style API)
// ============================================================================

/**
 * Validation result in Zod-style format
 */
export type ValidationResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { message: string } };

/**
 * Validate a JSON string as an OpenAI request
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateOpenAIRequest(json: string): ValidationResult<unknown> {
  try {
    const data = wasm.validate_openai_request(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as an OpenAI response
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateOpenAIResponse(json: string): ValidationResult<unknown> {
  try {
    const data = wasm.validate_openai_response(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as an Anthropic request
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateAnthropicRequest(json: string): ValidationResult<unknown> {
  try {
    const data = wasm.validate_anthropic_request(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as an Anthropic response
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateAnthropicResponse(json: string): ValidationResult<unknown> {
  try {
    const data = wasm.validate_anthropic_response(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}
