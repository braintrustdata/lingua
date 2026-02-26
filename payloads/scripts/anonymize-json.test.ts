import { describe, expect, it } from "vitest";
import { anonymizeJsonValue, type JsonValue } from "./anonymize-json";

describe("anonymizeJsonValue", () => {
  it("anonymizes content and metadata strings by default", () => {
    const input: JsonValue = {
      input: [
        { role: "user", content: "hello world", id: "user-1" },
        {
          role: "assistant",
          content: [{ type: "text", text: "hello world" }],
          finish_reason: "stop",
        },
      ],
      metadata: "leave me alone",
    };

    const result = anonymizeJsonValue(input);
    expect(result.value).toEqual({
      input: [
        { role: "user", content: "anon_1", id: "user-1" },
        {
          role: "assistant",
          content: [{ type: "text", text: "anon_1" }],
          finish_reason: "stop",
        },
      ],
      metadata: "anon_2",
    });
    expect(result.replacedStringCount).toBe(3);
    expect(result.uniqueReplacementCount).toBe(2);
  });

  it("anonymizes all strings when allStrings is enabled", () => {
    const input = {
      role: "user",
      content: "hello world",
      type: "text",
    };

    const result = anonymizeJsonValue(input, {
      allStrings: true,
    });
    expect(result.value).toEqual({
      role: "anon_1",
      content: "anon_2",
      type: "anon_3",
    });
    expect(result.replacedStringCount).toBe(3);
    expect(result.uniqueReplacementCount).toBe(3);
  });

  it("preserves configured keys inside content", () => {
    const input = {
      content: [{ toolName: "bash", text: "run ls", type: "text" }],
    };

    const result = anonymizeJsonValue(input, {
      preserveKeys: new Set(["type", "toolName"]),
    });
    expect(result.value).toEqual({
      content: [{ toolName: "bash", text: "anon_1", type: "text" }],
    });
    expect(result.replacedStringCount).toBe(1);
    expect(result.uniqueReplacementCount).toBe(1);
  });

  it("anonymizes all metadata strings", () => {
    const input = {
      metadata: {
        model: "gpt-5.1-2025-11-13",
        trace_id: "trace-1",
        route: "base",
        tool_definitions: [
          {
            name: "fetch_weather",
            description: "Get weather by city",
            parameters: {
              type: "object",
              properties: {
                city: { type: "string", description: "City name" },
              },
            },
          },
        ],
      },
    };

    const result = anonymizeJsonValue(input);
    expect(result.value).toEqual({
      metadata: {
        model: "gpt-5.1-2025-11-13",
        trace_id: "anon_1",
        route: "anon_2",
        tool_definitions: [
          {
            name: "anon_3",
            description: "anon_4",
            parameters: {
              type: "object",
              properties: {
                city: { type: "string", description: "anon_5" },
              },
            },
          },
        ],
      },
    });
  });

  it("anonymizes metadata variants like metadata2", () => {
    const input = {
      metadata2: {
        chatChannel: "SOLO_TOLAN:usr_abc",
        chatID: "cht_123",
        isFirstMessage: false,
      },
    };

    const result = anonymizeJsonValue(input);
    expect(result.value).toEqual({
      metadata2: {
        chatChannel: "anon_1",
        chatID: "anon_2",
        isFirstMessage: false,
      },
    });
  });

  it("removes metadata prompt subtree entirely", () => {
    const input = {
      metadata: {
        prompt: {
          id: "prm_123",
          key: "chat",
          variables: {
            activeChatType: { CONVERSATION_DEFAULT: true },
            medium: "TEXT",
          },
        },
        route: "base",
      },
    };

    const result = anonymizeJsonValue(input);
    expect(result.value).toEqual({
      metadata: {
        route: "anon_1",
      },
    });
  });

  it("anonymizes strings under context and output", () => {
    const input = {
      context: {
        caller_filename: "file:///tmp/project/src/main.ts",
        caller_functionname: "runJob",
        caller_lineno: 42,
      },
      output: "Final assistant response text",
      model: "gpt-5.1-2025-11-13",
    };

    const result = anonymizeJsonValue(input);
    expect(result.value).toEqual({
      context: {
        caller_filename: "anon_1",
        caller_functionname: "anon_2",
        caller_lineno: 42,
      },
      output: "anon_3",
      model: "gpt-5.1-2025-11-13",
    });
  });
});
