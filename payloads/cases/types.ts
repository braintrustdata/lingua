import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import type { Content, GenerateContentConfig, Tool } from "@google/genai";
import type { ConverseCommandInput } from "@aws-sdk/client-bedrock-runtime";

// Google Gemini API request type (matching the js-genai library)
export interface GoogleGenerateContentRequest {
  contents: Content[];
  config?: GenerateContentConfig;
  tools?: Tool[];
  systemInstruction?: Content;
}

// Re-export Bedrock type for convenience
export type BedrockConverseRequest = ConverseCommandInput;

// Extended Anthropic type that includes beta/new features
// The executor uses regular client.messages.create() but test cases may define extra params
export type AnthropicMessageCreateParams =
  Anthropic.Messages.MessageCreateParams & {
    output_format?: Anthropic.Beta.Messages.BetaJSONOutputFormat | null;
    output_config?: Anthropic.Messages.OutputConfig;
  };

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
  "chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  responses: OpenAI.Responses.ResponseCreateParams | null;
  anthropic: AnthropicMessageCreateParams | null;
  google: GoogleGenerateContentRequest | null;
  bedrock: BedrockConverseRequest | null;
  "bedrock-anthropic"?: AnthropicMessageCreateParams | null;
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
] as const;
