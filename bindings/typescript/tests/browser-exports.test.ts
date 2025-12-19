/**
 * Browser Exports Test
 *
 * Validates that the browser entry point exports all expected functionality
 * and that WASM requires explicit init() before use.
 */

import { describe, test, expect, beforeAll, afterAll } from "vitest";
import * as fs from "fs";
import * as path from "path";
import * as http from "http";

let server: http.Server | null = null;
let serverPort: number = 0;

function startWasmServer(): Promise<number> {
  return new Promise((resolve, reject) => {
    const wasmPath = path.join(__dirname, "../dist/wasm/web/lingua_bg.wasm");
    const wasmContent = fs.readFileSync(wasmPath);

    server = http.createServer((req, res) => {
      if (req.url === "/lingua_bg.wasm") {
        res.writeHead(200, {
          "Content-Type": "application/wasm",
          "Access-Control-Allow-Origin": "*",
          "Content-Length": wasmContent.length,
        });
        res.end(wasmContent);
      } else {
        res.writeHead(404);
        res.end();
      }
    });

    server.on("error", reject);

    server.listen(0, "127.0.0.1", () => {
      const address = server!.address();
      if (address && typeof address !== "string") {
        serverPort = address.port;
        resolve(serverPort);
      } else {
        reject(new Error("Failed to get server address"));
      }
    });
  });
}

function stopWasmServer(): Promise<void> {
  return new Promise((resolve, reject) => {
    if (server) {
      server.close((err) => {
        if (err) reject(err);
        else resolve();
      });
    } else {
      resolve();
    }
  });
}

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

  test("init() accepts WASM buffer and works", async () => {
    const init = (await import("../src/index.browser")).default;
    const { chatCompletionsMessagesToLingua } = await import(
      "../src/index.browser"
    );

    const wasmPath = path.join(__dirname, "../dist/wasm/web/lingua_bg.wasm");
    const wasmBuffer = fs.readFileSync(wasmPath);

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
  });

  describe("URL loading", () => {
    beforeAll(async () => {
      await startWasmServer();
    });

    afterAll(async () => {
      await stopWasmServer();
    });

    test("init() accepts string URL and works", async () => {
      const { resetWasmForTests } = await import("../src/wasm-runtime");
      resetWasmForTests();

      const init = (await import("../src/index.browser")).default;
      const { chatCompletionsMessagesToLingua } = await import(
        "../src/index.browser"
      );

      const wasmUrl = `http://127.0.0.1:${serverPort}/lingua_bg.wasm`;
      await init(wasmUrl);

      const simpleMessages = [
        {
          role: "user" as const,
          content: "Hello from URL!",
        },
      ];

      const result = chatCompletionsMessagesToLingua(simpleMessages);
      expect(result).toBeDefined();
      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("user");
    });

    test("init() accepts URL object and works", async () => {
      const { resetWasmForTests } = await import("../src/wasm-runtime");
      resetWasmForTests();

      const init = (await import("../src/index.browser")).default;
      const { linguaToChatCompletionsMessages } = await import(
        "../src/index.browser"
      );

      const wasmUrl = new URL(
        `http://127.0.0.1:${serverPort}/lingua_bg.wasm`
      );
      await init(wasmUrl);

      const linguaMessages = [
        {
          role: "user" as const,
          content: [
            {
              type: "text" as const,
              text: "Test with URL object",
            },
          ],
        },
      ];

      const result = linguaToChatCompletionsMessages(linguaMessages);
      expect(result).toBeDefined();
      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBe(1);
    });

    test("init() accepts fetch() Response and works", async () => {
      const { resetWasmForTests } = await import("../src/wasm-runtime");
      resetWasmForTests();

      const init = (await import("../src/index.browser")).default;
      const { anthropicMessagesToLingua } = await import(
        "../src/index.browser"
      );

      const wasmUrl = `http://127.0.0.1:${serverPort}/lingua_bg.wasm`;
      const response = await fetch(wasmUrl);
      await init(response);

      const anthropicMessages = [
        {
          role: "user" as const,
          content: "Test with fetch response",
        },
      ];

      const result = anthropicMessagesToLingua(anthropicMessages);
      expect(result).toBeDefined();
      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBe(1);
      expect(result[0].role).toBe("user");
    });
  });
});
