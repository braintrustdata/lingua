/**
 * Browser Build Integration Tests
 *
 * Verifies the browser build pipeline produces correct artifacts:
 * - Build files exist in expected locations
 * - No Node.js imports in browser bundle
 * - Package exports resolve correctly for browser
 * - Bundle is importable and initialization works
 */

import { describe, test, expect, beforeAll, afterEach } from "vitest";
import * as fs from "fs";
import * as path from "path";

const browserDistPath = path.join(__dirname, "../../dist/index.browser.js");
const wasmPath = path.join(__dirname, "../../wasm/bundler/lingua_bg.wasm");
const packageRoot = path.join(__dirname, "../..");
const packageJsonPath = path.join(packageRoot, "package.json");

beforeAll(() => {
  if (!fs.existsSync(browserDistPath)) {
    throw new Error(
      `Browser build not found at ${browserDistPath}. Run 'pnpm build' first.`
    );
  }
  if (!fs.existsSync(wasmPath)) {
    throw new Error(
      `Browser WASM not found at ${wasmPath}. Run 'pnpm build' first.`
    );
  }
});

describe("Browser Build Integration", () => {
  describe("Build Output Verification", () => {
    test("all browser build artifacts exist", () => {
      const requiredFiles = [
        "dist/index.browser.js",
        "dist/index.browser.d.mts",
        "wasm/bundler/lingua_bg.wasm",
        "wasm/bundler/lingua.js",
        "wasm/bundler/lingua_bg.js",
      ];

      for (const file of requiredFiles) {
        const fullPath = path.join(packageRoot, file);
        expect(fs.existsSync(fullPath)).toBe(true);
      }
    });

    test("no Node.js imports in browser bundle", () => {
      const content = fs.readFileSync(browserDistPath, "utf-8");
      expect(content).not.toContain('require("fs")');
      expect(content).not.toContain('require("path")');
      expect(content).not.toContain('from "node:');
    });
  });

  describe("Package Exports", () => {
    test("package.json is properly configured for browser", () => {
      const content = fs.readFileSync(packageJsonPath, "utf-8");
      const pkg = JSON.parse(content);

      expect(pkg.exports).toBeDefined();
      expect(pkg.exports["./browser"]).toBeDefined();

      const browserExport =
        pkg.exports["./browser"].import || pkg.exports["./browser"].default;
      expect(browserExport).toContain("dist/index.browser.js");
      expect(fs.existsSync(path.join(packageRoot, browserExport))).toBe(true);

      const wasmExport = pkg.exports["./browser/lingua_bg.wasm"];
      expect(wasmExport).toBeDefined();
      expect(fs.existsSync(path.join(packageRoot, wasmExport))).toBe(true);

      const typesPath = pkg.exports["./browser"].types;
      expect(typesPath).toBeDefined();
      expect(fs.existsSync(path.join(packageRoot, typesPath))).toBe(true);
    });
  });

  describe("Module Imports", () => {
    test("can import bundle without errors", async () => {
      const module = await import(browserDistPath);
      expect(module).toBeDefined();
    });

    test("exports init as default export", async () => {
      const module = await import(browserDistPath);
      expect(module.default).toBeDefined();
      expect(typeof module.default).toBe("function");
    });
  });

  describe("Initialization", () => {
    afterEach(async () => {
      const { resetWasmForTests } = await import(
        "../../src/wasm-runtime.js"
      );
      if (resetWasmForTests) {
        resetWasmForTests();
      }
    });

    test("can initialize with Buffer and use conversion", async () => {
      const modulePath = `${browserDistPath}?init-buffer-${Date.now()}`;
      const module = await import(modulePath);

      const wasmBuffer = fs.readFileSync(wasmPath);
      await module.default(wasmBuffer);

      const result = module.chatCompletionsMessagesToLingua([
        { role: "user", content: "Test" },
      ]);
      expect(result).toBeDefined();
      expect(Array.isArray(result)).toBe(true);
    });
  });
});
