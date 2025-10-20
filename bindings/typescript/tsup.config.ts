import { defineConfig } from "tsup";
import { readFileSync, writeFileSync } from "fs";
import { join } from "path";

export default defineConfig([
  {
    entry: {
      index: "src/index.ts",
    },
    format: ["esm"],
    outDir: "dist",
    dts: true,
    splitting: false,
    sourcemap: true,
    clean: true,
  },
  {
    entry: {
      "index.browser": "src/index.browser.ts",
    },
    format: ["esm"],
    outDir: "dist",
    dts: true,
    splitting: false,
    sourcemap: true,
    clean: true,
    onSuccess: async () => {
      const filePath = join(__dirname, "dist", "index.browser.mjs");
      let content = readFileSync(filePath, "utf-8");

      // Remove the fallback that breaks webpack static analysis
      // Replace: if (typeof module_or_path === 'undefined') { module_or_path = new URL('lingua_bg.wasm', import.meta.url); }
      content = content.replace(
        /if\s*\(\s*typeof\s+module_or_path\s*===\s*['"]undefined['"]\s*\)\s*\{\s*module_or_path\s*=\s*new\s+URL\s*\(\s*['"]lingua_bg\.wasm['"]\s*,\s*import\.meta\.url\s*\)\s*;\s*\}/g,
        "if (typeof module_or_path === 'undefined') { throw new Error('WASM module, path, or URL must be provided to init()'); }"
      );

      writeFileSync(filePath, content, "utf-8");
    },
  },
]);
