import { readFileSync, readdirSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { beforeAll, expect, test } from "vitest";
import * as full from "../src/index";
import * as trace from "../src/index.trace";
import * as converters from "../src/converters";
import { type Message } from "../src/types";

const traceBytes = readFileSync(
  new URL("../../lingua-wasm/web-trace/lingua_bg.wasm", import.meta.url)
);

beforeAll(async () => {
  await Promise.all([trace.init(traceBytes), trace.init(traceBytes)]);
});

test("keeps the full and trace runtimes independent", () => {
  expect(full.getWasm()).not.toBe(trace.getWasm());
  expect("validate_chat_completions_request" in trace.getWasm()).toBe(false);
  expect("transform_request" in trace.getWasm()).toBe(false);
  expect("TransformStreamSession" in trace.getWasm()).toBe(false);
  expect("validateChatCompletionsRequest" in trace).toBe(false);
  expect("createTransformStreamSession" in trace).toBe(false);
  expect(
    full.validateChatCompletionsRequest(
      JSON.stringify({ model: "gpt-5-mini", messages: [] })
    ).ok
  ).toBe(true);
});

test("converts messages through every provider", () => {
  const messages: Message[] = [{ role: "user", content: "Hello" }];
  const providers = [
    {
      expected: full.linguaToChatCompletionsMessages(messages),
      actual: trace.linguaToChatCompletionsMessages(messages),
      toLingua: trace.chatCompletionsMessagesToLingua,
    },
    {
      expected: full.linguaToResponsesMessages(messages),
      actual: trace.linguaToResponsesMessages(messages),
      toLingua: trace.responsesMessagesToLingua,
    },
    {
      expected: full.linguaToAnthropicMessages(messages),
      actual: trace.linguaToAnthropicMessages(messages),
      toLingua: trace.anthropicMessagesToLingua,
    },
    {
      expected: converters.linguaToGoogleContents(messages),
      actual: trace.linguaToGoogleContents(messages),
      toLingua: trace.googleContentsToLingua,
    },
  ];
  for (const provider of providers) {
    expect(provider.actual).toEqual(provider.expected);
    expect(provider.toLingua(provider.actual)).toEqual(messages);
  }
});

test("imports and deduplicates spans through the typed wrappers", () => {
  const spans = [{ input: [{ role: "user", content: "Hello" }] }];
  const messages = full.importMessagesFromSpans(spans);
  expect(trace.importMessagesFromSpans(spans)).toEqual(messages);
  expect(trace.importAndDeduplicateMessages(spans)).toEqual(
    full.importAndDeduplicateMessages(spans)
  );
  expect(trace.deduplicateMessages([...messages, ...messages])).toEqual(
    messages
  );
});

test("matches the full build across every import fixture and both APIs", () => {
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
    expect(trace.getWasm().import_messages_from_spans(spans), name).toEqual(
      full.getWasm().import_messages_from_spans(spans)
    );
    expect(
      trace.getWasm().import_and_deduplicate_messages(spans),
      name
    ).toEqual(full.getWasm().import_and_deduplicate_messages(spans));
  }
});

test("ships a smaller WASM artifact", () => {
  const fullBytes = readFileSync(
    new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)
  );
  expect(traceBytes.length).toBeLessThan(fullBytes.length);
  expect(gzipSync(traceBytes).length).toBeLessThan(gzipSync(fullBytes).length);
});
