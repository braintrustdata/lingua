import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    include: ["scripts/**/*.test.ts"],
    // Run up to 10 concurrent tests (for parallel API calls)
    maxConcurrency: 10,
  },
});
