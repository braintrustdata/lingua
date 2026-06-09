import {
  Type,
  ThinkingLevel,
  FunctionCallingConfigMode,
  Modality,
  MediaResolution,
} from "@google/genai";
import OpenAI from "openai";
import {
  ChatCompletionAssistantMessageWithCacheControl,
  ChatCompletionTextPartWithCacheControl,
  ChatCompletionUserMessageWithCacheControl,
  TestCase,
  TestCaseCollection,
} from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  OPENAI_REASONING_NONE_MODEL,
  OPENAI_NON_REASONING_MODEL,
  OPENAI_MINI_REASONING_MODEL,
  ANTHROPIC_MODEL,
  ANTHROPIC_FABLE_MODEL,
  ANTHROPIC_OPUS_MODEL,
  GOOGLE_MODEL,
  GOOGLE_GEMINI_3_MODEL,
  GOOGLE_IMAGE_MODEL,
  GOOGLE_TTS_MODEL,
  BEDROCK_MODEL,
} from "./models";

type ChatCompletionAssistantMessageWithReasoningSignature =
  OpenAI.Chat.Completions.ChatCompletionAssistantMessageParam & {
    reasoning_signature: string;
  };

const chatCompletionCacheControlTextPart = {
  type: "text",
  text: "Use this stable reference text as cacheable context.",
  cache_control: { type: "ephemeral", ttl: "1h" },
} satisfies ChatCompletionTextPartWithCacheControl;

const chatCompletionAssistantCacheControlTextPart = {
  type: "text",
  text: "This assistant prefill should remain cacheable.",
  cache_control: { type: "ephemeral", ttl: "1h" },
} satisfies ChatCompletionTextPartWithCacheControl;

const chatCompletionAssistantCacheControlMessage = {
  role: "assistant",
  content: [chatCompletionAssistantCacheControlTextPart],
} satisfies ChatCompletionAssistantMessageWithCacheControl;

const googleToolCallThoughtSignatureReplayAssistantMessage: ChatCompletionAssistantMessageWithReasoningSignature =
  {
    role: "assistant",
    content: null,
    reasoning_signature: "dGhvdWdodF9zaWduYXR1cmVfMTIz",
    tool_calls: [
      {
        id: "call_123",
        type: "function",
        function: {
          name: "list_collections",
          arguments: JSON.stringify({ database: "mydb" }),
        },
      },
    ],
  };

