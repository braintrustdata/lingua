import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// Well-defined types for test cases
export interface TestCase {
  "openai-chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  "openai-responses": OpenAI.Responses.ResponseCreateParams | null;
  anthropic: Anthropic.Messages.MessageCreateParams | null;
}

// Collection of test cases organized by name
export interface TestCaseCollection {
  [caseName: string]: TestCase;
}

// Provider type definitions
export type ProviderType = keyof TestCase;

export const PROVIDER_TYPES = [
  "openai-chat-completions",
  "openai-responses",
  "anthropic",
] as const;
