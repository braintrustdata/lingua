import OpenAI from "openai";
import { Type } from "@google/genai";
import { GOOGLE_MODEL } from "../cases/models";
import { ProxyTestCaseCollection } from "./types";

const TEXT_BASE64 = "SGVsbG8gd29ybGQhCg==";
const AUDIO_BASE64 =
  "UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA=";
const VIDEO_BASE64 = "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDE=";
const PDF_BASE64 =
  "JVBERi0xLjQKMSAwIG9iago8PC9UeXBlL0NhdGFsb2cvUGFnZXMgMiAwIFI+PgplbmRvYmoKMiAwIG9iago8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PgplbmRvYmoKMyAwIG9iago8PC9UeXBlL1BhZ2UvTWVkaWFCb3hbMCAwIDYxMiA3OTJdL1BhcmVudCAyIDAgUi9SZXNvdXJjZXM8PD4+Pj4KZW5kb2JqCnhyZWYKMCA0CjAwMDAwMDAwMDAgNjU1MzUgZiAKMDAwMDAwMDAxNSAwMDAwMCBuIAowMDAwMDAwMDYxIDAwMDAwIG4gCjAwMDAwMDAxMTggMDAwMDAgbiAKdHJhaWxlcgo8PC9TaXplIDQvUm9vdCAxIDAgUj4+CnN0YXJ0eHJlZgoyMTUKJSVFT0YK";
const IMAGE_BASE64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
const MD_BASE64 = "IyBUaXRsZQoKVGhpcyBpcyBhIHBhcmFncmFwaC4K";
const CSV_BASE64 = "bmFtZSxhZ2UKQWxpY2UsMzAKQm9iLDI1Cg==";