// OpenAI Responses API and Chat Completions API parameter test cases
// Each test case exercises specific parameters with bidirectional mappings where possible
// Note: temperature, top_p, and logprobs are not supported with reasoning models (gpt-5-nano)
export const paramsCases: TestCaseCollection = {
  bedrockDocumentCitationStreamingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: {
      modelId: BEDROCK_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              document: {
                name: "gateway-release-notes",
                format: "txt",
                source: {
                  bytes: new TextEncoder().encode(
                    "Braintrust Gateway supports OpenAI-compatible streaming over Bedrock Converse."
                  ),
                },
              },
            },
            {
              text: "Answer using the document and cite the source: what streaming route is supported?",
            },
          ],
        },
      ],
    },
  },

  bedrockGuardrailStopReasonStreamingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: {
      modelId: BEDROCK_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              text: "If a configured Bedrock guardrail intervenes, return the guardrail intervention response.",
            },
          ],
        },
      ],
      additionalModelResponseFieldPaths: ["/stop_sequence"],
    },
  },

  openaiPromptCacheKeyParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Summarize the cached policy." }],
      prompt_cache_key: "policy-cache-v1",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Summarize the cached policy." }],
      prompt_cache_key: "policy-cache-v1",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  chatCompletionsAnthropicCacheControlParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            chatCompletionCacheControlTextPart,
            {
              type: "text",
              text: "Now summarize it.",
            },
          ],
        } satisfies ChatCompletionUserMessageWithCacheControl,
      ],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: chatCompletionCacheControlTextPart.text,
              cache_control: { type: "ephemeral", ttl: "1h" },
            },
            {
              type: "text",
              text: "Now summarize it.",
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  chatCompletionsAssistantCacheControlParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        { role: "user", content: "Use the cached assistant prefill." },
        chatCompletionAssistantCacheControlMessage,
      ],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        { role: "user", content: "Use the cached assistant prefill." },
        {
          role: "assistant",
          content: [
            {
              type: "text",
              text: "This assistant prefill should remain cacheable.",
              cache_control: { type: "ephemeral", ttl: "1h" },
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

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
    google: {
      contents: [{ role: "user", parts: [{ text: "What is 2+2?" }] }],
      generationConfig: {
        thinkingConfig: {
          thinkingBudget: 10000,
          includeThoughts: true,
        },
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "What is 2+2?" }] }],
      generationConfig: {
        thinkingConfig: {
          thinkingBudget: 5000,
        },
      },
    },
    bedrock: null,
  },

  opus47AdaptiveThinkingReasoningEffortParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
      max_completion_tokens: 4096,
    },
    responses: null,
    anthropic: {
      model: "claude-opus-4-7",
      max_tokens: 4096,
      messages: [{ role: "user", content: "What is 2+2?" }],
      thinking: { type: "adaptive" },
      output_config: { effort: "medium" },
    },
    google: null,
    bedrock: null,
  },

  reasoningEffortMinimalParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "minimal",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "minimal" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  reasoningEffortNoneParam: {
    "chat-completions": {
      model: OPENAI_REASONING_NONE_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "none",
    },
    responses: {
      model: OPENAI_REASONING_NONE_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "none" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  reasoningEffortMaxClampsToGpt5NanoParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "max" },
    },
    google: null,
    bedrock: null,
  },

  responsesInputFileUrlParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "Analyze the letter and summarize the key points.",
            },
            {
              type: "input_file",
              file_url: "https://www.berkshirehathaway.com/letters/2024ltr.pdf",
            },
          ],
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesFunctionCallOutputWithoutThoughtSignatureParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "What databases exist in the connected MongoDB instance? Use the list_databases tool.",
            },
          ],
        },
        {
          type: "function_call",
          call_id: "6k7x6c84",
          name: "list_databases",
          arguments: "{}",
        },
        {
          type: "function_call_output",
          call_id: "6k7x6c84",
          output:
            '[{"type":"text","text":"{\\"databases\\":[\\"admin\\",\\"config\\",\\"local\\"]}"}]',
        },
      ],
      tools: [
        {
          type: "function",
          name: "list_databases",
          description: "List databases in the connected MongoDB instance.",
          parameters: {
            type: "object",
            properties: {},
            additionalProperties: false,
          },
          strict: false,
        },
      ],
      tool_choice: "auto",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  imageUrlMimeTypeFallbackParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "Describe this image.",
            },
            {
              type: "image_url",
              image_url: {
                url: "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
              },
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Describe this image." },
            {
              fileData: {
                fileUri:
                  "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
                mimeType: "image/jpeg",
              },
            },
          ],
        },
      ],
    },
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
    google: {
      contents: [
        { role: "user", parts: [{ text: 'Return {"status": "ok"} as JSON.' }] },
      ],
      generationConfig: {
        responseMimeType: "application/json",
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Extract: John is 25." }] }],
      generationConfig: {
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
        },
      },
    },
    bedrock: null,
  },

  textFormatJsonSchemaMissingRequiredPropertyParam: {
    "chat-completions": {
      model: OPENAI_MINI_REASONING_MODEL,
      messages: [
        {
          role: "user",
          content: "Return an answer and short reasoning.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "structured_response",
          schema: {
            type: "object",
            properties: {
              answer: { type: "string" },
              reasoning: { type: "string" },
            },
            required: ["answer"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_MINI_REASONING_MODEL,
      input: [
        {
          role: "user",
          content: "Return an answer and short reasoning.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "structured_response",
          schema: {
            type: "object",
            properties: {
              answer: { type: "string" },
              reasoning: { type: "string" },
            },
            required: ["answer"],
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Extract: John is 25." }] }],
      generationConfig: {
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
        },
      },
    },
    bedrock: null,
  },

  textFormatJsonSchemaNullableUnionTypeGpt54NanoParam: {
    "chat-completions": {
      model: "gpt-5.4-nano",
      messages: [
        {
          role: "user",
          content: "Classify the query and return JSON.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "query_result",
          strict: true,
          schema: {
            type: "object",
            properties: {
              explanation: {
                type: "string",
              },
              filter: {
                type: ["string", "null"],
              },
              match: {
                type: "boolean",
              },
            },
            required: ["explanation", "filter", "match"],
            additionalProperties: false,
          },
        },
      },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  googleResponseSchemaPropertyOrderingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Return JSON with keys gateway and score." }],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.OBJECT,
          properties: {
            gateway: { type: Type.STRING },
            score: { type: Type.INTEGER },
          },
          required: ["gateway", "score"],
          propertyOrdering: ["gateway", "score"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaPrefixItemsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        { role: "user", content: 'Return {"tuple": ["gateway", 7]} as JSON.' },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "tuple_response",
          schema: {
            type: "object",
            properties: {
              tuple: {
                type: "array",
                prefixItems: [{ type: "string" }, { type: "integer" }],
                items: { anyOf: [{ type: "string" }, { type: "integer" }] },
                minItems: 2,
                maxItems: 2,
              },
            },
            required: ["tuple"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        { role: "user", content: 'Return {"tuple": ["gateway", 7]} as JSON.' },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "tuple_response",
          schema: {
            type: "object",
            properties: {
              tuple: {
                type: "array",
                prefixItems: [{ type: "string" }, { type: "integer" }],
                items: { anyOf: [{ type: "string" }, { type: "integer" }] },
                minItems: 2,
                maxItems: 2,
              },
            },
            required: ["tuple"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: 'Return {"tuple": ["gateway", 7]} as JSON.' }],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            tuple: {
              type: "array",
              prefixItems: [{ type: "string" }, { type: "integer" }],
              items: { anyOf: [{ type: "string" }, { type: "integer" }] },
              minItems: 2,
              maxItems: 2,
            },
          },
          required: ["tuple"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaFormatParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "timestamp_response",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "timestamp_response",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
        },
      },
    },
    google: {
      contents: [
        {
          role: "user",
          parts: [
            {
              text: "Return JSON with an ISO 8601 timestamp in created_at.",
            },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            created_at: { type: "string", format: "date-time" },
          },
          required: ["created_at"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaMinMaxItemsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with tags as an array of 2 to 3 strings.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "tag_list",
          schema: {
            type: "object",
            properties: {
              tags: {
                type: "array",
                items: { type: "string" },
                minItems: 2,
                maxItems: 3,
              },
            },
            required: ["tags"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with tags as an array of 2 to 3 strings.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "tag_list",
          schema: {
            type: "object",
            properties: {
              tags: {
                type: "array",
                items: { type: "string" },
                minItems: 2,
                maxItems: 3,
              },
            },
            required: ["tags"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Return JSON with tags as an array of 2 to 3 strings." },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            tags: {
              type: "array",
              items: { type: "string" },
              minItems: 2,
              maxItems: 3,
            },
          },
          required: ["tags"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaMinimumMaximumParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with score as an integer from 0 to 10.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "bounded_score",
          schema: {
            type: "object",
            properties: {
              score: {
                type: "integer",
                minimum: 0,
                maximum: 10,
              },
            },
            required: ["score"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with score as an integer from 0 to 10.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "bounded_score",
          schema: {
            type: "object",
            properties: {
              score: {
                type: "integer",
                minimum: 0,
                maximum: 10,
              },
            },
            required: ["score"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Return JSON with score as an integer from 0 to 10." },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            score: {
              type: "integer",
              minimum: 0,
              maximum: 10,
            },
          },
          required: ["score"],
        },
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Latest OpenAI news" }] }],
      tools: [{ googleSearch: {} }],
    },
    bedrock: null,
  },

  // Provider-hosted code execution tools are not lossless analogues:
  // Responses code_interpreter is Python/container based, Anthropic bash is a
  // shell tool, and Google codeExecution is a Google-specific execution tool.
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
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Execute Python code to generate a random number" }],
        },
      ],
      tools: [{ codeExecution: {} }],
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Tokyo weather" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  location: { type: Type.STRING },
                },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
          allowedFunctionNames: ["get_weather"],
        },
      },
    },
    bedrock: null,
  },

  toolChoiceRequiredWithReasoningParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      reasoning_effort: "medium",
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
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_GEMINI_3_MODEL,
      contents: [{ role: "user", parts: [{ text: "Tokyo weather" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  location: { type: Type.STRING },
                },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
          allowedFunctionNames: ["get_weather"],
        },
      },
      generationConfig: {
        thinkingConfig: {
          thinkingLevel: ThinkingLevel.MEDIUM,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  // Reproduces: "Function tools with reasoning_effort are not supported for
  // gpt-5.4-mini in /v1/chat/completions. Please use /v1/responses instead."
  // The router should detect reasoning_effort + function tools and forward to
  // the responses endpoint rather than passing through to chat/completions.
  functionToolsWithReasoningEffortParam: {
    "chat-completions": {
      model: OPENAI_MINI_REASONING_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      reasoning_effort: "medium",
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
    },
    responses: {
      model: OPENAI_MINI_REASONING_MODEL,
      input: [{ role: "user", content: "Tokyo weather" }],
      reasoning: { effort: "medium" },
      tools: [
        {
          type: "function",
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
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  googleToolCallThoughtSignatureReplayParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "List the collections in the mydb database.",
        },
        googleToolCallThoughtSignatureReplayAssistantMessage,
        {
          role: "tool",
          tool_call_id: "call_123",
          content: JSON.stringify(["movies", "users"]),
        },
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "list_collections",
            description: "List the collections in a MongoDB database.",
            parameters: {
              type: "object",
              properties: {
                database: { type: "string" },
              },
              required: ["database"],
            },
          },
        },
      ],
      tool_choice: "auto",
    },
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "List the collections in the mydb database." }],
        },
        {
          role: "model",
          parts: [
            {
              functionCall: {
                name: "list_collections",
                args: { database: "mydb" },
              },
              thoughtSignature: "dGhvdWdodF9zaWduYXR1cmVfMTIz",
            },
          ],
        },
        {
          role: "user",
          parts: [
            {
              functionResponse: {
                name: "list_collections",
                response: { output: ["movies", "users"] },
              },
            },
          ],
        },
      ],
      tools: [
        {
          functionDeclarations: [
            {
              name: "list_collections",
              description: "List the collections in a MongoDB database.",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  database: { type: Type.STRING },
                },
                required: ["database"],
              },
            },
          ],
        },
      ],
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Hi" }] }],
      systemInstruction: { parts: [{ text: "Always say ok." }] },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        temperature: 0.7,
      },
    },
    bedrock: null,
  },

  fableTemperatureParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_FABLE_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        topP: 0.9,
      },
    },
    bedrock: null,
  },

  topPReasoningModelParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
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
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Count to 20." }],
      stop_sequences: ["10", "ten"],
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Count from 1 to 20." }] }],
      generationConfig: {
        stopSequences: ["10", "ten"],
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Say ok." }] }],
      generationConfig: {
        maxOutputTokens: 500,
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Pick a number." }] }],
      generationConfig: {
        seed: 12345,
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        topK: 40,
      },
    },
    bedrock: null,
  },

  googleOpenAIModelTopKParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Write a short sentence about API gateways." }],
        },
      ],
      generationConfig: {
        topK: 1,
        maxOutputTokens: 1024,
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "AI news" }] }],
      tools: [
        {
          googleSearch: {
            timeRangeFilter: {
              startTime: "2025-01-01T00:00:00Z",
              endTime: "2025-01-03T00:00:00Z",
            },
          },
        },
      ],
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.AUTO,
        },
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
        },
      },
    },
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
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.NONE,
        },
      },
    },
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

  // === Google-Specific Parameters ===

  thinkingLevelParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_GEMINI_3_MODEL,
      contents: [
        { role: "user", parts: [{ text: "Solve this complex problem." }] },
      ],
      generationConfig: {
        thinkingConfig: {
          thinkingLevel: ThinkingLevel.HIGH,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  toolModeValidatedParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather in Tokyo?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.VALIDATED,
        },
      },
    },
    bedrock: null,
  },

  thoughtSignatureParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  urlContextToolParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            {
              text: "Summarize https://ai.google.dev/gemini-api/docs/url-context and highlight the key constraints.",
            },
          ],
        },
      ],
      tools: [{ urlContext: {} }],
    },
    bedrock: null,
  },

  responseModalitiesAudioParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_TTS_MODEL,
      contents: [
        {
          role: "user",
          parts: [{ text: "Say hello in a warm, concise voice." }],
        },
      ],
      generationConfig: {
        responseModalities: [Modality.AUDIO],
      },
    },
    bedrock: null,
  },

  speechConfigParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_TTS_MODEL,
      contents: [
        {
          role: "user",
          parts: [
            {
              text: 'Generate audio speaking exactly this text: "Hello."',
            },
          ],
        },
      ],
      generationConfig: {
        responseModalities: [Modality.AUDIO],
        speechConfig: {
          voiceConfig: {
            prebuiltVoiceConfig: {
              voiceName: "Kore",
            },
          },
        },
      },
    },
    bedrock: null,
  },

  imageConfigParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_IMAGE_MODEL,
      contents: [
        { role: "user", parts: [{ text: "Generate a tiny red dot." }] },
      ],
      generationConfig: {
        responseModalities: [Modality.IMAGE],
        imageConfig: {
          aspectRatio: "1:1",
        },
      },
    },
    bedrock: null,
  },

  mediaResolutionParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        { role: "user", parts: [{ text: "Describe this image briefly." }] },
      ],
      generationConfig: {
        mediaResolution: MediaResolution.MEDIA_RESOLUTION_LOW,
      },
    },
    bedrock: null,
  },

  googleToolSchemaNumericInt64Param: (() => {
    const indexNameSchema: Record<string, unknown> = {
      type: Type.STRING,
      minLength: 1,
      maxLength: 128,
    };
    const tagsSchema: Record<string, unknown> = {
      type: Type.ARRAY,
      items: { type: Type.STRING },
      minItems: 1,
      maxItems: 3,
    };

    const testCase: TestCase = {
      "chat-completions": null,
      responses: null,
      anthropic: null,
      google: {
        model: GOOGLE_MODEL,
        contents: [
          { role: "user", parts: [{ text: "Validate tool schema bounds." }] },
        ],
        tools: [
          {
            functionDeclarations: [
              {
                name: "validate_bounds",
                description: "Validate bounded string and array inputs.",
                parameters: {
                  type: Type.OBJECT,
                  properties: {
                    index_name: indexNameSchema,
                    tags: tagsSchema,
                  },
                  required: ["index_name", "tags"],
                },
              },
            ],
          },
        ],
      },
      bedrock: null,
    };
    return testCase;
  })(),

  exclusiveMinimumToolParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Configure the LLM." }],
      tools: [
        {
          type: "function",
          function: {
            name: "configure_llm",
            description: "Configure LLM generation parameters",
            parameters: {
              type: "object",
              properties: {
                max_tokens: {
                  type: "number",
                  exclusiveMinimum: 0,
                  description: "Maximum number of tokens to generate",
                },
              },
              required: ["max_tokens"],
              additionalProperties: false,
            },
          },
        },
      ],
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Configure the LLM." }],
      tools: [
        {
          type: "function",
          name: "configure_llm",
          description: "Configure LLM generation parameters",
          parameters: {
            type: "object",
            properties: {
              max_tokens: {
                type: "number",
                exclusiveMinimum: 0,
                description: "Maximum number of tokens to generate",
              },
            },
            required: ["max_tokens"],
            additionalProperties: false,
          },
          strict: false,
        },
      ],
    },
    anthropic: null,
    google: (() => {
      // Assigned to a variable first so TypeScript applies structural (not
      // excess-property) checking when it lands in Record<string, Schema>.
      // exclusiveMinimum is not in Gemini's Schema type but IS passed here
      // deliberately to capture the resulting 400 INVALID_ARGUMENT error.
      const maxTokensSchema = {
        type: Type.NUMBER,
        exclusiveMinimum: 0,
        description: "Maximum number of tokens to generate",
      };
      return {
        model: GOOGLE_MODEL,
        contents: [{ role: "user", parts: [{ text: "Configure the LLM." }] }],
        tools: [
          {
            functionDeclarations: [
              {
                name: "configure_llm",
                description: "Configure LLM generation parameters",
                parameters: {
                  type: Type.OBJECT,
                  properties: { max_tokens: maxTokensSchema },
                  required: ["max_tokens"],
                },
              },
            ],
          },
        ],
      };
    })(),
    bedrock: null,
  },

  anthropicMessageWithSystemMessage: (() => {
    const testCase: TestCase = {
      "chat-completions": null,
      responses: null,
      anthropic: {
        model: "claude-opus-4-8",
        max_tokens: 32_000,
        system: [
          {
            type: "text",
            text: "You are running inside Claude Code.",
          },
          {
            type: "text",
            text: "Preserve the user's coding instructions.",
            cache_control: { type: "ephemeral" },
          },
        ],
        messages: [
          {
            role: "user",
            content: [
              {
                type: "text",
                text: "hello world",
                cache_control: { type: "ephemeral" },
              },
            ],
          },
          {
            role: "system",
            content: "Only use the exact tools provided by Claude Code.",
          },
        ],
        tools: [
          {
            name: "Read",
            description: "Reads a file from the local filesystem.",
            input_schema: {
              type: "object",
              properties: {
                file_path: {
                  type: "string",
                },
              },
              required: ["file_path"],
              additionalProperties: false,
            },
          },
        ],
        thinking: {
          type: "adaptive",
        },
        output_config: {
          effort: "high",
        },
      },
      google: null,
      bedrock: null,
    };

    return testCase;
  })(),
};
