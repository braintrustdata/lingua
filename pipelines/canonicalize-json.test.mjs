import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { canonicalizeJsonFile, sortJsonKeys } from "./canonicalize-json.mjs";

test("sorts object keys recursively while preserving array order", () => {
  assert.deepEqual(
    sortJsonKeys({
      z: { second: 2, first: 1 },
      array: [{ z: 1, a: 2 }, "unchanged", 3],
      a: true,
    }),
    {
      a: true,
      array: [{ a: 2, z: 1 }, "unchanged", 3],
      z: { first: 1, second: 2 },
    }
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
  const expected = `${JSON.stringify(sortJsonKeys(JSON.parse(source)), null, 2)}\n`;

  assert.equal(source, expected);
});
