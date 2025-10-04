/**
 * Provider conversion functions using WASM
 *
 * These functions handle conversion between provider-specific formats
 * (OpenAI, Anthropic) and Lingua Message format.
 *
 * Uses direct object passing for maximum efficiency - no JSON serialization!
 * All functions throw ConversionError on failure instead of returning error objects.
 */

// @ts-ignore - WASM module types are generated
import * as wasm from '../wasm/lingua.js';
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
 * Creates a converter function that transforms provider format to Lingua
 * @param wasmFn - The WASM function to call
 * @param provider - Provider name for error reporting
 * @returns A function that converts provider format to Lingua
 */
function createToLinguaConverter<T, U extends Message | Message[]>(
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
        `Failed to convert ${provider} message to Lingua`,
        provider,
        'to_lingua',
        error
      );
    }
  };
}

/**
 * Creates a converter function that transforms Lingua to provider format
 * @param wasmFn - The WASM function to call
 * @param provider - Provider name for error reporting
 * @returns A function that converts Lingua to provider format
 */
function createFromLinguaConverter<T extends Message | Message[], U>(
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
        `Failed to convert Lingua to ${provider} format`,
        provider,
        'from_lingua',
        error
      );
    }
  };
}

// ============================================================================
// Chat Completions API Conversions
// ============================================================================

/**
 * Convert array of Chat Completions messages to Lingua Messages
 * @throws {ConversionError} If conversion fails
 */
export const chatCompletionsMessagesToLingua = createToLinguaConverter<unknown[], Message[]>(
  wasm.chat_completions_messages_to_llmir,
  'Chat Completions'
);

/**
 * Convert array of Lingua Messages to Chat Completions messages
 * @throws {ConversionError} If conversion fails
 */
export const linguaToChatCompletionsMessages = createFromLinguaConverter<Message[], unknown[]>(
  wasm.llmir_to_chat_completions_messages,
  'Chat Completions'
);

// ============================================================================
// Responses API Conversions
// ============================================================================

/**
 * Convert array of Responses API messages to Lingua Messages
 * @throws {ConversionError} If conversion fails
 */
export const responsesMessagesToLingua = createToLinguaConverter<unknown[], Message[]>(
  wasm.responses_messages_to_llmir,
  'Responses'
);

/**
 * Convert array of Lingua Messages to Responses API messages
 * @throws {ConversionError} If conversion fails
 */
export const linguaToResponsesMessages = createFromLinguaConverter<Message[], unknown[]>(
  wasm.llmir_to_responses_messages,
  'Responses'
);

// ============================================================================
// Anthropic Conversions
// ============================================================================

/**
 * Convert array of Anthropic messages to Lingua Messages
 * @throws {ConversionError} If conversion fails
 */
export const anthropicMessagesToLingua = createToLinguaConverter<unknown[], Message[]>(
  wasm.anthropic_messages_to_llmir,
  'Anthropic'
);

/**
 * Convert array of Lingua Messages to Anthropic messages
 * @throws {ConversionError} If conversion fails
 */
export const linguaToAnthropicMessages = createFromLinguaConverter<Message[], unknown[]>(
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
