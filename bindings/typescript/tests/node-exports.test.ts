/**
 * Node.js Exports Test
 *
 * Validates that the Node.js entry point exports all expected functionality
 * and that WASM auto-initializes without requiring manual init() calls.
 */

import { describe, test, expect } from "vitest";

describe("Node.js exports", () => {
  test("should export version constant", async () => {
    const exports = await import("../src/index");

    expect(exports.VERSION).toBeDefined();
    expect(typeof exports.VERSION).toBe("string");
  });

  test("should export conversion functions", async () => {
    const exports = await import("../src/index");

    expect(typeof exports.chatCompletionsMessagesToLingua).toBe("function");
    expect(typeof exports.linguaToChatCompletionsMessages).toBe("function");
    expect(typeof exports.anthropicMessagesToLingua).toBe("function");
    expect(typeof exports.linguaToAnthropicMessages).toBe("function");
    expect(typeof exports.transformRequest).toBe("function");
    expect(typeof exports.transformResponse).toBe("function");
  });

  test("should export validation functions", async () => {
    const exports = await import("../src/index");

    expect(typeof exports.validateChatCompletionsRequest).toBe("function");
    expect(typeof exports.validateChatCompletionsResponse).toBe("function");
    expect(typeof exports.validateAnthropicRequest).toBe("function");
    expect(typeof exports.validateAnthropicResponse).toBe("function");
  });

  test("should export error classes", async () => {
    const exports = await import("../src/index");

    expect(exports.ConversionError).toBeDefined();
    expect(exports.ConversionError.prototype).toBeInstanceOf(Error);
  });

  test("should auto-initialize WASM without manual init()", async () => {
    const { chatCompletionsMessagesToLingua } = await import("../src/index");

    const simpleMessages = [
      {
        role: "user" as const,
        content: "Hello, world!",
      },
    ];

    const result = chatCompletionsMessagesToLingua(simpleMessages);
    expect(result).toBeDefined();
    expect(Array.isArray(result)).toBe(true);
    expect(result.length).toBe(1);
    expect(result[0].role).toBe("user");
  });

  test("should transform chat completions request to universal request", async () => {
    const { transformRequest } = await import("../src/index");

    const result = transformRequest(
      JSON.stringify({
        model: "gpt-5-mini",
        messages: [{ role: "user", content: "Hello" }],
        temperature: 0.2,
      }),
      "universal",
    );

    expect(result.data).toMatchObject({
      model: "gpt-5-mini",
      messages: [{ role: "user", content: "Hello" }],
      params: { temperature: 0.2 },
    });
  });

  test("should report actual target format when universal request upgrades to responses", async () => {
    const { transformRequest } = await import("../src/index");

    const result = transformRequest(
      JSON.stringify({
        model: "gpt-5.4-mini",
        messages: [{ role: "user", content: "Tokyo weather" }],
        params: {
          reasoning: {
            enabled: true,
            effort: "medium",
          },
          tools: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: "object",
                properties: { location: { type: "string" } },
                required: ["location"],
              },
              kind: "function",
            },
          ],
        },
      }),
      "openai",
    );

    expect(result).toMatchObject({
      transformed: true,
      sourceFormat: "universal",
      actualTargetFormat: "responses",
    });
    expect(result.data).toHaveProperty("input");
  });

  test("should transform chat completions response to universal response", async () => {
    const { transformResponse } = await import("../src/index");

    const result = transformResponse(
      JSON.stringify({
        id: "chatcmpl-123",
        object: "chat.completion",
        model: "gpt-5-mini",
        choices: [
          {
            index: 0,
            message: { role: "assistant", content: "Hello!" },
            finish_reason: "stop",
          },
        ],
      }),
      "universal",
    );

    expect(result.data).toMatchObject({
      model: "gpt-5-mini",
      messages: [{ role: "assistant", content: "Hello!" }],
      finish_reason: "Stop",
    });
  });

  test("should import messages from prompt wrapper with tool calls", async () => {
    const { importMessagesFromSpans } = await import("../src/index");

    const input = {
      prompt: [
        { role: "system", content: "You are a helpful assistant." },
        { role: "user", content: [{ type: "text", text: "Hello" }] },
        {
          role: "assistant",
          content: [
            {
              type: "tool-call",
              toolCallId: "call_1",
              toolName: "bash",
              input: { command: "ls" },
            },
          ],
        },
        {
          role: "tool",
          content: [
            {
              type: "tool-result",
              toolCallId: "call_1",
              toolName: "bash",
              output: { stdout: "ok" },
            },
          ],
        },
        { role: "assistant", content: [{ type: "text", text: "Done" }] },
      ],
    };

    const messages = importMessagesFromSpans([{ input }]);
    expect(messages.length).toBe(5);
  });

  test("should return plain numeric arrays for imported anthropic tool_use arguments", async () => {
    const { importMessagesFromSpans } = await import("../src/index");

    const messages = importMessagesFromSpans([
      {
        input: [
          {
            role: "assistant",
            content: [
              {
                type: "tool_use",
                id: "toolu_123",
                name: "subagent--provide_builder_input",
                input: {
                  input: [0, 1],
                  message:
                    "Please implement both photo uploads for sightings and the interactive map view.",
                  project_id: "project_123",
                },
              },
            ],
          },
        ],
      },
    ]);

    expect(messages).toEqual([
      {
        role: "assistant",
        id: null,
        content: [
          {
            type: "tool_call",
            tool_call_id: "toolu_123",
            tool_name: "subagent--provide_builder_input",
            arguments: {
              type: "valid",
              value: {
                input: [0, 1],
                message:
                  "Please implement both photo uploads for sightings and the interactive map view.",
                project_id: "project_123",
              },
            },
          },
        ],
      },
    ]);
    expect(JSON.stringify(messages)).not.toContain("$serde_json::private::Number");
  });

  test("should return bigint values for imported integers outside JS safe range", async () => {
    const { importMessagesFromSpans } = await import("../src/index");

    const messages = importMessagesFromSpans([
      {
        input: [
          {
            role: "assistant",
            content: [
              {
                type: "tool_use",
                id: "toolu_123",
                name: "subagent--provide_builder_input",
                input: {
                  input: [9007199254740993n],
                  message: "Preserve bigint tool arguments.",
                  project_id: "project_123",
                },
              },
            ],
          },
        ],
      },
    ]);

    const value = (
      messages[0].content[0].arguments.type === "valid" &&
      messages[0].content[0].arguments.value.input[0]
    );

    expect(typeof value).toBe("bigint");
    expect(value).toBe(9007199254740993n);
  });

  test("should deduplicate messages across spans and return plain-object results", async () => {
    const { importAndDeduplicateMessages } = await import("../src/index");

    const sharedTurn = [
      { role: "user", content: "what is 2+2?" },
      { role: "assistant", content: "4" },
    ];

    const messages = importAndDeduplicateMessages([
      { input: sharedTurn },
      { input: sharedTurn, output: { role: "assistant", content: "4" } },
      { input: [{ role: "user", content: "and 3+3?" }] },
    ]);

    expect(messages).toEqual([
      { role: "user", content: "what is 2+2?" },
      { role: "assistant", id: null, content: "4" },
      { role: "user", content: "and 3+3?" },
    ]);
  });

  test("should NOT export browser-specific init function", async () => {
    const exports = await import("../src/index");

    expect(exports.default).toBeUndefined();
  });
});
