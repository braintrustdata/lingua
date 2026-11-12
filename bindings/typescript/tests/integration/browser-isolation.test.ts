import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

test("the built browser entry retries initialization without loading Node WASM", () => {
  const output = execFileSync(
    process.execPath,
    [
      "--input-type=module",
      "--eval",
      `
        import assert from "node:assert/strict";
        import { readFileSync } from "node:fs";
        import { createRequire } from "node:module";
        import * as browser from "@braintrust/lingua/browser";

        const require = createRequire(import.meta.url);
        const nodeModule = require.resolve("@braintrust/lingua-wasm");
        assert.equal(require.cache[nodeModule], undefined);
        assert.throws(() => browser.getWasm(), /not initialized/);
        await assert.rejects(browser.init(new Uint8Array([0])));
        const bytes = readFileSync(new URL(import.meta.resolve(
          "@braintrust/lingua-wasm/browser/lingua_bg.wasm"
        )));
        await Promise.all([browser.init(bytes), browser.init(bytes)]);
        assert.deepEqual(
          browser.importMessagesFromSpans([
            { input: [{ role: "user", content: "Hello" }] }
          ]),
          [{ role: "user", content: "Hello" }]
        );
        assert.equal(require.cache[nodeModule], undefined);
        assert.equal("validate_chat_completions_request" in browser.getWasm(), false);
        console.log("browser build ready");
      `,
    ],
    {
      cwd: fileURLToPath(new URL("../../", import.meta.url)),
      encoding: "utf8",
    }
  );
  expect(output).toContain("browser build ready");
});
