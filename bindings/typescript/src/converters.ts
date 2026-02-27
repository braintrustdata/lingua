/**
 * Provider conversion functions using WASM
 *
 * These functions handle conversion between provider-specific formats
 * (OpenAI, Anthropic) and Lingua Message format.
 *
 * Uses direct object passing for maximum efficiency - no JSON serialization!
 * All functions throw ConversionError on failure instead of returning error objects.
 */

import { getWasm } from "./wasm-runtime";
import type { Message } from "./generated/Message";
import type { ChatCompletionRequestMessage } from "./generated/openai/ChatCompletionRequestMessage";
import type { InputItem } from "./generated/openai/InputItem";
import type { InputMessage } from "./generated/anthropic/InputMessage";

type GoogleContent = Record<string, unknown>;
type ImportSpan = {
  input?: unknown;
  output?: unknown;
  metadata?: unknown;
  [key: string]: unknown;
};

type GoogleWasmExports = {
  google_contents_to_lingua: (value: unknown) => unknown;
  lingua_to_google_contents: (value: unknown) => unknown;
};

// ============================================================================
// Error handling
// ============================================================================

export class ConversionError extends Error {
  constructor(
    message: string,
    public readonly provider?: string,
    public readonly direction?: "to_lingua" | "from_lingua",
    public readonly cause?: unknown
  ) {
    super(message);
    this.name = "ConversionError";

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
    return value.map((item) => convertMapsToObjects(item));
  }

