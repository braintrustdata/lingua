import { defineConfig } from "tsup";
import { readFileSync, writeFileSync, cpSync, mkdirSync } from "fs";
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
    external: [/^\.\.\/wasm\//],
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
    clean: false,
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

      // Copy .wasm binaries for export
      const distWasmNode = join(__dirname, "dist", "wasm-node");
      const distWasmWeb = join(__dirname, "dist", "wasm-web");
      mkdirSync(distWasmNode, { recursive: true });
      mkdirSync(distWasmWeb, { recursive: true });
      cpSync(join(__dirname, "wasm", "lingua_bg.wasm"), join(distWasmNode, "lingua_bg.wasm"));
      cpSync(join(__dirname, "wasm-web", "lingua_bg.wasm"), join(distWasmWeb, "lingua_bg.wasm"));
    },
  },
]);
