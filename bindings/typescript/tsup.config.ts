import { defineConfig } from "tsup";

export default defineConfig([
  {
    entry: {
      index: "src/index.ts",
      "index.browser": "src/index.browser.ts",
    },
    format: ["esm"],
    outDir: "dist/types",
    dts: { only: true },
    splitting: false,
    clean: false,
    external: [/^\.\.\/dist\//],
  },
]);
