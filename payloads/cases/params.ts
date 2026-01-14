import { TestCaseCollection } from "./types";
import { OPENAI_RESPONSES_MODEL } from "./models";

// OpenAI Responses API parameter test cases
// Each test case exercises specific parameters from the Responses API
// Note: temperature, top_p, and logprobs are not supported with reasoning models (gpt-5-nano)
export const paramsCases: TestCaseCollection = {
  // === Reasoning Configuration ===

  reasoningSummaryParam: {
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
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
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      safety_identifier: "test-user",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },
};
