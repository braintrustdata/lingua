import { readFileSync, readdirSync } from "node:fs";
import { gzipSync } from "node:zlib";
import * as full from "@braintrust/lingua-wasm/full";
import { beforeAll, expect, test } from "vitest";
import * as node from "../src/index";
import * as browser from "../src/index.browser";

const nodeWasm = node.getWasm();
const bytes = readFileSync(
  new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)
);

beforeAll(async () => {
  await browser.init(bytes);
});

test("the standard WASM API covers Braintrust message and trajectory calls", () => {
  for (const wasm of [nodeWasm, browser.getWasm()]) {
    expect("validate_chat_completions_request" in wasm).toBe(false);
    expect("transform_request" in wasm).toBe(false);
    expect("TransformStreamSession" in wasm).toBe(false);
    expect(
      browser.linguaToChatCompletionsMessages(
        browser.importMessagesFromSpans([
          { input: [{ role: "user", content: "Hello" }] },
        ])
      )
    ).toEqual([{ role: "user", content: "Hello" }]);
    expect(wasm.trajectory_metadata_fields()).toContain("model");
    const span = wasm.import_span({
      id: "span",
      span_id: "span",
      root_span_id: "span",
    });
    expect(span).toBeInstanceOf(wasm.ImportedSpan);
    span.free();
    const stream = new wasm.TrajectoryStream([], false);
    const collector = new wasm.TrajectoryCollector();
    try {
      for (const event of stream.finish()) collector.push(event);
      expect(collector.isComplete()).toBe(true);
      expect(collector.snapshot()).toEqual([]);
    } finally {
      stream.free();
      collector.free();
    }
  }
});

test("the default build matches the full build for every import fixture and both APIs", () => {
  const directory = new URL("../../../payloads/import-cases/", import.meta.url);
  const fixtures = readdirSync(directory).filter(
    (name) => name.endsWith(".json") && !name.endsWith(".assertions.json")
  );
  expect(fixtures.length).toBeGreaterThan(0);
  for (const name of fixtures) {
    const value: unknown = JSON.parse(
      readFileSync(new URL(name, directory), "utf8")
    );
    const spans: unknown[] = Array.isArray(value) ? value : [value];
    for (const wasm of [nodeWasm, browser.getWasm()]) {
      expect(wasm.import_messages_from_spans(spans), name).toEqual(
        full.import_messages_from_spans(spans)
      );
      expect(wasm.import_and_deduplicate_messages(spans), name).toEqual(
        full.import_and_deduplicate_messages(spans)
      );
    }
  }
});

test("the standard WASM artifact is smaller than the full build", () => {
  const fullBytes = readFileSync(
    new URL("../../lingua-wasm/nodejs-full/lingua_bg.wasm", import.meta.url)
  );
  expect(bytes.length).toBeLessThan(fullBytes.length);
  expect(gzipSync(bytes).length).toBeLessThan(gzipSync(fullBytes).length);
});
