import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

test("the built trace entry retries initialization without loading full WASM", () => {
  const output = execFileSync(
    process.execPath,
    [
      "--input-type=module",
      "--eval",
      `
        import assert from "node:assert/strict";
        import { readFileSync } from "node:fs";
        import { createRequire } from "node:module";
        import * as trace from "@braintrust/lingua/browser/trace";

        const require = createRequire(import.meta.url);
        const fullModule = require.resolve("@braintrust/lingua-wasm");
        assert.equal(require.cache[fullModule], undefined);
        assert.throws(() => trace.getWasm(), /not initialized/);
        await assert.rejects(trace.init(new Uint8Array([0])));
        const bytes = readFileSync(new URL(import.meta.resolve(
          "@braintrust/lingua-wasm/browser/trace/lingua_bg.wasm"
        )));
        await Promise.all([trace.init(bytes), trace.init(bytes)]);
        assert.deepEqual(
          trace.importMessagesFromSpans([
            { input: [{ role: "user", content: "Hello" }] }
          ]),
          [{ role: "user", content: "Hello" }]
        );
        assert.equal(require.cache[fullModule], undefined);
        assert.equal("validate_chat_completions_request" in trace.getWasm(), false);
        console.log("trace build ready");
      `,
    ],
    {
      cwd: fileURLToPath(new URL("../../", import.meta.url)),
      encoding: "utf8",
    }
  );
  expect(output).toContain("trace build ready");
});
