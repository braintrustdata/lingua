import { type Options } from "tsup";

export default {
  entry: {
    index: "src/index.ts",
  },
  format: ["esm"],
  outDir: "dist",
  dts: true,
  splitting: false,
  sourcemap: false,
  clean: true,
} satisfies Options;
