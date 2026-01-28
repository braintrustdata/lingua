/**
 * Test cases ported from proxy integration tests.
 * These test OpenAI chat-completions compatibility with various providers.
 */

import OpenAI from "openai";
import { TestCaseCollection } from "./types";
import { ANTHROPIC_MODEL } from "./models";

// Text file: "Hello world!\n"
const TEXT_BASE64 = "SGVsbG8gd29ybGQhCg==";

// Minimal WAV header for audio error test (triggers unsupported media type)
const AUDIO_BASE64 =
  "UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQAAAAA=";

// Minimal MP4 for video error test (triggers unsupported media type)
const VIDEO_BASE64 = "AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDE=";

// Small valid PDF (minimal structure)
const PDF_BASE64 =
  "JVBERi0xLjQKMSAwIG9iago8PC9UeXBlL0NhdGFsb2cvUGFnZXMgMiAwIFI+PgplbmRvYmoKMiAwIG9iago8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PgplbmRvYmoKMyAwIG9iago8PC9UeXBlL1BhZ2UvTWVkaWFCb3hbMCAwIDYxMiA3OTJdL1BhcmVudCAyIDAgUi9SZXNvdXJjZXM8PD4+Pj4KZW5kb2JqCnhyZWYKMCA0CjAwMDAwMDAwMDAgNjU1MzUgZiAKMDAwMDAwMDAxNSAwMDAwMCBuIAowMDAwMDAwMDYxIDAwMDAwIG4gCjAwMDAwMDAxMTggMDAwMDAgbiAKdHJhaWxlcgo8PC9TaXplIDQvUm9vdCAxIDAgUj4+CnN0YXJ0eHJlZgoyMTUKJSVFT0YK";

// Small 1x1 PNG
const IMAGE_BASE64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

// Markdown file: "# Title\n\nThis is a paragraph.\n"
const MD_BASE64 = "IyBUaXRsZQoKVGhpcyBpcyBhIHBhcmFncmFwaC4K";

// CSV file: "name,age\nAlice,30\nBob,25\n"
const CSV_BASE64 = "bmFtZSxhZ2UKQWxpY2UsMzAKQm9iLDI1Cg==";

