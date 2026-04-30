import { defineConfig } from "tsup";

export default defineConfig({
  entry: {
    index: "src/index.ts",
  },
  format: ["esm"],
  outDir: "dist",
  dts: true,
  splitting: false,
  sourcemap: false,
  clean: true,
});
