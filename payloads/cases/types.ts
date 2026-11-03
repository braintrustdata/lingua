import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import type { Content, GenerateContentConfig, Tool } from "@google/genai";
import type { ConverseCommandInput } from "@aws-sdk/client-bedrock-runtime";

// Google Gemini API request type (matching the js-genai library)
export interface GoogleGenerateContentRequest {
  model?: string;
  contents: Content[];
  generationConfig?: GenerateContentConfig;
  tools?: Tool[];
  toolConfig?: Record<string, unknown>;
  systemInstruction?: Content;
}

// Re-export Bedrock type for convenience
export type BedrockConverseRequest = ConverseCommandInput;

// Extended Anthropic type that includes beta/new features.
// The executor uses regular client.messages.create() but test cases may define extra params.
export type AnthropicMessageCreateParams =
  Anthropic.Messages.MessageCreateParams & {
    output_format?: Anthropic.Beta.Messages.BetaJSONOutputFormat | null;
    output_config?: Anthropic.Messages.OutputConfig;
    context_management?: {
      edits: Array<
        | { type: "clear_tool_uses_20250919"; clear_tool_inputs?: boolean }
        | { type: "clear_thinking_20251015" }
      >;
    } | null;
  };

export type ChatCompletionTextPartWithCacheControl =
  OpenAI.Chat.Completions.ChatCompletionContentPartText & {
    cache_control: { type: "ephemeral"; ttl?: "5m" | "1h" };
  };

export type ChatCompletionUserMessageWithCacheControl = Omit<
  OpenAI.Chat.Completions.ChatCompletionUserMessageParam,
  "content"
> & {
  content: Array<
    | OpenAI.Chat.Completions.ChatCompletionContentPart
    | ChatCompletionTextPartWithCacheControl
  >;
};

export type ChatCompletionSystemMessageWithCacheControl = Omit<
  OpenAI.Chat.Completions.ChatCompletionSystemMessageParam,
  "content"
> & {
  content: Array<ChatCompletionTextPartWithCacheControl>;
};

export type ChatCompletionAssistantMessageWithCacheControl = Omit<
  OpenAI.Chat.Completions.ChatCompletionAssistantMessageParam,
  "content"
> & {
  content:
    | OpenAI.Chat.Completions.ChatCompletionAssistantMessageParam["content"]
    | Array<ChatCompletionTextPartWithCacheControl>;
};

export type ChatCompletionCreateParams = Omit<
  OpenAI.Chat.Completions.ChatCompletionCreateParams,
  "messages"
> & {
  messages: Array<
    | OpenAI.Chat.Completions.ChatCompletionMessageParam
    | ChatCompletionUserMessageWithCacheControl
    | ChatCompletionAssistantMessageWithCacheControl
    | ChatCompletionSystemMessageWithCacheControl
  >;
};

// The Responses API accepts replayed encrypted reasoning items without an ID,
// although the SDK's response-object type requires one.
type OpenAIResponseReasoningReplayItem = Omit<
  OpenAI.Responses.ResponseReasoningItem,
  "id"
> & {
  id?: string;
};

// Function call outputs carry the name of the tool that produced them, which
// the installed SDK's input-item type does not expose yet.
type OpenAIResponseFunctionCallOutputWithName =
  OpenAI.Responses.ResponseInputItem.FunctionCallOutput & {
    name?: string;
  };

type OpenAIResponseInputItem =
  | Exclude<
      OpenAI.Responses.ResponseInputItem,
      | OpenAI.Responses.ResponseReasoningItem
      | OpenAI.Responses.ResponseInputItem.FunctionCallOutput
    >
  | OpenAIResponseReasoningReplayItem
  | OpenAIResponseFunctionCallOutputWithName
  | OpenAI.Beta.Responses.BetaResponseInputItem.AgentMessage;

type OpenAIResponseCreateParamsWithExtendedServiceTier<T> = T extends unknown
  ? Omit<T, "input" | "service_tier"> & {
      input?: string | OpenAIResponseInputItem[];
      service_tier?:
        | "auto"
        | "default"
        | "flex"
        | "scale"
        | "priority"
        | "fast"
        | null;
    }
  : never;

export type OpenAIResponseCreateParams =
  OpenAIResponseCreateParamsWithExtendedServiceTier<OpenAI.Responses.ResponseCreateParams>;