// Test cases ported from proxy/packages/proxy/src/providers/anthropic.test.ts
export const proxyCases: TestCaseCollection = {
  /**
   * Basic non-streaming request with system message.
   * Tests: Response format with logprobs field, finish_reason, usage.
   * From: anthropic.test.ts "should convert OpenAI non-streaming request to Anthropic and back"
   */
  proxyAnthropicBasic: {
    "chat-completions": {
      model: "claude-3-haiku-20240307",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Tell me a short joke about programming." },
      ],
      stream: false,
      max_tokens: 150,
    },
    responses: null, // Not testing responses API
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 150,
      system: "You are a helpful assistant.",
      messages: [
        { role: "user", content: "Tell me a short joke about programming." },
      ],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
        object: "chat.completion",
      },
    },
  },

  /**
   * Reasoning/thinking with multi-turn conversation.
   * Tests: reasoning_effort param, reasoning blocks in response.
   * From: anthropic.test.ts "should accept and return reasoning/thinking params"
   */
  proxyAnthropicReasoning: {
    "chat-completions": {
      model: "claude-3-7-sonnet-20250219",
      reasoning_effort: "medium",
      stream: false,
      messages: [
        {
          role: "user",
          content: "How many rs in 'ferrocarril'",
        },
        {
          role: "assistant",
          content: "There are 4 letter 'r's in the word \"ferrocarril\".",
        },
        {
          role: "user",
          content: "How many e in what you said?",
        },
      ],
    },
    responses: null,
    anthropic: {
      model: "claude-3-7-sonnet-20250219",
      max_tokens: 16000,
      messages: [
        {
          role: "user",
          content: "How many rs in 'ferrocarril'",
        },
        {
          role: "assistant",
          content: [
            {
              type: "thinking",
              thinking:
                "Let me count: f-e-r-r-o-c-a-r-r-i-l. The 'r' appears at positions 3, 4, 8, 9. So 4 total.",
              // Signature is required for thinking blocks
              signature: "thinking-signature-placeholder",
            },
            {
              type: "text",
              text: "There are 4 letter 'r's in the word \"ferrocarril\".",
            },
          ],
        },
        {
          role: "user",
          content: "How many e in what you said?",
        },
      ],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * Tool call with max_tokens causing truncation.
   * Tests: tool_calls in response, finish_reason handling.
   * From: anthropic.test.ts "should handle max_tokens stop reason correctly with tool calls"
   */
  proxyAnthropicToolCall: {
    "chat-completions": {
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
      max_tokens: 50, // Low to potentially cause truncation
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 50,
      messages: [
        {
          role: "user",
          content:
            "Use the calculate function to add 2 and 3 together. Explain your reasoning in detail.",
        },
      ],
      tools: [
        {
          name: "calculate",
          description: "Perform a mathematical calculation",
          input_schema: {
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
      ],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * PDF file content handling.
   * Tests: file content part conversion to Anthropic document format.
   * From: anthropic.test.ts "should handle file content parts with PDF data"
   */
  proxyAnthropicPdfFile: {
    "chat-completions": {
      model: "claude-3-5-sonnet-20241022",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this PDF?" },
            {
              // Using image_url with PDF data URL (Braintrust converts to document)
              type: "image_url",
              image_url: {
                url: `data:application/pdf;base64,${PDF_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 200,
    },
    responses: null,
    anthropic: {
      model: "claude-3-5-sonnet-20241022",
      max_tokens: 200,
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this PDF?" },
            {
              type: "document",
              source: {
                type: "base64",
                media_type: "application/pdf",
                data: PDF_BASE64,
              },
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * Image file content handling.
   * Tests: image content part handling.
   * From: anthropic.test.ts "should handle file content parts with image data"
   */
  proxyAnthropicImageFile: {
    "chat-completions": {
      model: "claude-3-5-sonnet-20241022",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What color is this pixel?" },
            {
              type: "image_url",
              image_url: {
                url: `data:image/png;base64,${IMAGE_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: {
      model: "claude-3-5-sonnet-20241022",
      max_tokens: 100,
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What color is this pixel?" },
            {
              type: "image",
              source: {
                type: "base64",
                media_type: "image/png",
                data: IMAGE_BASE64,
              },
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * Streaming request.
   * Tests: SSE event format, delta structure.
   * From: anthropic.test.ts "should convert OpenAI streaming request to Anthropic and back"
   */
  proxyAnthropicStreaming: {
    "chat-completions": {
      model: "claude-3-haiku-20240307",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Say hello in 3 words." },
      ],
      stream: true,
      max_tokens: 50,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 50,
      stream: true,
      system: "You are a helpful assistant.",
      messages: [{ role: "user", content: "Say hello in 3 words." }],
    },
    google: null,
    bedrock: null,
    expect: {
      status: 200,
    },
  },

  // ============================================================
  // Expectation-based tests (skip capture, validated by expectations)
  // ============================================================

  /**
   * Audio file error - Anthropic doesn't support audio input.
   * Tests: 400 error for unsupported media type.
   * From: anthropic.test.ts "should return 400 for unsupported audio file"
   */
  proxyAnthropicAudioError: {
    "chat-completions": {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this audio?" },
            {
              type: "input_audio",
              input_audio: {
                data: AUDIO_BASE64,
                format: "wav",
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
      error: { type: "invalid_request_error" },
    },
  },

  /**
   * Video file error - Anthropic doesn't support video input.
   * Tests: 400 error for unsupported media type.
   * From: anthropic.test.ts "should return 400 for unsupported video file"
   */
  proxyAnthropicVideoError: {
    "chat-completions": {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this video?" },
            {
              type: "image_url",
              image_url: {
                url: `data:video/mp4;base64,${VIDEO_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
      error: { type: "invalid_request_error" },
    },
  },

  /**
   * Max tokens exceeds model limit.
   * Tests: 400 error when max_tokens exceeds Anthropic's limit.
   * From: anthropic.test.ts "should return 400 when max_tokens exceeds limit"
   */
  proxyAnthropicMaxTokensExceeds: {
    "chat-completions": {
      model: "claude-sonnet-4-5-20250514",
      messages: [{ role: "user", content: "Hello" }],
      max_tokens: 200000, // Exceeds Anthropic's max
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
    },
  },

  /**
   * Reasoning disabled via reasoning_enabled: false.
   * Tests: Response should not contain reasoning block.
   * From: anthropic.test.ts "should disable reasoning when reasoning_enabled is false"
   */
  proxyAnthropicReasoningDisabled: {
    // Cast needed: reasoning_enabled is a Braintrust proxy extension
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    "chat-completions": {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_enabled: false,
      max_tokens: 100,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.reasoning": { exists: false },
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * JSON object response format.
   * Tests: response_format: json_object triggers tool-based workaround.
   * From: anthropic.test.ts "should handle json_object response format"
   */
  proxyAnthropicJsonObject: {
    "chat-completions": {
      model: "claude-3-haiku-20240307",
      messages: [
        {
          role: "user",
          content: "Return a JSON object with a greeting field.",
        },
      ],
      response_format: { type: "json_object" },
      max_tokens: 150,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  /**
   * Tool call with tool_choice: required.
   * Tests: finish_reason should be "tool_calls".
   * From: anthropic.test.ts "should handle tool_choice required"
   */
  proxyAnthropicToolCallRequired: {
    "chat-completions": {
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
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].finish_reason": "tool_calls",
        "choices[0].message.tool_calls[0].type": "function",
      },
    },
  },

  /**
   * Plain text file support.
   * Tests: text/plain files are properly handled.
   * From: anthropic.test.ts "should handle plain text file"
   */
  proxyAnthropicPlainTextFile: {
    "chat-completions": {
      model: "claude-3-5-sonnet-20241022",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What does this text file say?" },
            {
              type: "image_url",
              image_url: {
                url: `data:text/plain;base64,${TEXT_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Default max_tokens injection.
   * Tests: Request without max_tokens still works (proxy injects default).
   * From: anthropic.test.ts "should inject default max_tokens"
   */
  proxyAnthropicDefaultMaxTokens: {
    "chat-completions": {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Say hi" }],
      // Note: no max_tokens - proxy should inject default
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * OpenAI reasoning_effort on non-reasoning model.
   * Tests: gpt-4o-mini doesn't support reasoning_effort.
   * From: openai.test.ts "should reject reasoning_effort on non-reasoning model"
   */
  proxyOpenAIReasoningDenied: {
    "chat-completions": {
      model: "gpt-4o-mini",
      messages: [{ role: "user", content: "Hello" }],
      reasoning_effort: "high",
      max_tokens: 50,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
      error: {
        message: "Unrecognized request argument supplied: reasoning_effort",
      },
    },
  },

  /**
   * OpenAI o3-mini with reasoning_effort.
   * Tests: o3-mini supports reasoning_effort parameter.
   * From: openai.test.ts "should support reasoning_effort on o3-mini"
   */
  proxyOpenAIO3MiniReasoning: {
    "chat-completions": {
      model: "o3-mini-2025-01-31",
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
      max_tokens: 1000,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].finish_reason": "stop",
        object: "chat.completion",
      },
    },
  },

  // ============================================================
  // Google Provider Tests
  // ============================================================

  /**
   * Basic Google request translation.
   * Tests: OpenAI format → Google format via proxy.
   * From: google.test.ts basic request handling
   */
  proxyGoogleBasic: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: "Say hello in exactly 3 words." },
      ],
      max_tokens: 50,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        object: "chat.completion",
      },
    },
  },

  /**
   * Google parameter translation.
   * Tests: temperature, top_p, max_tokens → Google format.
   * From: google.params.test.ts parameter mapping
   */
  proxyGoogleParamTranslation: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Count to 3." }],
      temperature: 0.7,
      top_p: 0.9,
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Google tool calling.
   * Tests: OpenAI tools format → Google function declarations.
   * From: google.test.ts tool calling tests
   */
  proxyGoogleToolCall: {
    "chat-completions": {
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
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.tool_calls[0].type": "function",
      },
    },
  },

  /**
   * Google reasoning/thinking config.
   * Tests: reasoning_effort → thinkingConfig translation.
   * From: google.test.ts reasoning tests
   */
  proxyGoogleReasoning: {
    "chat-completions": {
      model: "gemini-2.5-flash-preview-04-17",
      messages: [{ role: "user", content: "What is the square root of 144?" }],
      reasoning_effort: "medium",
      max_tokens: 500,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Google image content support.
   * Tests: image_url handling for Google.
   * From: google.test.ts multimodal tests
   */
  proxyGoogleImageContent: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you see in this image?" },
            {
              type: "image_url",
              image_url: {
                url: `data:image/png;base64,${IMAGE_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Google audio support (Google DOES support audio unlike Anthropic).
   * Tests: audio content handling for Google.
   * From: google.test.ts audio support
   */
  proxyGoogleAudioSupport: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you hear in this audio?" },
            {
              type: "input_audio",
              input_audio: {
                data: AUDIO_BASE64,
                format: "wav",
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Google video support (Google DOES support video unlike Anthropic).
   * Tests: video content handling for Google.
   * From: google.test.ts video support
   */
  proxyGoogleVideoSupport: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What do you see in this video?" },
            {
              type: "image_url",
              image_url: {
                url: `data:video/mp4;base64,${VIDEO_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Google stop sequences.
   * Tests: stop sequences translation.
   * From: google.params.test.ts stop sequences
   */
  proxyGoogleStopSequences: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  // ============================================================
  // Additional Anthropic Tests
  // ============================================================

  /**
   * Markdown file support.
   * Tests: text/markdown files are properly handled.
   * From: anthropic.test.ts "should handle markdown file"
   */
  proxyAnthropicMarkdownFile: {
    "chat-completions": {
      model: "claude-3-5-sonnet-20241022",
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
              image_url: {
                url: `data:text/markdown;base64,${MD_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * CSV file support.
   * Tests: text/csv files are properly handled.
   * From: anthropic.test.ts "should handle CSV file"
   */
  proxyAnthropicCSVFile: {
    "chat-completions": {
      model: "claude-3-5-sonnet-20241022",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "How many rows are in this CSV file?" },
            {
              type: "image_url",
              image_url: {
                url: `data:text/csv;base64,${CSV_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Tool call with sufficient tokens.
   * Tests: finish_reason is "tool_calls" not "length" when tokens are sufficient.
   * From: anthropic.test.ts "should handle tool_use stop reason"
   */
  proxyAnthropicToolCallSufficientTokens: {
    "chat-completions": {
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
      max_tokens: 500, // Sufficient tokens - should not truncate
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].finish_reason": "tool_calls",
      },
    },
  },

  /**
   * Streaming with reasoning.
   * Tests: SSE events work correctly with reasoning enabled.
   * From: anthropic.test.ts "should stream reasoning content"
   */
  proxyAnthropicStreamingReasoning: {
    "chat-completions": {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "What is 15 * 17?" }],
      reasoning_effort: "low",
      stream: true,
      max_tokens: 2000,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
    },
  },

  /**
   * Multi-turn conversation with tool results.
   * Tests: Tool result handling in conversation flow.
   * From: anthropic.test.ts "should handle multi-turn with tool results"
   */
  proxyAnthropicToolResultConversation: {
    "chat-completions": {
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
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  /**
   * Streaming with tool calls.
   * Tests: SSE events work correctly with tool calling.
   * From: anthropic.test.ts "should stream tool calls"
   */
  proxyAnthropicStreamingToolCall: {
    "chat-completions": {
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
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
    },
  },

  // ============================================================
  // Additional OpenAI Tests
  // ============================================================

  /**
   * OpenAI PDF file handling.
   * Tests: OpenAI doesn't support PDFs in chat completions.
   * From: openai.test.ts "should reject PDF files"
   */
  proxyOpenAIPdfError: {
    "chat-completions": {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this PDF?" },
            {
              type: "image_url",
              image_url: {
                url: `data:application/pdf;base64,${PDF_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
    },
  },

  /**
   * OpenAI text file handling.
   * Tests: OpenAI doesn't support text files like Anthropic does.
   * From: openai.test.ts "should reject text files"
   */
  proxyOpenAITextFileError: {
    "chat-completions": {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What is in this text file?" },
            {
              type: "image_url",
              image_url: {
                url: `data:text/plain;base64,${TEXT_BASE64}`,
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 400,
    },
  },

  /**
   * OpenAI structured output with json_schema.
   * Tests: response_format with json_schema type.
   * From: openai.test.ts "should handle structured output"
   */
  proxyOpenAIStructuredOutput: {
    "chat-completions": {
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
            properties: {
              result: { type: "number" },
            },
            required: ["result"],
          },
        },
      },
      max_tokens: 50,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  /**
   * OpenAI reasoning_effort with null value.
   * Tests: null reasoning_effort should fallback to medium.
   * From: openai.test.ts "should fallback to medium when reasoning_effort is null"
   */
  proxyOpenAIReasoningEffortNull: {
    "chat-completions": {
      model: "o3-mini-2025-01-31",
      messages: [{ role: "user", content: "What is 5+5?" }],
      reasoning_effort: null,
      max_tokens: 500,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  // ============================================================
  // Cross-Provider Behavior Tests
  // ============================================================

  /**
   * Azure parameter filtering.
   * Tests: Braintrust-specific params are filtered for Azure.
   * From: azure.test.ts "should filter Braintrust parameters"
   */
  proxyAzureParamFiltering: {
    // Cast needed: reasoning_enabled/reasoning_budget are Braintrust proxy extensions
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    "chat-completions": {
      model: "azure/gpt-4o",
      messages: [{ role: "user", content: "Hello" }],
      reasoning_enabled: true,
      reasoning_budget: 1000,
      max_tokens: 50,
    } as OpenAI.Chat.Completions.ChatCompletionCreateParams,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Claude 3.7 model-specific max_tokens default.
   * Tests: Claude 3.7 gets 128k default with beta header.
   * From: anthropic.test.ts "should use model-specific max_tokens"
   */
  proxyModelSpecificDefaults: {
    "chat-completions": {
      model: "claude-3-7-sonnet-20250219",
      messages: [{ role: "user", content: "Hi" }],
      // No max_tokens - should get model-specific default
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Anthropic stop sequences.
   * Tests: stop sequences are properly translated.
   * From: anthropic.test.ts stop sequences handling
   */
  proxyAnthropicStopSequences: {
    "chat-completions": {
      model: "claude-3-haiku-20240307",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * OpenAI stop sequences consistency.
   * Tests: stop sequences work on native OpenAI.
   * From: schema tests for cross-provider consistency
   */
  proxyOpenAIStopSequences: {
    "chat-completions": {
      model: "gpt-4o-mini",
      messages: [{ role: "user", content: "Count from 1 to 10." }],
      stop: ["5", "END"],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  // ============================================================
  // Additional Missing Tests (from proxy test analysis)
  // ============================================================

  /**
   * Google response_format: json_object.
   * Tests: json_object → generationConfig.response_mime_type: "application/json".
   * From: google.params.test.ts "should translate json_object response format"
   */
  proxyGoogleJsonObjectFormat: {
    "chat-completions": {
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
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  /**
   * Google response_format: json_schema.
   * Tests: json_schema → response_mime_type + response_schema translation.
   * From: google.params.test.ts "should translate json_schema response format"
   */
  proxyGoogleJsonSchemaFormat: {
    "chat-completions": {
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
            properties: {
              result: { type: "number" },
            },
            required: ["result"],
          },
        },
      },
      max_tokens: 50,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
      },
    },
  },

  /**
   * Google unsupported parameter filtering.
   * Tests: frequency_penalty, presence_penalty are filtered (not sent to Google).
   * From: google.params.test.ts "should filter unsupported parameters"
   */
  proxyGoogleUnsupportedParamsFilter: {
    "chat-completions": {
      model: "gemini-2.0-flash",
      messages: [{ role: "user", content: "Say hello." }],
      frequency_penalty: 0.5, // Google doesn't support this
      presence_penalty: 0.5, // Google doesn't support this
      max_tokens: 50,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * OpenAI PDF URL conversion.
   * Tests: Remote PDF URL (in image_url format) → proxy fetches and converts to file block.
   * From: openai.test.ts "should convert PDF file URL to file block"
   * Note: Proxy detects .pdf extension and converts to native file format.
   */
  proxyOpenAIPdfUrlConversion: {
    "chat-completions": {
      model: "gpt-4o",
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "What type of document is this?" },
            {
              // Proxy detects PDF URLs in image_url and converts to file block
              type: "image_url",
              image_url: {
                // Using a small, publicly available PDF
                url: "https://www.w3.org/WAI/WCAG21/Techniques/pdf/img/table-word.pdf",
              },
            },
          ],
        },
      ],
      max_tokens: 100,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
      },
    },
  },

  /**
   * Anthropic claude-3-7 128k output beta header.
   * Tests: claude-3-7 without max_tokens gets 128k default and beta header.
   * From: anthropic.test.ts "should use 128k max_tokens and beta header for claude-3-7"
   * Note: Different from proxyModelSpecificDefaults - this tests with high output expectation.
   */
  proxyAnthropic128kBetaHeader: {
    "chat-completions": {
      model: "claude-3-7-sonnet-latest",
      messages: [
        {
          role: "user",
          content: "Write a very short poem (2 lines) about coding.",
        },
      ],
      // No max_tokens - proxy should inject 128000 and add beta header
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
      fields: {
        "choices[0].message.role": "assistant",
        "choices[0].finish_reason": "stop",
        object: "chat.completion",
      },
    },
  },

  /**
   * OpenAI o3-mini streaming with reasoning.
   * Tests: o3-mini streaming returns reasoning content in delta.
   * From: openai.test.ts "should accept reasoning with o3-mini (streaming)"
   */
  proxyOpenAIO3MiniStreamingReasoning: {
    "chat-completions": {
      model: "o3-mini-2025-01-31",
      messages: [{ role: "user", content: "What is 7 * 8?" }],
      reasoning_effort: "medium",
      stream: true,
      max_tokens: 1000,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    expect: {
      status: 200,
    },
  },
};
