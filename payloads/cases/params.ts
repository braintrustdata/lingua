import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  OPENAI_NON_REASONING_MODEL,
} from "./models";

// OpenAI Responses API and Chat Completions API parameter test cases
// Each test case exercises specific parameters with bidirectional mappings where possible
// Note: temperature, top_p, and logprobs are not supported with reasoning models (gpt-5-nano)
export const paramsCases: TestCaseCollection = {
  // === Reasoning Configuration ===

  reasoningSummaryParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "2+2" }],
      reasoning: {
        effort: "medium",
        summary: "detailed",
      },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Text Response Configuration ===

  textFormatJsonObjectParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: 'Return {"status": "ok"} as JSON.' }],
      response_format: { type: "json_object" },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Return JSON with a=1" }],
      text: {
        format: {
          type: "json_object",
        },
      },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  textFormatJsonSchemaParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Extract: John is 25.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "person_info",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Name: John, Age: 25" }],
      text: {
        format: {
          type: "json_schema",
          name: "person_info",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Tool Configuration ===

  webSearchToolParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Latest OpenAI news" }],
      tools: [{ type: "web_search_preview" }],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  codeInterpreterToolParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Execute Python code to generate a random number",
        },
      ],
      tools: [{ type: "code_interpreter", container: { type: "auto" } }],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  toolChoiceRequiredParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
      tool_choice: { type: "function", function: { name: "get_weather" } },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get weather",
          strict: true,
          parameters: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
            additionalProperties: false,
          },
        },
      ],
      tool_choice: { type: "function", name: "get_weather" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  parallelToolCallsDisabledParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Weather in NYC and LA?" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
      parallel_tool_calls: false,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "NYC and LA weather" }],
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get weather",
          strict: true,
          parameters: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
            additionalProperties: false,
          },
        },
      ],
      parallel_tool_calls: false,
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Context & State Management ===

  instructionsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        { role: "system", content: "Always say ok." },
        { role: "user", content: "Hi" },
      ],
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      instructions: "Reply with OK",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  truncationAutoParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      truncation: "auto",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  storeDisabledParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      store: false,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      store: false,
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Caching & Performance ===

  serviceTierParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      service_tier: "default",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      service_tier: "default",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  promptCacheKeyParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      prompt_cache_key: "user-123-ml-explanation",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      prompt_cache_key: "test-key",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Metadata & Identification ===

  metadataParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      store: true,
      metadata: {
        request_id: "req-12345",
        user_tier: "premium",
        experiment: "control",
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      metadata: { key: "value" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  safetyIdentifierParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      safety_identifier: "hashed-user-id-abc123",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      safety_identifier: "test-user",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Sampling Parameters (require non-reasoning model) ===

  temperatureParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  topPParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  frequencyPenaltyParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      frequency_penalty: 0.5,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  presencePenaltyParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      presence_penalty: 0.5,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  logprobsParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "What is 2 + 2?" }],
      logprobs: true,
      top_logprobs: 2,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Output Control ===

  nMultipleCompletionsParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say a word." }],
      n: 2,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  stopSequencesParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Count from 1 to 20." }],
      stop: ["10", "ten"],
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  maxCompletionTokensParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      max_completion_tokens: 500,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Advanced Parameters ===

  predictionParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [
        {
          role: "user",
          content:
            "Update this function to add error handling:\n\nfunction divide(a, b) {\n  return a / b;\n}",
        },
      ],
      prediction: {
        type: "content",
        content:
          "function divide(a, b) {\n  if (b === 0) {\n    throw new Error('Cannot divide by zero');\n  }\n  return a / b;\n}",
      },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  seedParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Pick a number." }],
      seed: 12345,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  logitBiasParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hello." }],
      logit_bias: { "15339": -100 },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },
};
