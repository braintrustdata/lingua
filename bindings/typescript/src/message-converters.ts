/**
 * Provider conversion functions using WASM
 *
 * These functions handle conversion between provider-specific formats
 * (OpenAI, Anthropic) and Lingua Message format.
 *
 * Uses direct object passing for maximum efficiency - no JSON serialization!
 * All functions throw ConversionError on failure instead of returning error objects.
 */

import { type Message } from "./generated/Message";
import { type ChatCompletionRequestMessage } from "./generated/openai/ChatCompletionRequestMessage";
import { type InputItem } from "./generated/openai/InputItem";
import { type InputMessage } from "./generated/anthropic/InputMessage";

type MessageWasm = Pick<
  typeof import("@braintrust/lingua-wasm"),
  | "chat_completions_messages_to_lingua"
  | "lingua_to_chat_completions_messages"
  | "responses_messages_to_lingua"
  | "lingua_to_responses_messages"
  | "anthropic_messages_to_lingua"
  | "lingua_to_anthropic_messages"
  | "google_contents_to_lingua"
  | "lingua_to_google_contents"
  | "deduplicate_messages"
  | "import_messages_from_spans"
  | "import_and_deduplicate_messages"
>;

type GoogleContent = Record<string, unknown>;
type ImportSpan = {
  input?: unknown;
  output?: unknown;
  metadata?: unknown;
  [key: string]: unknown;
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
 * Creates a converter function that transforms provider format to Lingua
 * @param wasmFn - The WASM function to call
 * @param provider - Provider name for error reporting
 * @returns A function that converts provider format to Lingua
 */
function createToLinguaConverter<TOutput extends Message | Message[]>(
  wasmFn: () => MessageWasm["chat_completions_messages_to_lingua"],
  provider: string
): (input: unknown) => TOutput {
  return (input: unknown): TOutput => {
    try {
      return wasmFn()(input);
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
  wasmFn: () => MessageWasm["lingua_to_chat_completions_messages"],
  provider: string
): <T = TOutput>(input: TInput) => T {
  return <T = TOutput>(input: TInput): T => {
    try {
      return wasmFn()(input);
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

export function createMessageConverters(getWasm: () => MessageWasm) {
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
  const chatCompletionsMessagesToLingua = createToLinguaConverter<Message[]>(
    () => getWasm().chat_completions_messages_to_lingua,
    "Chat Completions"
  );

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
  const linguaToChatCompletionsMessages = createFromLinguaConverter<
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
  const responsesMessagesToLingua = createToLinguaConverter<Message[]>(
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
   * import type { ResponseInputItem } from 'openai/resources/responses/responses'
   * const messages = linguaToResponsesMessages<ResponseInputItem[]>(lingua)
   *
   * @throws {ConversionError} If conversion fails
   */
  const linguaToResponsesMessages = createFromLinguaConverter<
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
  const anthropicMessagesToLingua = createToLinguaConverter<Message[]>(
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
  const linguaToAnthropicMessages = createFromLinguaConverter<
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
  const googleContentsToLingua = createToLinguaConverter<Message[]>(
    () => getWasm().google_contents_to_lingua,
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
  const linguaToGoogleContents = createFromLinguaConverter<
    Message[],
    GoogleContent[]
  >(() => getWasm().lingua_to_google_contents, "Google");

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
  function deduplicateMessages(messages: Message[]): Message[] {
    try {
      return getWasm().deduplicate_messages(messages);
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
  function importMessagesFromSpans(spans: ImportSpan[]): Message[] {
    try {
      return getWasm().import_messages_from_spans(spans);
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
  function importAndDeduplicateMessages(spans: ImportSpan[]): Message[] {
    try {
      return getWasm().import_and_deduplicate_messages(spans);
    } catch (error: unknown) {
      throw new ConversionError(
        "Failed to import and deduplicate messages from spans",
        undefined,
        undefined,
        error
      );
    }
  }

  return {
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
  };
}
