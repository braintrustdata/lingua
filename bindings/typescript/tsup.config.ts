import { type Options } from "tsup";

export default {
  entry: {
    index: "src/index.ts",
    "index.browser": "src/index.browser.ts",
  },
  format: ["esm"],
  outDir: "dist",
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
} satisfies Options;
