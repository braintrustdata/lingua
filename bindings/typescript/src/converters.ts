import { getWasm } from "./wasm-runtime";
import { ConversionError, createMessageConverters } from "./message-converters";

export { ConversionError };

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

export interface StreamSessionChunk {
  data: unknown;
  eventType?: string;
}

export interface TransformStreamSessionHandle {
  push(input: string): StreamSessionChunk[];
  finish(): StreamSessionChunk[];
  pushSse(input: string): string[];
  finishSse(): string[];
}

interface WasmTransformStreamSession {
  push(input: string): StreamSessionChunk[];
  finish(): StreamSessionChunk[];
  pushSse(input: string): string[];
  finishSse(): string[];
}

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

/**
 * Create a stateful streaming transformation session.
 *
 * The session owns target-provider sequencing rules that require context
 * across multiple chunks, such as Anthropic message_delta usage merging.
 */
export function createTransformStreamSession(
  targetFormat: string
): TransformStreamSessionHandle {
  try {
    const WasmSession = getWasm().TransformStreamSession as new (
      targetFormat: string
    ) => WasmTransformStreamSession;
    const session = new WasmSession(targetFormat);
    return {
      push(input: string) {
        return session.push(input);
      },
      finish() {
        return session.finish();
      },
      pushSse(input: string) {
        return session.pushSse(input);
      },
      finishSse() {
        return session.finishSse();
      },
    };
  } catch (error: unknown) {
    throw new ConversionError(
      `Failed to create stream transform session for ${targetFormat}`,
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
