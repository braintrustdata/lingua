import { type Options } from "tsup";

export default [
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
    clean: false,
  },
] satisfies Options[];
