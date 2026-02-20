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
});
