import { defineConfig } from "tsup";

export default defineConfig([
  {
    entry: {
      index: "src/index.ts",
    },
    format: ["esm"],
    outDir: "dist",
    outExtension: () => ({ js: ".js" }),
    dts: true,
    splitting: false,
    sourcemap: true,
    clean: false,
    external: [/^\.\.\/wasm\//],
  },
  {
    entry: {
      "index.browser": "src/index.browser.ts",
    },
    format: ["esm"],
    outDir: "dist",
    outExtension: () => ({ js: ".js" }),
    dts: true,
    splitting: false,
    sourcemap: true,
    clean: false,
    external: [/^\.\.\/wasm\//],
  },
]);
