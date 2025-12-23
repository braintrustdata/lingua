import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import type { ConverseCommandInput } from "@aws-sdk/client-bedrock-runtime";

// Bedrock request type (alias for clarity)
export type BedrockConverseRequest = ConverseCommandInput;

// Well-defined types for test cases
export interface TestCase {
  "chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  responses: OpenAI.Responses.ResponseCreateParams | null;
  anthropic: Anthropic.Messages.MessageCreateParams | null;
  bedrock: BedrockConverseRequest | null;
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
  "bedrock",
] as const;
