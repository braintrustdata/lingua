import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  canonicalizeJson,
  canonicalizeJsonFile,
  cleanupTempFile,
} from "./canonicalize-json.mjs";

test("sorts object keys recursively while preserving array order", () => {
  assert.equal(
    canonicalizeJson(
      '{"z":{"second":2,"first":1},"array":[{"z":1,"a":2},"unchanged",3],"a":true}'
    ),
    `{
  "a": true,
  "array": [
    {
      "a": 2,
      "z": 1
    },
    "unchanged",
    3
  ],
  "z": {
    "first": 1,
    "second": 2
  }
}\n`
  );
});

test("canonicalizes files atomically and is idempotent", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lingua-canonical-json-"));
  const filePath = join(directory, "discovery.json");

  try {
    await writeFile(filePath, '{"z":1,"a":{"z":2,"a":3}}');

    assert.equal(await canonicalizeJsonFile(filePath), true);
    const canonical = `{
  "a": {
    "a": 3,
    "z": 2
  },
  "z": 1
}\n`;
    assert.equal(await readFile(filePath, "utf8"), canonical);

    assert.equal(await canonicalizeJsonFile(filePath), false);
    assert.equal(await readFile(filePath, "utf8"), canonical);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("canonicalizes JSON without changing number precision", () => {
  const source =
    '{"z":9007199254740993,"negativeZero":-0,"exponent":1e+400,"decimal":0.12345678901234567890123456789}';

  assert.equal(
    canonicalizeJson(source),
    `{
  "decimal": 0.12345678901234567890123456789,
  "exponent": 1e+400,
  "negativeZero": -0,
  "z": 9007199254740993
}\n`
  );
});

test("leaves the original file untouched when JSON parsing fails", async () => {
  const directory = await mkdtemp(join(tmpdir(), "lingua-invalid-json-"));
  const filePath = join(directory, "discovery.json");
  const invalidJson = '{"not":"complete"';

  try {
    await writeFile(filePath, invalidJson);
    await assert.rejects(canonicalizeJsonFile(filePath), SyntaxError);
    assert.equal(await readFile(filePath, "utf8"), invalidJson);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("ignores an expected missing temporary file during cleanup", async () => {
  const originalError = new Error("write failed");
  const missingError = Object.assign(new Error("file not found"), {
    code: "ENOENT",
  });

  await cleanupTempFile("temporary.json", originalError, async () => {
    throw missingError;
  });
});

test("combines the original and cleanup errors", async () => {
  const originalError = new Error("write failed");
  const cleanupError = Object.assign(new Error("permission denied"), {
    code: "EACCES",
  });

  await assert.rejects(
    cleanupTempFile("temporary.json", originalError, async () => {
      throw cleanupError;
    }),
    (error) => {
      assert(error instanceof AggregateError);
      assert.deepEqual(error.errors, [originalError, cleanupError]);
      assert.match(error.message, /write failed/);
      assert.match(error.message, /permission denied/);
      return true;
    }
  );
});

test("the provider pipeline canonicalizes only the Google JSON spec", async () => {
  const pipeline = await readFile(
    new URL("./generate-provider-types.sh", import.meta.url),
    "utf8"
  );

  assert.match(
    pipeline,
    /if \[ "\$PROVIDER" = "google" \]; then\s+node "\$SCRIPT_DIR\/canonicalize-json\.mjs" "\$SPEC_FILE"\s+fi/
  );
});

test("the checked-in Google Discovery spec is canonical", async () => {
  const specPath = new URL("../specs/google/discovery.json", import.meta.url);
  const source = await readFile(specPath, "utf8");
  const expected = canonicalizeJson(source);

  assert.equal(source, expected);
});
