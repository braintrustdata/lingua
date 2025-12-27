/**
 * Browser Exports Test
 *
 * Validates that the browser entry point exports all expected functionality.
 * With the web target, init(url) must be called with explicit WASM path.
 */

import { describe, test, expect, beforeAll, afterEach } from "vitest";
import { readFileSync } from "fs";
import { join } from "path";

const wasmPath = join(__dirname, "../../lingua-wasm/web/lingua_bg.wasm");

describe("Browser exports", () => {
  afterEach(async () => {
    const { resetWasmForTests } = await import("../src/wasm-runtime");
    resetWasmForTests();
  });

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

  test("should work after init() with WASM buffer", async () => {
    const { init, chatCompletionsMessagesToLingua } = await import(
      "../src/index.browser"
    );

    const wasmBuffer = readFileSync(wasmPath);
    await init(wasmBuffer);

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

  test("init() can be called multiple times safely", async () => {
    const { init, chatCompletionsMessagesToLingua } = await import(
      "../src/index.browser"
    );

    const wasmBuffer = readFileSync(wasmPath);
    
    // Multiple init calls should be safe
    await init(wasmBuffer);
    await init(wasmBuffer);

    // Functions should still work
    const result = chatCompletionsMessagesToLingua([
      { role: "user", content: "Test" },
    ]);
    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });
});
