import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    baseline: { type: "string" },
    candidate: { type: "string" },
    samples: { type: "string", default: "100" },
    warmup: { type: "string", default: "20" },
    filter: { type: "string" },
  },
});

const samples = Number(values.samples);
const warmup = Number(values.warmup);
assert.ok(Number.isInteger(samples) && samples > 0);
assert.ok(Number.isInteger(warmup) && warmup >= 0);

async function loadWasm(path) {
  const wasm = path
    ? await import(pathToFileURL(resolve(path)).href)
    : await import("@braintrust/lingua-wasm");
  assert.equal(typeof wasm.import_messages_from_spans, "function");
  return wasm;
}

const candidate = await loadWasm(values.candidate);
const baseline = values.baseline ? await loadWasm(values.baseline) : undefined;
const fixtureDir = fileURLToPath(
  new URL("../../../payloads/import-cases/", import.meta.url)
);
const fixtures = readdirSync(fixtureDir)
  .filter(
    (name) => name.endsWith(".json") && !name.endsWith(".assertions.json")
  )
  .sort()
  .map((name) => {
    const value = JSON.parse(readFileSync(resolve(fixtureDir, name), "utf8"));
    return { name, spans: Array.isArray(value) ? value : [value] };
  });

function fixture(name) {
  const found = fixtures.find((entry) => entry.name === name);
  assert.ok(found, `Missing fixture: ${name}`);
  return found.spans;
}

if (baseline) {
  for (const { name, spans } of fixtures) {
    for (const operation of [
      "import_messages_from_spans",
      "import_and_deduplicate_messages",
    ]) {
      assert.deepEqual(
        candidate[operation](spans),
        baseline[operation](spans),
        `${name}: ${operation}`
      );
    }
  }
  console.log(`Complete output parity: ${fixtures.length} fixtures, both APIs`);
}

const sourceText = readFileSync(
  resolve(fixtureDir, "chat-completions-basic.spans.json"),
  "utf8"
);
const largeText = sourceText
  .repeat(Math.ceil((4 * 1024 * 1024) / sourceText.length))
  .slice(0, 4 * 1024 * 1024);
const unsupported = { unrelated: largeText };
const cases = [
  {
    name: "Chat Completions",
    spans: fixture("chat-completions-basic.spans.json"),
    compatible: true,
  },
  {
    name: "Responses tool loop",
    spans: fixture("openai-responses-real-world-tool-loop.spans.json"),
    compatible: true,
  },
  {
    name: "Anthropic tool use",
    spans: fixture("anthropic-tool-use-blocks.spans.json"),
    compatible: true,
  },
  {
    name: "Gemini",
    spans: fixture("gemini-basic-generate-content.spans.json"),
    compatible: true,
  },
  {
    name: "Pydantic",
    spans: fixture("pydantic-ai-wrapper-user-prompt.spans.json"),
    compatible: true,
  },
  {
    name: "LangChain",
    spans: fixture("langchain-human-ai-basic.spans.json"),
    compatible: true,
  },
  {
    name: "AI SDK tool use",
    spans: fixture("ai-sdk-multi-round-tool-use.spans.json"),
    compatible: true,
  },
  {
    name: "All import fixtures",
    spans: fixtures.flatMap(({ spans }) => spans),
    compatible: true,
  },
  {
    name: "LLM text scaled to 4 MiB",
    spans: [{ input: [{ role: "user", content: largeText }] }],
    compatible: true,
  },
  {
    name: "LLM span with 4 MiB extra field",
    spans: fixture("chat-completions-basic.spans.json").map((span) => ({
      ...span,
      unrelated: largeText,
    })),
    compatible: true,
  },
  {
    name: "Gemini with 4 MiB extra input field",
    spans: fixture("gemini-basic-generate-content.spans.json").map((span) => ({
      ...span,
      input: { ...span.input, unrelated: largeText },
    })),
    compatible: true,
  },
  {
    name: "Unsupported 4 MiB object",
    spans: [{ input: unsupported }],
    compatible: false,
  },
  {
    name: "Primitive input and output",
    spans: [{ input: 42, output: false }],
    compatible: false,
  },
  {
    name: "Primitive array",
    spans: [{ input: Array.from({ length: 1024 }, (_, index) => index) }],
    compatible: false,
  },
  {
    name: "Non-message fixture",
    spans: fixture("no-messages.spans.json"),
    compatible: false,
  },
  {
    name: "Empty fixture",
    spans: fixture("empty-spans.spans.json"),
    compatible: false,
  },
  {
    name: "Unsupported object array",
    spans: [
      { input: Array.from({ length: 100 }, () => ({ unrelated: sourceText })) },
    ],
    compatible: false,
  },
  {
    name: "Invalid role with 4 MiB content",
    spans: [{ input: [{ role: "invalid", content: largeText }] }],
    compatible: false,
  },
  {
    name: "Unsupported 4 MiB input with LLM output",
    spans: [{ input: unsupported, output: "Hello" }],
    compatible: true,
  },
];

function summarize(times) {
  times.sort((a, b) => a - b);
  return {
    medianMs: times[Math.floor(times.length / 2)],
    p95Ms: times[Math.min(times.length - 1, Math.floor(times.length * 0.95))],
  };
}

const rows = [];
const filter = new RegExp(values.filter ?? "");
const selectedCases = cases.filter(({ name }) => filter.test(name));
assert.ok(selectedCases.length > 0, "No benchmark cases matched the filter");
for (const { name, spans, compatible } of selectedCases) {
  const messages = candidate.import_messages_from_spans(spans);
  assert.equal(messages.length > 0, compatible, name);
  if (baseline) {
    for (const operation of [
      "import_messages_from_spans",
      "import_and_deduplicate_messages",
    ]) {
      assert.deepEqual(
        candidate[operation](spans),
        baseline[operation](spans),
        `${name}: ${operation}`
      );
    }
  }

  for (let i = 0; i < warmup; i++) {
    baseline?.import_messages_from_spans(spans);
    candidate.import_messages_from_spans(spans);
  }
  globalThis.gc?.();

  const candidateTimes = [];
  const baselineTimes = [];
  const runs = [
    { wasm: baseline, times: baselineTimes },
    { wasm: candidate, times: candidateTimes },
  ];
  for (let i = 0; i < samples; i++) {
    runs.reverse();
    for (const { wasm, times } of runs) {
      if (!wasm) continue;
      const start = performance.now();
      wasm.import_messages_from_spans(spans);
      times.push(performance.now() - start);
    }
  }

  const current = summarize(candidateTimes);
  const previous = baseline ? summarize(baselineTimes) : undefined;
  rows.push({
    case: name,
    bytes: Buffer.byteLength(JSON.stringify(spans)),
    messages: messages.length,
    "baseline median ms": previous?.medianMs.toFixed(4),
    "candidate median ms": current.medianMs.toFixed(4),
    "baseline p95 ms": previous?.p95Ms.toFixed(4),
    "candidate p95 ms": current.p95Ms.toFixed(4),
    speedup: previous
      ? `${(previous.medianMs / current.medianMs).toFixed(2)}x`
      : undefined,
  });
}

console.log(`Samples: ${samples}; warmup: ${warmup}; Node: ${process.version}`);
console.table(rows);
