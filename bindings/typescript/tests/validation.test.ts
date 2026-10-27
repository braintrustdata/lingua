import { describe, expect, test } from "vitest";
import * as full from "@braintrust/lingua-wasm/full";
import {
  type ChatCompletionCreateParamsNonStreaming,
  type ChatCompletion,
} from "openai/resources/chat/completions";
import {
  type MessageCreateParams,
  type Message,
} from "@anthropic-ai/sdk/resources/messages";

const OPENAI_REQUEST_DATA: ChatCompletionCreateParamsNonStreaming = {
  model: "gpt-4",
  messages: [
    {
      role: "user",
      content: "Hello",
    },
  ],
  frequency_penalty: 0.2,
};
const OPENAI_REQUEST = JSON.stringify(OPENAI_REQUEST_DATA);

const OPENAI_RESPONSE_DATA: ChatCompletion = {
  id: "chatcmpl-123",
  object: "chat.completion",
  created: 1677652288,
  model: "gpt-4",
  choices: [
    {
      index: 0,
      message: {
        role: "assistant",
        content: "Hello!",
      },
      finish_reason: "stop",
      logprobs: null,
    },
  ],
  usage: {
    prompt_tokens: 9,
    completion_tokens: 12,
    total_tokens: 21,
  },
};
const OPENAI_RESPONSE = JSON.stringify(OPENAI_RESPONSE_DATA);

const ANTHROPIC_REQUEST_DATA: MessageCreateParams = {
  model: "claude-3-5-sonnet-20241022",
  messages: [
    {
      role: "user",
      content: [
        {
          type: "text",
          text: "Hello",
        },
      ],
    },
  ],
  max_tokens: 1024,
};
const ANTHROPIC_REQUEST = JSON.stringify(ANTHROPIC_REQUEST_DATA);

const ANTHROPIC_RESPONSE_DATA: Message = {
  id: "msg_123",
  type: "message",
  role: "assistant",
  content: [
    {
      type: "text",
      text: "Hello!",
    },
  ],
  model: "claude-3-5-sonnet-20241022",
  stop_reason: "end_turn",
  stop_sequence: null,
  usage: {
    input_tokens: 10,
    output_tokens: 20,
  },
};
const ANTHROPIC_RESPONSE = JSON.stringify(ANTHROPIC_RESPONSE_DATA);

const STREAM_CHUNK_FINAL = JSON.stringify({
  id: "chatcmpl-abc123",
  object: "chat.completion.chunk",
  created: 1677652288,
  model: "gpt-4",
  choices: [
    {
      index: 0,
      delta: {},
      finish_reason: "stop",
    },
  ],
});

const STREAM_CHUNK_CONTENT = JSON.stringify({
  id: "chatcmpl-abc123",
  object: "chat.completion.chunk",
  created: 1677652288,
  model: "gpt-4",
  choices: [
    {
      index: 0,
      delta: { content: "Paris" },
      finish_reason: null,
    },
  ],
});

describe.each([
  {
    name: "Chat Completions",
    validateRequest: full.validate_chat_completions_request,
    validateResponse: full.validate_chat_completions_response,
    request: OPENAI_REQUEST,
    response: OPENAI_RESPONSE,
    otherResponse: ANTHROPIC_RESPONSE,
    model: "gpt-4",
  },
  {
    name: "Anthropic",
    validateRequest: full.validate_anthropic_request,
    validateResponse: full.validate_anthropic_response,
    request: ANTHROPIC_REQUEST,
    response: ANTHROPIC_RESPONSE,
    otherResponse: OPENAI_RESPONSE,
    model: "claude-3-5-sonnet-20241022",
  },
])(
  "full WASM $name validation",
  ({
    validateRequest,
    validateResponse,
    request,
    response,
    otherResponse,
    model,
  }) => {
    test("returns the parsed request", () => {
      expect(validateRequest(request)).toMatchObject({
        model,
        messages: expect.any(Array),
      });
    });
    test("returns the parsed response", () => {
      expect(validateResponse(response)).toMatchObject({ model });
    });
    test("rejects another provider's response", () => {
      expect(() => validateResponse(otherResponse)).toThrow();
    });
    test("rejects invalid JSON", () => {
      expect(() => validateRequest("{invalid json}")).toThrow(
        "Deserialization failed"
      );
    });
    test("rejects missing required fields", () => {
      expect(() => validateRequest(JSON.stringify({ model }))).toThrow();
    });
  }
);

test("the full build rejects an OpenAI request as Anthropic", () => {
  expect(() => full.validate_anthropic_request(OPENAI_REQUEST)).toThrow();
});

test("the full build validates final stream chunks", () => {
  expect(
    full.validate_chat_completions_stream_chunk(STREAM_CHUNK_FINAL)
  ).toMatchObject({ choices: expect.any(Array) });
  expect(() =>
    full.validate_chat_completions_stream_chunk("{bad json}")
  ).toThrow();
  expect(() =>
    full.validate_chat_completions_stream_chunk(OPENAI_RESPONSE)
  ).toThrow();
});

test("the full build preserves stream passthrough and transformation", () => {
  expect(
    full.transform_stream_chunk(STREAM_CHUNK_CONTENT, "openai")
  ).toHaveProperty("passThrough", true);
  expect(
    full.transform_stream_chunk(STREAM_CHUNK_CONTENT, "anthropic")
  ).toMatchObject({ transformed: true, sourceFormat: "openai" });
  expect(() =>
    full.transform_stream_chunk(STREAM_CHUNK_CONTENT, "not_a_format")
  ).toThrow();
});