  if (value !== null && typeof value === "object") {
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
function createToLinguaConverter<TOutput extends Message | Message[]>(
  wasmFn: () => (value: unknown) => unknown,
  provider: string
): (input: unknown) => TOutput {
  return (input: unknown): TOutput => {
    try {
      const result = wasmFn()(input);
      // Convert any Map objects to plain objects
      return convertMapsToObjects(result) as TOutput;
    } catch (error: unknown) {
      throw new ConversionError(
        `Failed to convert ${provider} message to Lingua`,
        provider,
        "to_lingua",
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
function createFromLinguaConverter<TInput extends Message | Message[], TOutput>(
  wasmFn: () => (value: unknown) => unknown,
  provider: string
): <T = TOutput>(input: TInput) => T {
  return <T = TOutput>(input: TInput): T => {
    try {
      const result = wasmFn()(input);
      // Convert any Map objects to plain objects
      return convertMapsToObjects(result) as T;
    } catch (error: unknown) {
      throw new ConversionError(
        `Failed to convert Lingua to ${provider} format`,
        provider,
        "from_lingua",
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
 *
 * Returns messages in Lingua's universal format. Accepts messages from:
 * - Direct REST API responses
 * - OpenAI SDK (ChatCompletionMessage types)
 * - Any structurally compatible message format
 *
 * @example
 * const lingua = chatCompletionsMessagesToLingua(messages)
 *
 * @throws {ConversionError} If conversion fails
 */
export const chatCompletionsMessagesToLingua = createToLinguaConverter<
  Message[]
>(() => getWasm().chat_completions_messages_to_lingua, "Chat Completions");

/**
 * Convert array of Lingua Messages to Chat Completions messages
 *
 * Returns messages in Chat Completions format (OpenAI-compatible REST API).
 * By default, returns our generated types based on the OpenAPI spec.
 *
 * Use the generic parameter to specify your target SDK type:
 *
 * @example
 * // Default - returns ChatCompletionRequestMessage[]
 * const messages = linguaToChatCompletionsMessages(lingua)
 *
 * @example
 * // For OpenAI SDK
 * import type OpenAI from 'openai'
 * const messages = linguaToChatCompletionsMessages<OpenAI.Chat.ChatCompletionMessageParam[]>(lingua)
 *
 * @example
 * // For Vercel AI SDK
 * import type { CoreMessage } from 'ai'
 * const messages = linguaToChatCompletionsMessages<CoreMessage[]>(lingua)
 *
 * @throws {ConversionError} If conversion fails
 */
export const linguaToChatCompletionsMessages = createFromLinguaConverter<
  Message[],
  ChatCompletionRequestMessage[]
>(() => getWasm().lingua_to_chat_completions_messages, "Chat Completions");

// ============================================================================
// Responses API Conversions
// ============================================================================

/**
 * Convert array of Responses API messages to Lingua Messages
 *
 * Returns messages in Lingua's universal format. Accepts messages from:
 * - Direct Responses API responses
 * - OpenAI SDK (InputItem types)
 * - Any structurally compatible message format
 *
 * @example
 * const lingua = responsesMessagesToLingua(messages)
 *
 * @throws {ConversionError} If conversion fails
 */
export const responsesMessagesToLingua = createToLinguaConverter<Message[]>(
  () => getWasm().responses_messages_to_lingua,
  "Responses"
);

/**
 * Convert array of Lingua Messages to Responses API messages
 *
 * Returns messages in Responses API format (OpenAI's newer conversation API).
 * By default, returns our generated types based on the OpenAPI spec.
 *
 * Use the generic parameter to specify your target SDK type:
 *
 * @example
 * // Default - returns InputItem[]
 * const messages = linguaToResponsesMessages(lingua)
 *
 * @example
 * // For OpenAI SDK
 * import type OpenAI from 'openai'
 * const messages = linguaToResponsesMessages<OpenAI.Beta.Responses.InputItem[]>(lingua)
 *
 * @throws {ConversionError} If conversion fails
 */
export const linguaToResponsesMessages = createFromLinguaConverter<
  Message[],
  InputItem[]
>(() => getWasm().lingua_to_responses_messages, "Responses");

// ============================================================================
// Anthropic Conversions
// ============================================================================

/**
 * Convert array of Anthropic messages to Lingua Messages
 *
 * Returns messages in Lingua's universal format. Accepts messages from:
 * - Direct Anthropic API responses
 * - Anthropic SDK (MessageParam types)
 * - Any structurally compatible message format
 *
 * @example
 * const lingua = anthropicMessagesToLingua(messages)
 *
 * @throws {ConversionError} If conversion fails
 */
export const anthropicMessagesToLingua = createToLinguaConverter<Message[]>(
  () => getWasm().anthropic_messages_to_lingua,
  "Anthropic"
);

/**
 * Convert array of Lingua Messages to Anthropic messages
 *
 * Returns messages in Anthropic's Messages API format.
 * By default, returns our generated types based on the OpenAPI spec.
 *
 * Use the generic parameter to specify your target SDK type:
 *
 * @example
 * // Default - returns InputMessage[]
 * const messages = linguaToAnthropicMessages(lingua)
 *
 * @example
 * // For Anthropic SDK
 * import type Anthropic from '@anthropic-ai/sdk'
 * const messages = linguaToAnthropicMessages<Anthropic.MessageParam[]>(lingua)
 *
 * @throws {ConversionError} If conversion fails
 */
export const linguaToAnthropicMessages = createFromLinguaConverter<
  Message[],
  InputMessage[]
>(() => getWasm().lingua_to_anthropic_messages, "Anthropic");

// ============================================================================
// Google Conversions
// ============================================================================

/**
 * Convert array of Google Content items to Lingua Messages
 *
 * Returns messages in Lingua's universal format. Accepts contents from:
 * - Google GenerateContent requests (`contents` array)
 *
 * @example
 * const lingua = googleContentsToLingua(contents)
 *
 * @throws {ConversionError} If conversion fails
 */
export const googleContentsToLingua = createToLinguaConverter<Message[]>(
  () =>
    (getWasm() as unknown as GoogleWasmExports).google_contents_to_lingua,
  "Google"
);

/**
 * Convert array of Lingua Messages to Google Content items
 *
 * Returns contents in Google GenerateContent format.
 *
 * @example
 * const contents = linguaToGoogleContents(lingua)
 *
 * @throws {ConversionError} If conversion fails
 */
export const linguaToGoogleContents = createFromLinguaConverter<
  Message[],
  GoogleContent[]
>(
  () =>
    (getWasm() as unknown as GoogleWasmExports).lingua_to_google_contents,
  "Google"
);

// ============================================================================
// Processing functions
// ============================================================================

/**
 * Deduplicate messages based on role and content.
 *
 * Two messages are considered duplicates if:
 * - They have the same role
 * - Their content is semantically identical
 *
 * This handles equivalence between string and array content representations:
 * - `{"role": "user", "content": "foo"}` equals `{"role": "user", "content": [{"type": "text", "text": "foo"}]}`
 *
 * The function preserves the order of messages and keeps the first occurrence of each unique message.
 * Original messages are returned unmodified - hashing is only used for deduplication.
 *
 * @param messages - Array of Lingua messages to deduplicate
 * @returns Deduplicated array of messages
 * @throws {ConversionError} If processing fails
 */
export function deduplicateMessages(messages: Message[]): Message[] {
  try {
    const result = getWasm().deduplicate_messages(messages);
    // Convert any Map objects to plain objects
    return convertMapsToObjects(result) as Message[];
  } catch (error: unknown) {
    throw new ConversionError(
      "Failed to deduplicate messages",
      undefined,
      undefined,
      error
    );
  }
}

/**
 * Import messages from logging spans by parsing input/output fields
 *
 * This function accepts an array of span objects and extracts messages from their
 * input and output fields. It attempts to parse these fields in various provider formats
 * (Chat Completions, Responses API, Anthropic) and converts them to Lingua Messages.
 *
 * Only spans with successfully parsed input/output are included. Spans that don't contain
 * valid message data return [].
 *
 * @param spans - Array of span objects with optional input/output fields and additional span metadata
 * @returns Array of Lingua messages extracted from spans
 * @throws {ConversionError} If processing fails
 */
export function importMessagesFromSpans(
  spans: ImportSpan[]
): Message[] {
  try {
    const result = getWasm().import_messages_from_spans(spans);
    // Convert any Map objects to plain objects
    return convertMapsToObjects(result) as Message[];
  } catch (error: unknown) {
    throw new ConversionError(
      "Failed to import messages from spans",
      undefined,
      undefined,
      error
    );
  }
}

/**
 * Import and deduplicate messages from spans in a single operation
 *
 * Combines importMessagesFromSpans and deduplicateMessages for optimal performance
 * when processing span data. More efficient than calling the functions separately.
 *
 * @param spans - Array of span objects with optional input/output fields and additional span metadata
 * @returns Deduplicated array of Lingua messages extracted from spans
 * @throws {ConversionError} If processing fails
 */
export function importAndDeduplicateMessages(
  spans: ImportSpan[]
): Message[] {
  try {
    const result = getWasm().import_and_deduplicate_messages(spans);
    // Convert any Map objects to plain objects
    return convertMapsToObjects(result) as Message[];
  } catch (error: unknown) {
    throw new ConversionError(
      "Failed to import and deduplicate messages from spans",
      undefined,
      undefined,
      error
    );
  }
}

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
 * Validate a JSON string as a Chat Completions request
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateChatCompletionsRequest(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_chat_completions_request(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as a Chat Completions response
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateChatCompletionsResponse(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_chat_completions_response(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as a Responses API request
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateResponsesRequest(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_responses_request(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as a Responses API response
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateResponsesResponse(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_responses_response(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Validate a JSON string as an OpenAI request
 * @deprecated Use validateChatCompletionsRequest instead
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateOpenAIRequest(json: string): ValidationResult<unknown> {
  return validateChatCompletionsRequest(json);
}

/**
 * Validate a JSON string as an OpenAI response
 * @deprecated Use validateChatCompletionsResponse instead
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateOpenAIResponse(
  json: string
): ValidationResult<unknown> {
  return validateChatCompletionsResponse(json);
}

/**
 * Validate a JSON string as an Anthropic request
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateAnthropicRequest(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_anthropic_request(json);
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
export function validateAnthropicResponse(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_anthropic_response(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

// ============================================================================
// Stream chunk validation and transformation
// ============================================================================

/**
 * Validate a JSON string as a Chat Completions stream chunk
 * @returns Zod-style result: `{ ok: true, data: T }` or `{ ok: false, error: {...} }`
 */
export function validateChatCompletionsStreamChunk(
  json: string
): ValidationResult<unknown> {
  try {
    const data = getWasm().validate_chat_completions_stream_chunk(json);
    return { ok: true, data };
  } catch (error: unknown) {
    return {
      ok: false,
      error: { message: String(error) },
    };
  }
}

/**
 * Result from transforming a stream chunk
 */
export type TransformStreamChunkResult =
  | { passThrough: true; data: unknown }
  | { transformed: true; data: unknown; sourceFormat: string };

/**
 * Transform a streaming chunk from one provider format to another.
 *
 * Auto-detects the source format and transforms to the target format.
 *
 * @param input - JSON string of the stream chunk
 * @param targetFormat - Target provider format (e.g. "chat_completions", "anthropic")
 * @returns Object with either `{ passThrough: true, data }` or `{ transformed: true, data, sourceFormat }`
 * @throws {ConversionError} If transformation fails
 */
export function transformStreamChunk(
  input: string,
  targetFormat: string
): TransformStreamChunkResult {
  try {
    return getWasm().transform_stream_chunk(
      input,
      targetFormat
    ) as TransformStreamChunkResult;
  } catch (error: unknown) {
    throw new ConversionError(
      `Failed to transform stream chunk to ${targetFormat}`,
      undefined,
      undefined,
      error
    );
  }
}

// ============================================================================
// Type re-exports
// ============================================================================

export type { Message } from "./generated/Message";
export type { ChatCompletionRequestMessage } from "./generated/openai/ChatCompletionRequestMessage";
export type { InputItem } from "./generated/openai/InputItem";
export type { InputMessage } from "./generated/anthropic/InputMessage";
