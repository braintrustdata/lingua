import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// Well-defined types for test cases
export interface TestCase {
  "openai-chat-completions"?: OpenAI.Chat.Completions.ChatCompletionCreateParams;
  "openai-responses"?: OpenAI.Responses.ResponseCreateParams;
  "anthropic"?: Anthropic.Messages.MessageCreateParams;
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
  "anthropic"
] as const;