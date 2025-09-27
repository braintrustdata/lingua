import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  ANTHROPIC_MODEL,
} from "./models";
import { warn } from "console";

// Simple test cases - basic functionality testing
export const simpleCases: TestCaseCollection = {
  simpleRequest: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "What is the capital of France?",
        },
      ],
      reasoning_effort: "low",
    },

    "openai-responses": {
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
  },

  reasoningRequest: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content:
            "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
        },
      ],
    },

    "openai-responses": {
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
  },

  reasoningRequestTruncated: {
    "openai-chat-completions": {
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

    "openai-responses": {
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
  },

  toolCallRequest: {
    "openai-chat-completions": {
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
    "openai-responses": {
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
  },
};
