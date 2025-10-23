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

  test("should NOT export browser-specific init function", async () => {
    const exports = await import("../src/index");

    expect(exports.default).toBeUndefined();
  });
});
