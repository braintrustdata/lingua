/**
 * Browser Exports Test
 *
 * Validates that the browser entry point exports all expected functionality.
 * With the bundler target, WASM is auto-initialized at import time.
 */

import { describe, test, expect } from "vitest";

describe("Browser exports", () => {
  test("should export default init function", async () => {
    const exports = await import("../src/index.browser");

    expect(exports.default).toBeDefined();
    expect(typeof exports.default).toBe("function");
  });

  test("should export named init function", async () => {
    const exports = await import("../src/index.browser");

    expect(exports.init).toBeDefined();
    expect(typeof exports.init).toBe("function");
  });

  test("should export version constant", async () => {
    const exports = await import("../src/index.browser");

    expect(exports.VERSION).toBeDefined();
    expect(typeof exports.VERSION).toBe("string");
  });

  test("should export conversion functions", async () => {
    const exports = await import("../src/index.browser");

    expect(typeof exports.chatCompletionsMessagesToLingua).toBe("function");
    expect(typeof exports.linguaToChatCompletionsMessages).toBe("function");
    expect(typeof exports.anthropicMessagesToLingua).toBe("function");
    expect(typeof exports.linguaToAnthropicMessages).toBe("function");
  });

  test("should export validation functions", async () => {
    const exports = await import("../src/index.browser");

    expect(typeof exports.validateChatCompletionsRequest).toBe("function");
    expect(typeof exports.validateChatCompletionsResponse).toBe("function");
    expect(typeof exports.validateAnthropicRequest).toBe("function");
    expect(typeof exports.validateAnthropicResponse).toBe("function");
  });

  test("should export error classes", async () => {
    const exports = await import("../src/index.browser");

    expect(exports.ConversionError).toBeDefined();
    expect(exports.ConversionError.prototype).toBeInstanceOf(Error);
  });

  test("should auto-initialize WASM and work without manual init()", async () => {
    const { chatCompletionsMessagesToLingua } = await import(
      "../src/index.browser"
    );

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

  test("init() should be a no-op that resolves immediately", async () => {
    const { init, chatCompletionsMessagesToLingua } = await import(
      "../src/index.browser"
    );

    // init() should complete without error (it's a no-op for bundler target)
    await init();

    // Functions should still work
    const result = chatCompletionsMessagesToLingua([
      { role: "user", content: "Test" },
    ]);
    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });
});
