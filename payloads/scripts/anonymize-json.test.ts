import { describe, expect, it } from "vitest";
import { anonymizeJsonValue, type JsonValue } from "./anonymize-json";

interface DefaultFixture {
  input: Array<{
    role: string;
    content: string | Array<{ type: string; text: string }>;
    id?: string;
    finish_reason?: string;
  }>;
  metadata: string;
}

describe("anonymizeJsonValue", () => {
  it("anonymizes only strings inside content by default", () => {
    const input: DefaultFixture = {
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

    const result = anonymizeJsonValue(input as unknown as JsonValue);
    const output = result.value as unknown as DefaultFixture;
    const secondContent = output.input[1].content as Array<{
      type: string;
      text: string;
    }>;

    expect(output.input[0].role).toBe("user");
    expect(output.input[0].id).toBe("user-1");
    expect(secondContent[0].type).toBe("text");
    expect(output.input[1].finish_reason).toBe("stop");
    expect(output.metadata).toBe("leave me alone");

    expect(output.input[0].content).toBe("anon_1");
    expect(secondContent[0].text).toBe("anon_1");
    expect(result.replacedStringCount).toBe(2);
    expect(result.uniqueReplacementCount).toBe(1);
  });

  it("anonymizes all strings when allStrings is enabled", () => {
    const input = {
      role: "user",
      content: "hello world",
      type: "text",
    };

    const result = anonymizeJsonValue(input as unknown as JsonValue, {
      allStrings: true,
    });
    const output = result.value as typeof input;

    expect(output.role).toBe("anon_1");
    expect(output.content).toBe("anon_2");
    expect(output.type).toBe("anon_3");
    expect(result.replacedStringCount).toBe(3);
    expect(result.uniqueReplacementCount).toBe(3);
  });

  it("preserves configured keys inside content", () => {
    const input = {
      content: [{ toolName: "bash", text: "run ls", type: "text" }],
    };

    const result = anonymizeJsonValue(input as unknown as JsonValue, {
      preserveKeys: new Set(["type", "toolName"]),
    });
    const output = result.value as typeof input;

    expect(output.content[0].toolName).toBe("bash");
    expect(output.content[0].type).toBe("text");
    expect(output.content[0].text).toBe("anon_1");
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

    const result = anonymizeJsonValue(input as unknown as JsonValue);
    const output = result.value as typeof input;

    expect(output.metadata.model).toBe("gpt-5.1-2025-11-13");
    expect(output.metadata.trace_id).toBe("anon_1");
    expect(output.metadata.route).toBe("anon_2");
    expect(output.metadata.tool_definitions[0].name).toBe("anon_3");
    expect(output.metadata.tool_definitions[0].description).toBe("anon_4");
    expect(output.metadata.tool_definitions[0].parameters.type).toBe("object");
    expect(
      output.metadata.tool_definitions[0].parameters.properties.city.type
    ).toBe("string");
    expect(
      output.metadata.tool_definitions[0].parameters.properties.city.description
    ).toBe("anon_5");
  });

  it("anonymizes metadata variants like metadata2", () => {
    const input = {
      metadata2: {
        chatChannel: "SOLO_TOLAN:usr_abc",
        chatID: "cht_123",
        isFirstMessage: false,
      },
    };

    const result = anonymizeJsonValue(input as unknown as JsonValue);
    const output = result.value as typeof input;

    expect(output.metadata2.chatChannel).toBe("anon_1");
    expect(output.metadata2.chatID).toBe("anon_2");
    expect(output.metadata2.isFirstMessage).toBe(false);
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

    const result = anonymizeJsonValue(input as unknown as JsonValue);
    const output = result.value as {
      metadata: { prompt?: unknown; route: string };
    };

    expect(output.metadata.prompt).toBeUndefined();
    expect(output.metadata.route).toBe("anon_1");
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

    const result = anonymizeJsonValue(input as unknown as JsonValue);
    const output = result.value as typeof input;

    expect(output.context.caller_filename).toBe("anon_1");
    expect(output.context.caller_functionname).toBe("anon_2");
    expect(output.context.caller_lineno).toBe(42);
    expect(output.output).toBe("anon_3");
    expect(output.model).toBe("gpt-5.1-2025-11-13");
  });
});
