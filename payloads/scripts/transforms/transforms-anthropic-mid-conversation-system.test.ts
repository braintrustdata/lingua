import { describe, expect, test } from "vitest";
import { transform_request } from "@braintrust/lingua-wasm";
import {
  type MessageCreateParamsNonStreaming,
  type MessageParam,
  type OutputConfig,
} from "@anthropic-ai/sdk/resources/messages";
import { ANTHROPIC_MODEL, OPENAI_CHAT_COMPLETIONS_MODEL } from "../../cases";
import { transformAndValidateRequest } from "./helpers";

describe("anthropic mid-conversation system messages", () => {
  const user: MessageParam = { role: "user", content: "Say OK." };
  const assistant: MessageParam = { role: "assistant", content: "OK" };
  const patch: MessageParam = {
    role: "system",
    content: "Keep answers brief.",
  };
  const effort = {
    role: "system",
    content: [],
    output_config: { effort: "high" },
  } satisfies MessageParam & { output_config: OutputConfig };
  const requestTemplate = {
    model: "claude-opus-5-5",
    max_tokens: 1024,
    system: "Follow the user's instructions.",
    thinking: { type: "adaptive" },
    output_config: { effort: "high" },
    messages: [user, patch, effort],
  } satisfies MessageCreateParamsNonStreaming;
  const serverResult: MessageParam = {
    role: "assistant",
    content: [
      {
        type: "web_search_tool_result",
        tool_use_id: "srvtoolu_1",
        content: [],
      },
    ],
  };

  test.each([
    { name: "consecutive system messages", messages: [user, patch, patch] },
    { name: "a prompt patch with effort", messages: requestTemplate.messages },
    {
      name: "a section before an assistant",
      messages: [user, effort, patch, assistant],
    },
    {
      name: "leading system text for upstream validation",
      messages: [patch, user],
    },
    { name: "an initial effort directive", messages: [effort, user] },
    { name: "effort between turns", messages: [user, assistant, effort, user] },
    { name: "consecutive effort directives", messages: [effort, effort] },
    {
      name: "a section after server tool results",
      messages: [user, serverResult, patch, effort],
    },
    {
      name: "leading content for upstream validation",
      messages: [effort, patch, user],
    },
    {
      name: "content before a user turn for upstream validation",
      messages: [user, patch, effort, user],
    },
  ])("preserves $name without placement validation", ({ messages }) => {
    const request = { ...requestTemplate, messages };
    expect(transform_request(JSON.stringify(request), "Anthropic")).toEqual({
      passThrough: true,
      data: request,
    });
  });

  test.each(["claude-opus-4-8", "claude-haiku-4-5", "gpt-5-mini"])(
    "detects the wire format independently of model capabilities: %s",
    (model) => {
      const request = { model, max_tokens: 1024, messages: [user, patch] };
      expect(transform_request(JSON.stringify(request), "Anthropic")).toEqual({
        passThrough: true,
        data: request,
      });
    }
  );

  test("recognizes native tool syntax with leading system instructions", () => {
    const request = {
      model: "claude-opus-4-8",
      max_tokens: 1024,
      messages: [patch, user],
      tools: [{ name: "echo", input_schema: { type: "object" } }],
    };
    expect(transform_request(JSON.stringify(request), "Anthropic")).toEqual({
      passThrough: true,
      data: request,
    });
  });

  test("distinguishes unmarked OpenAI leading system instructions", () => {
    const request = {
      model: "gpt-5-mini",
      max_tokens: 1024,
      messages: [patch, user],
    };
    expect(transform_request(JSON.stringify(request), "OpenAI")).toEqual({
      transformed: true,
      sourceFormat: "openai",
      data: {
        model: request.model,
        max_completion_tokens: request.max_tokens,
        messages: request.messages,
      },
    });
  });

  test.each(["OpenAI", "Responses", "Google"])(
    "rejects per-message effort conversion to %s",
    (target) => {
      expect(() =>
        transform_request(JSON.stringify(requestTemplate), target)
      ).toThrow("per-message output_config");
    }
  );

  test("rejects non-leading system messages for Anthropic export", () => {
    expect(() =>
      transformAndValidateRequest(
        {
          model: OPENAI_CHAT_COMPLETIONS_MODEL,
          max_completion_tokens: 300,
          messages: [
            {
              role: "system",
              content: "Use the initial policy.",
            },
            {
              role: "user",
              content: "What is the required answer?",
            },
            {
              role: "system",
              content: "Use the updated policy.",
            },
          ],
        },
        "Anthropic",
        "anthropic",
        ANTHROPIC_MODEL
      )
    ).toThrowErrorMatchingSnapshot();
  });
});
