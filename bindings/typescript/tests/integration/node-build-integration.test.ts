/**
 * Node.js Build Integration Tests
 *
 * Verifies the Node.js build pipeline produces correct artifacts:
 * - Build files exist in expected locations
 * - Package exports resolve correctly
 * - Bundle is importable
 */

import { describe, test, expect, beforeAll } from "vitest";
import * as fs from "fs";
import * as path from "path";

const distPath = path.join(__dirname, "../../dist/index.mjs");
const packageRoot = path.join(__dirname, "../..");
const packageJsonPath = path.join(packageRoot, "package.json");

beforeAll(() => {
  if (!fs.existsSync(distPath)) {
    throw new Error(
      `Build output not found at ${distPath}. Run 'pnpm build' first.`
    );
  }
});

describe("Node.js Build Integration", () => {
  describe("Build Output Verification", () => {
    test("all Node.js build artifacts exist", () => {
      const requiredFiles = [
        "dist/index.mjs",
        "dist/index.d.mts",
        "dist/index.mjs.map",
        "dist/wasm-node/lingua_bg.wasm",
      ];

      for (const file of requiredFiles) {
        const fullPath = path.join(packageRoot, file);
        expect(fs.existsSync(fullPath)).toBe(true);
      }
    });
  });

  describe("Package Exports", () => {
    test("package.json is properly configured for Node.js", () => {
      const content = fs.readFileSync(packageJsonPath, "utf-8");
      const pkg = JSON.parse(content);

      // Check exports exist
      expect(pkg.exports).toBeDefined();
      expect(pkg.exports["."]).toBeDefined();
      expect(pkg.exports["./node"]).toBeDefined();

      // Check default export points correctly
      const defaultExport =
        pkg.exports["."].import || pkg.exports["."].default;
      expect(defaultExport).toContain("dist/index.mjs");
      expect(fs.existsSync(path.join(packageRoot, defaultExport))).toBe(true);

      // Check types
      const typesPath = pkg.exports["."].types || pkg.types;
      expect(typesPath).toBeDefined();
      expect(typesPath).toContain(".d.mts");
      expect(fs.existsSync(path.join(packageRoot, typesPath))).toBe(true);
    });
  });

  describe("Module Imports", () => {
    test("can import bundle without errors", async () => {
      const module = await import(distPath);
      expect(module).toBeDefined();
    });
  });
});
