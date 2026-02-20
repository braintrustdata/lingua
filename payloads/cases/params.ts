import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  OPENAI_NON_REASONING_MODEL,
  ANTHROPIC_MODEL,
  ANTHROPIC_OPUS_MODEL,
} from "./models";

// OpenAI Responses API and Chat Completions API parameter test cases
// Each test case exercises specific parameters with bidirectional mappings where possible
// Note: temperature, top_p, and logprobs are not supported with reasoning models (gpt-5-nano)
export const paramsCases: TestCaseCollection = {
  // === Reasoning Configuration ===

  reasoningSummaryParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL, // Must use reasoning model for reasoning_effort
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
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "medium" },
    },
    google: null,
    bedrock: null,
  },

  reasoningEffortLowParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL, // Must use reasoning model
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "low",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "low" },
    },
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "low" },
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },

  textFormatJsonSchemaWithDescriptionParam: {
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
          description: "Extract person information from text",
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
          description: "Extract person information from text",
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },

  textFormatTextParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say hello." }],
      response_format: { type: "text" },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Say hello." }],
      text: { format: { type: "text" } },
    },
    anthropic: null, // text is default, no explicit param needed
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Latest OpenAI news" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
        },
      ],
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Run Python" }],
      tools: [
        {
          type: "bash_20250124",
          name: "bash",
        },
      ],
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "tool", name: "get_weather" },
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "NYC and LA weather" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "auto", disable_parallel_tool_use: true },
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Hi" }],
      system: "Say OK",
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      service_tier: "auto",
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral" },
        },
      ],
      messages: [{ role: "user", content: "Say ok." }],
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      metadata: { user_id: "user-12345" },
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      metadata: { user_id: "hashed-user-id-abc123" },
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Count to 20." }],
      stop_sequences: ["10", "ten"],
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 500,
      messages: [{ role: "user", content: "Say ok." }],
    },
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

  // === Anthropic-Specific Parameters ===

  topKParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      top_k: 40,
    },
    google: null,
    bedrock: null,
  },

  streamParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      stream: true,
    },
    google: null,
    bedrock: null,
  },

  textEditorToolParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [{ type: "text_editor_20250124", name: "str_replace_editor" }],
    },
    google: null,
    bedrock: null,
  },

  textEditorToolV2Param: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [
        { type: "text_editor_20250429", name: "str_replace_based_edit_tool" },
      ],
    },
    google: null,
    bedrock: null,
  },

  textEditorToolV3Param: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [
        {
          type: "text_editor_20250728",
          name: "str_replace_based_edit_tool",
          max_characters: 10000,
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  webSearchToolAdvancedParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "AI news" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
          allowed_domains: ["wikipedia.org", "arxiv.org"],
          max_uses: 3,
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  webSearchUserLocationParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Local food" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
          user_location: {
            type: "approximate",
            city: "San Francisco",
            region: "California",
            country: "US",
            timezone: "America/Los_Angeles",
          },
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  toolChoiceAutoParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "auto" },
    },
    google: null,
    bedrock: null,
  },

  toolChoiceAnyParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "any" },
    },
    google: null,
    bedrock: null,
  },

  toolChoiceNoneParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "none" },
    },
    google: null,
    bedrock: null,
  },

  cacheControl5mParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral", ttl: "5m" },
        },
      ],
      messages: [{ role: "user", content: "Hi" }],
    },
    google: null,
    bedrock: null,
  },

  cacheControl1hParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral", ttl: "1h" },
        },
      ],
      messages: [{ role: "user", content: "Hi" }],
    },
    google: null,
    bedrock: null,
  },

  imageContentParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "image",
              source: {
                type: "base64",
                media_type: "image/png",
                data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
              },
            },
            { type: "text", text: "Describe." },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  documentContentParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "document",
              source: {
                type: "text",
                media_type: "text/plain",
                data: "Sample text.",
              },
              title: "Doc",
            },
            { type: "text", text: "Summarize." },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  thinkingDisabledParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "2+2?" }],
      thinking: { type: "disabled" },
    },
    google: null,
    bedrock: null,
  },

  // Anthropic thinking enabled with budget_tokens - exercises budget→effort conversion
  // with small max_tokens (1024). budget/max_tokens = 100% → high effort.
  thinkingEnabledParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL,
      messages: [{ role: "user", content: "Think hard about 2+2" }],
      reasoning_effort: "high",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Think hard about 2+2" }],
      reasoning: { effort: "high" },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Think hard about 2+2" }],
      thinking: { type: "enabled", budget_tokens: 1024 },
    },
    google: null,
    bedrock: null,
  },

  // === Output Config (structured output) ===

  outputFormatJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_format: {
        type: "json_schema",
        schema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
          additionalProperties: false,
        },
      },
    },
    google: null,
    bedrock: null,
  },

  outputConfigJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },

  outputConfigEffortWithJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        effort: "medium",
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },
};
