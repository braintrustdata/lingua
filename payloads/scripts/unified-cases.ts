import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// Unified test cases organized by case name
// Each case has multiple provider implementations to compare side by side
export const unifiedTestCases = {
  simpleRequest: {
    "openai-chat-completions": {
      model: "gpt-4o-mini",
      messages: [
        {
          role: "user" as const,
          content: "What is the capital of France? Please explain your reasoning.",
        },
      ],
      max_tokens: 150,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,

    "openai-responses": {
      model: "gpt-5-nano",
      reasoning: { effort: "low", summary: "auto" },
      input: [
        {
          role: "user" as const,
          content: "What is the capital of France? Please explain your reasoning.",
        },
      ],
      max_output_tokens: 200000,
    } as OpenAI.Responses.ResponseCreateParams,

    anthropic: {
      model: "claude-3-5-haiku-20241022",
      max_tokens: 150,
      messages: [
        {
          role: "user" as const,
          content: "What is the capital of France? Please explain your reasoning.",
        },
      ],
    } as Anthropic.Messages.MessageCreateParams,
  },

  reasoningRequest: {
    "openai-chat-completions": {
      model: "gpt-4o-mini",
      messages: [
        {
          role: "user" as const,
          content: "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
      max_tokens: 300,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,

    "openai-responses": {
      model: "gpt-5-nano",
      reasoning: { effort: "high" as const },
      input: [
        {
          role: "user" as const,
          content: "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
      max_output_tokens: 300,
    } as OpenAI.Responses.ResponseCreateParams,

    anthropic: {
      model: "claude-3-5-haiku-20241022",
      max_tokens: 300,
      messages: [
        {
          role: "user" as const,
          content: "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    } as Anthropic.Messages.MessageCreateParams,
  },

  reasoningWithOutput: {
    "openai-responses": {
      model: "gpt-5-nano",
      reasoning: { effort: "low" as const },
      input: [
        {
          role: "user" as const,
          content: "What color is the sky?",
        },
      ],
      max_output_tokens: 2000,
    } as OpenAI.Responses.ResponseCreateParams,
  },

  toolCallRequest: {
    "openai-chat-completions": {
      model: "gpt-4o-mini",
      messages: [
        {
          role: "user" as const,
          content: "What's the weather like in San Francisco?",
        },
      ],
      tools: [
        {
          type: "function" as const,
          function: {
            name: "get_weather",
            description: "Get the current weather for a location",
            parameters: {
              type: "object",
              properties: {
                location: {
                  type: "string",
                  description: "The city and state, e.g. San Francisco, CA",
                },
              },
              required: ["location"],
            },
          },
        },
      ],
      tool_choice: "auto" as const,
      max_tokens: 200,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,
  },
} as const;

// Helper to get all case names
export function getAllCaseNames(): string[] {
  return Object.keys(unifiedTestCases);
}

// Helper to get all provider types for a case
export function getProviderTypesForCase(caseName: string): string[] {
  const testCase = unifiedTestCases[caseName as keyof typeof unifiedTestCases];
  return testCase ? Object.keys(testCase) : [];
}

// Helper to get all provider types across all cases
export function getAllProviderTypes(): string[] {
  const providerTypes = new Set<string>();
  Object.values(unifiedTestCases).forEach(testCase => {
    Object.keys(testCase).forEach(providerType => {
      providerTypes.add(providerType);
    });
  });
  return Array.from(providerTypes).sort();
}

// Helper to get a specific case for a provider
export function getCaseForProvider(caseName: string, providerType: string): any {
  const testCase = unifiedTestCases[caseName as keyof typeof unifiedTestCases];
  return testCase?.[providerType as keyof typeof testCase];
}