export const proxyCases: ProxyTestCaseCollection = {
  proxyAnthropicBasic: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Tell me a short joke about programming." },
      ],
      max_tokens: 150,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
        object: "chat.completion",
      },
    },
  },

  proxyAnthropicReasoning: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-20250219",
      reasoning_effort: "medium",
      messages: [
        { role: "user", content: "How many rs in 'ferrocarril'" },
        {
          role: "assistant",
          content: "There are 4 letter 'r's in the word \"ferrocarril\".",
        },
        { role: "user", content: "How many e in what you said?" },
      ],
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyAnthropicToolCall: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [
        {
          role: "user",
          content:
            "Use the calculate function to add 2 and 3 together. Explain your reasoning in detail.",
        },
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "calculate",
            description: "Perform a mathematical calculation",
            parameters: {
              type: "object",
              properties: {
                operation: {
                  type: "string",
                  enum: ["add", "subtract", "multiply", "divide"],
                  description: "The operation to perform",
                },
                a: { type: "number", description: "First operand" },
                b: { type: "number", description: "Second operand" },
              },
              required: ["operation", "a", "b"],
            },
          },
        },
      ],
      tool_choice: "auto",
      max_tokens: 50,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyAnthropicPdfFile: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this PDF?" },
            {
              type: "image_url",
              image_url: { url: `data:application/pdf;base64,${PDF_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 200,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyAnthropicImageFile: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What color is this pixel?" },
            {
              type: "image_url",
              image_url: { url: `data:image/png;base64,${IMAGE_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyAnthropicStreaming: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Say hello in 3 words." },
      ],
      stream: true,
      max_tokens: 50,
    },
    expect: { status: 200 },
  },

  proxyAnthropicAudioError: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this audio?" },
            {
              type: "input_audio",
              input_audio: { data: AUDIO_BASE64, format: "wav" },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 400, error: { type: "invalid_request_error" } },
  },

  proxyAnthropicVideoError: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this video?" },
            {
              type: "image_url",
              image_url: { url: `data:video/mp4;base64,${VIDEO_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 400, error: { type: "invalid_request_error" } },
  },

  proxyAnthropicMaxTokensExceeds: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [{ role: "user", content: "Hello" }],
      max_tokens: 200000,
    },
    expect: { status: 400 },
  },

  proxyAnthropicReasoningDisabled: {
    format: "chat-completions",
    // Cast: reasoning_enabled is a Braintrust proxy extension
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    request: {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_enabled: false,
      max_tokens: 100,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.reasoning": { exists: false },
        "choices[0].message.role": "assistant",
      },
    },
  },

  proxyAnthropicJsonObject: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: "Return a JSON object with a greeting field.",
        },
      ],
      response_format: { type: "json_object" },
      max_tokens: 150,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  proxyAnthropicToolCallRequired: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Get the weather in San Francisco" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather",
            parameters: {
              type: "object",
              properties: {
                location: { type: "string", description: "City name" },
              },
              required: ["location"],
            },
          },
        },
      ],
      tool_choice: "required",
      max_tokens: 150,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].finish_reason": "tool_calls",
        "choices[0].message.tool_calls[0].type": "function",
      },
    },
  },

  proxyAnthropicPlainTextFile: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What does this text file say?" },
            {
              type: "image_url",
              image_url: { url: `data:text/plain;base64,${TEXT_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropicDefaultMaxTokens: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Say hi" }],
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyOpenAIReasoningDenied: {
    format: "chat-completions",
    request: {
      model: "gpt-4o-mini",
      messages: [{ role: "user", content: "Hello" }],
      reasoning_effort: "high",
      max_tokens: 50,
    },
    expect: {
      status: 400,
      error: {
        message: "Unrecognized request argument supplied: reasoning_effort",
      },
    },
  },

  proxyOpenAIO3MiniReasoning: {
    format: "chat-completions",
    request: {
      model: "o3-mini-2025-01-31",
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
      max_tokens: 1000,
    },
    expect: {
      status: 200,
      fields: { "choices[0].finish_reason": "stop", object: "chat.completion" },
    },
  },

  proxyGoogleBasic: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Say hello in exactly 3 words." },
      ],
      max_tokens: 50,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  proxyGoogleParamTranslation: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Count to 3." }],
      temperature: 0.7,
      top_p: 0.9,
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleToolCall: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "What's the weather in Tokyo?" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather in a location",
            parameters: {
              type: "object",
              properties: {
                location: { type: "string", description: "City name" },
              },
              required: ["location"],
            },
          },
        },
      ],
      tool_choice: "auto",
      max_tokens: 200,
    },
    expect: {
      status: 200,
      fields: { "choices[0].message.tool_calls[0].type": "function" },
    },
  },

  proxyGoogleReasoning: {
    format: "chat-completions",
    request: {
      model: "gemini-2.5-flash-preview-04-17",
      messages: [{ role: "user", content: "What is the square root of 144?" }],
      reasoning_effort: "medium",
      max_tokens: 500,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleImageContent: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you see in this image?" },
            {
              type: "image_url",
              image_url: { url: `data:image/png;base64,${IMAGE_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleAudioSupport: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you hear in this audio?" },
            {
              type: "input_audio",
              input_audio: { data: AUDIO_BASE64, format: "wav" },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleVideoSupport: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you see in this video?" },
            {
              type: "image_url",
              image_url: { url: `data:video/mp4;base64,${VIDEO_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleStopSequences: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropicMarkdownFile: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "What is the heading in this markdown file?",
            },
            {
              type: "image_url",
              image_url: { url: `data:text/markdown;base64,${MD_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropicCSVFile: {
    format: "chat-completions",
    request: {
      model: "claude-sonnet-4-5-20250929",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "How many rows are in this CSV file?" },
            {
              type: "image_url",
              image_url: { url: `data:text/csv;base64,${CSV_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropicToolCallSufficientTokens: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Get the weather in Paris" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather",
            parameters: {
              type: "object",
              properties: {
                location: { type: "string", description: "City name" },
              },
              required: ["location"],
            },
          },
        },
      ],
      tool_choice: "required",
      max_tokens: 500,
    },
    expect: {
      status: 200,
      fields: { "choices[0].finish_reason": "tool_calls" },
    },
  },

  proxyAnthropicStreamingReasoning: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "What is 15 * 17?" }],
      reasoning_effort: "low",
      stream: true,
      max_tokens: 2000,
    },
    expect: { status: 200 },
  },

  proxyAnthropicToolResultConversation: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [
        { role: "user", content: "What's the weather in London?" },
        {
          role: "assistant",
          content: null,
          tool_calls: [
            {
              id: "call_123",
              type: "function",
              function: {
                name: "get_weather",
                arguments: '{"location": "London"}',
              },
            },
          ],
        },
        {
          role: "tool",
          tool_call_id: "call_123",
          content: "Currently 15°C and cloudy in London.",
        },
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather",
            parameters: {
              type: "object",
              properties: {
                location: { type: "string", description: "City name" },
              },
              required: ["location"],
            },
          },
        },
      ],
      max_tokens: 200,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  proxyAnthropicStreamingToolCall: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Get weather in Berlin" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get the current weather",
            parameters: {
              type: "object",
              properties: {
                location: { type: "string", description: "City name" },
              },
              required: ["location"],
            },
          },
        },
      ],
      tool_choice: "required",
      stream: true,
      max_tokens: 200,
    },
    expect: { status: 200 },
  },

  proxyOpenAIPdfError: {
    format: "chat-completions",
    request: {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this PDF?" },
            {
              type: "image_url",
              image_url: { url: `data:application/pdf;base64,${PDF_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 400 },
  },

  proxyOpenAITextFileError: {
    format: "chat-completions",
    request: {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this text file?" },
            {
              type: "image_url",
              image_url: { url: `data:text/plain;base64,${TEXT_BASE64}` },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 400 },
  },

  proxyOpenAIStructuredOutput: {
    format: "chat-completions",
    request: {
      model: "gpt-4o",
      messages: [
        { role: "user", content: "What is 2+2? Answer with just the number." },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "math_result",
          schema: {
            type: "object",
            properties: { result: { type: "number" } },
            required: ["result"],
          },
        },
      },
      max_tokens: 50,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  proxyAzureParamFiltering: {
    format: "chat-completions",
    // Cast: reasoning_enabled/reasoning_budget are Braintrust proxy extensions
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    request: {
      model: "azure/gpt-4o",
      messages: [{ role: "user", content: "Hello" }],
      reasoning_enabled: true,
      reasoning_budget: 1000,
      max_tokens: 50,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyModelSpecificDefaults: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "Hi" }],
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropicStopSequences: {
    format: "chat-completions",
    request: {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyOpenAIStopSequences: {
    format: "chat-completions",
    request: {
      model: "gpt-4o-mini",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyGoogleJsonObjectFormat: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: "Return a JSON object with a greeting field set to hello.",
        },
      ],
      response_format: { type: "json_object" },
      max_tokens: 100,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  proxyGoogleJsonSchemaFormat: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: "What is 10 + 5? Answer with just the number.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "math_result",
          schema: {
            type: "object",
            properties: { result: { type: "number" } },
            required: ["result"],
          },
        },
      },
      max_tokens: 50,
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  proxyGoogleUnsupportedParamsFilter: {
    format: "chat-completions",
    request: {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Say hello." }],
      frequency_penalty: 0.5,
      presence_penalty: 0.5,
      max_tokens: 50,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyOpenAIPdfUrlConversion: {
    format: "chat-completions",
    request: {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What type of document is this?" },
            {
              type: "image_url",
              image_url: {
                url: "https://www.w3.org/WAI/WCAG21/Techniques/pdf/img/table-word.pdf",
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    expect: { status: 200, fields: { "choices[0].message.role": "assistant" } },
  },

  proxyAnthropic128kBetaHeader: {
    format: "chat-completions",
    request: {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: "Write a very short poem (2 lines) about coding.",
        },
      ],
    },
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
        object: "chat.completion",
      },
    },
  },

  proxyOpenAIO3MiniStreamingReasoning: {
    format: "chat-completions",
    request: {
      model: "o3-mini-2025-01-31",
      messages: [{ role: "user", content: "What is 7 * 8?" }],
      reasoning_effort: "medium",
      stream: true,
      max_tokens: 1000,
    },
    expect: { status: 200 },
  },

  proxyGoogleNativeBasic: {
    format: "google",
    request: {
      model: GOOGLE_MODEL,
      contents: [
        {
          role: "user",
          parts: [{ text: "Say 'hello' and nothing else." }],
        },
      ],
      config: { maxOutputTokens: 50 },
    },
    expect: {
      status: 200,
      fields: {
        "candidates[0].content.role": "model",
      },
    },
  },

  proxyGoogleNativeSystemInstruction: {
    format: "google",
    request: {
      model: GOOGLE_MODEL,
      contents: [
        {
          role: "user",
          parts: [{ text: "What is your role?" }],
        },
      ],
      systemInstruction: {
        parts: [{ text: "You are a helpful assistant." }],
      },
      config: { maxOutputTokens: 100 },
    },
    expect: {
      status: 200,
      fields: {
        "candidates[0].content.role": "model",
      },
    },
  },

  proxyGoogleNativeMultiTurn: {
    format: "google",
    request: {
      model: GOOGLE_MODEL,
      contents: [
        { role: "user", parts: [{ text: "My name is Alice." }] },
        {
          role: "model",
          parts: [{ text: "Hello Alice! Nice to meet you." }],
        },
        { role: "user", parts: [{ text: "What is my name?" }] },
      ],
      config: { maxOutputTokens: 50 },
    },
    expect: {
      status: 200,
      fields: {
        "candidates[0].content.role": "model",
      },
    },
  },

  proxyGoogleNativeToolCall: {
    format: "google",
    request: {
      model: GOOGLE_MODEL,
      contents: [
        {
          role: "user",
          parts: [{ text: "What is the weather in Paris?" }],
        },
      ],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get the current weather for a location",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  location: {
                    type: Type.STRING,
                    description: "City name",
                  },
                },
                required: ["location"],
              },
            },
          ],
        },
      ],
      config: { maxOutputTokens: 200 },
    },
    expect: {
      status: 200,
      fields: {
        "candidates[0].content.role": "model",
      },
    },
  },
};
