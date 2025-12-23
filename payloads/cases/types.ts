import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
<<<<<<< HEAD
import type { ConverseCommandInput } from "@aws-sdk/client-bedrock-runtime";

// Bedrock request type (alias for clarity)
=======
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
>>>>>>> 2be3c6a (feat(adapter): add Google adapter and payload snapshots)
export type BedrockConverseRequest = ConverseCommandInput;

// Well-defined types for test cases
export interface TestCase {
  "chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  responses: OpenAI.Responses.ResponseCreateParams | null;
  anthropic: Anthropic.Messages.MessageCreateParams | null;
  google: GoogleGenerateContentRequest | null;
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
  "google",
  "bedrock",
] as const;