// OpenAI Decisions API (POST /v1/decisions) payloads, following
// specs/openai/decisions.yml. The OpenAI SDK has no Decisions types yet.
type DecisionsInputText = { type: "input_text"; text: string };
type DecisionsOutputText = { type: "output_text"; text: string };
type DecisionsInputImage = {
  type: "input_image";
  image_url: string;
  detail?: "auto" | "low" | "high";
};

export type DecisionsInputItem =
  | {
      type?: "message";
      role: "user" | "system" | "developer";
      content: string | Array<DecisionsInputText | DecisionsInputImage>;
    }
  | {
      type?: "message";
      role: "assistant";
      content: string | DecisionsOutputText[];
    }
  | { type: "function_call"; call_id: string; name: string; arguments: string }
  | {
      type: "function_call_output";
      call_id: string;
      output: string | DecisionsInputText[];
    };

type DecisionsInstructions = string | DecisionsInputText[];
type DecisionsChoiceValue = string | boolean;

export type DecisionsQuestion =
  | { type: "predicate"; name?: string; instructions: DecisionsInstructions }
  | {
      type: "choice";
      name?: string;
      instructions: DecisionsInstructions;
      choices: Array<{ value: DecisionsChoiceValue; description?: string }>;
    }
  | {
      type: "score";
      name?: string;
      instructions: DecisionsInstructions;
      levels: Array<{ label: string; description?: string }>;
    };

export interface DecisionsRequest {
  model: string;
  input: string | DecisionsInputItem[];
  questions: DecisionsQuestion[];
  prompt_cache_key?: string | null;
  prompt_cache_options?: { mode?: "implicit" | "explicit"; ttl?: "30m" };
  safety_identifier?: string;
}

// TypeSafe System One API (POST /v1/systemone) payloads, following
// specs/typesafe/openapi.json. TypeSafe publishes no TypeScript SDK.
type SystemOneEntry = string | Record<string, unknown> | unknown[];

export type SystemOneQuestion =
  | {
      type: "noul";
      instructions?: SystemOneEntry;
      criteria?: { true?: SystemOneEntry; false?: SystemOneEntry };
    }
  | {
      type: "choice";
      instructions?: SystemOneEntry;
      criteria: Record<string, SystemOneEntry | null>;
    }
  | {
      type: "score";
      instructions?: SystemOneEntry;
      criteria: SystemOneEntry[];
    };

export interface SystemOneRequest {
  model: string;
  state: SystemOneEntry;
  questions: Record<string, SystemOneQuestion>;
}

// Expectation-based validation for proxy compatibility tests
// When present, capture.ts skips the case and validate.ts checks expectations
export interface TestExpectation {
  // Expected HTTP status code
  status?: number;
  // Expected field values using dot notation paths (e.g., "choices[0].logprobs")
  fields?: Record<string, unknown>;
  // Expected error response shape
  error?: {
    type?: string;
    message?: string;
  };
}

// Well-defined types for test cases
export interface TestCase {
  "chat-completions": ChatCompletionCreateParams | null;
  responses: OpenAIResponseCreateParams | null;
  anthropic: AnthropicMessageCreateParams | null;
  google: GoogleGenerateContentRequest | null;
  bedrock: BedrockConverseRequest | null;
  "bedrock-anthropic"?: AnthropicMessageCreateParams | null;
  "vertex-anthropic"?: AnthropicMessageCreateParams | null;
  "vertex-google"?: GoogleGenerateContentRequest | null;
  // Baseten serves OSS models via an OpenAI-compatible chat-completions API.
  baseten?: ChatCompletionCreateParams | null;
  // Decision models answer typed questions instead of generating text.
  decisions?: DecisionsRequest | null;
  typesafe?: SystemOneRequest | null;
  // Optional expectations for proxy compatibility tests
  expect?: TestExpectation;
}

// Collection of test cases organized by name
export interface TestCaseCollection {
  [caseName: string]: TestCase;
}

// Provider type definitions
export type ProviderType = keyof TestCase;

export const PROVIDER_TYPES = [
  "chat-completions",
  "responses",
  "anthropic",
  "google",
  "bedrock",
  "bedrock-anthropic",
  "vertex-anthropic",
  "vertex-google",
] as const;
