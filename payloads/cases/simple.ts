import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  ANTHROPIC_MODEL,
} from "./models";
import { openai } from "@ai-sdk/openai";
import { tool } from "ai";
import { z } from "zod";

// Simple test cases - basic functionality testing
export const simpleCases: TestCaseCollection = {
  simpleRequest: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "What is the capital of France?",
        },
      ],
      reasoning_effort: "low",
    },

    responses: {
      model: OPENAI_RESPONSES_MODEL,
      reasoning: { effort: "minimal" },
      text: { verbosity: "low" },
      input: [
        {
          role: "user",
          content: "What is the capital of France?",
        },
      ],
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20_000,
      messages: [
        {
          role: "user",
          content: "What is the capital of France?",
        },
      ],
    },

    "ai-sdk": {
      model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
      messages: [
        {
          role: "user",
          content: "What is the capital of France?",
        },
      ],
    },
  },

  reasoningRequest: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },

    responses: {
      model: OPENAI_RESPONSES_MODEL,
      reasoning: { effort: "high" },
      input: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20_000,
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },

    "ai-sdk": {
      model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
      maxOutputTokens: 100,
    },
  },

  reasoningRequestTruncated: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      max_completion_tokens: 100,
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },

    responses: {
      model: OPENAI_RESPONSES_MODEL,
      max_output_tokens: 100,
      reasoning: { effort: "high" },
      input: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 100,
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },

    "ai-sdk": {
      model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
      maxOutputTokens: 100,
    },
  },

  toolCallRequest: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?",
        },
      ],
      tools: [
        {
          type: "function",
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
      tool_choice: "auto",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?",
        },
      ],
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get the current weather for a location",
          strict: true,
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
      ],
      tool_choice: "auto",
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20_000,
      messages: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?",
        },
      ],
      tools: [
        {
          name: "get_weather",
          description: "Get the current weather for a location",
          input_schema: {
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
      ],
    },

    "ai-sdk": {
      model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
      messages: [
        {
          role: "user",
          content: "What's the weather like in San Francisco?",
        },
      ],
      tools: {
        get_weather: tool({
          description: "Get the current weather for a location",
          inputSchema: z.object({
            location: z
              .string()
              .describe("The city and state, e.g. San Francisco, CA"),
          }),
        }),
      },
      toolChoice: "auto",
    },
  },
};
