import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseArgs } from "node:util";
import {
  runDisplay,
  runTrajectory,
  trajectoryCases,
  validateTrajectory,
} from "./workloads.mjs";

const { values } = parseArgs({
  options: {
    baseline: { type: "string" },
    candidate: { type: "string" },
    samples: { type: "string", default: "100" },
    warmup: { type: "string", default: "20" },
    filter: { type: "string" },
    suite: { type: "string", default: "all" },
    target: { type: "string", default: "node" },
  },
});

const samples = Number(values.samples);
const warmup = Number(values.warmup);
assert.ok(
  Number.isInteger(samples) && samples > 0,
  "Invalid --samples: expected a positive integer"
);
assert.ok(
  Number.isInteger(warmup) && warmup >= 0,
  "Invalid --warmup: expected a nonnegative integer"
);
assert.ok(
  ["import", "display", "trajectory", "all"].includes(values.suite),
  "Invalid --suite: expected import, display, trajectory, or all"
);
assert.ok(
  ["node", "browser"].includes(values.target),
  "Invalid --target: expected node or browser"
);

async function loadWasm(path) {
  const url = path
    ? pathToFileURL(resolve(path))
    : new URL(
        import.meta.resolve(
          values.target === "browser"
            ? "@braintrust/lingua-wasm/browser"
            : "@braintrust/lingua-wasm"
        )
      );
  try {
    const wasm = await import(url.href);
    if (values.target === "browser") {
      assert.equal(
        typeof wasm.default,
        "function",
        "Expected web bindings for --target browser"
      );
      await wasm.default({
        module_or_path: readFileSync(new URL("./lingua_bg.wasm", url)),
      });
    } else {
      assert.ok(
        !("initSync" in wasm),
        "Expected Node bindings for --target node"
      );
    }
    const required = ["import_messages_from_spans"];
    if (values.baseline) required.push("import_and_deduplicate_messages");
    if (["all", "display"].includes(values.suite))
      required.push("lingua_to_chat_completions_messages");
    if (["all", "trajectory"].includes(values.suite))
      required.push("import_span", "TrajectoryStream", "TrajectoryCollector");
    for (const name of required)
      assert.equal(
        typeof wasm[name],
        "function",
        `Missing WASM export ${name}`
      );
    return wasm;
  } catch (error) {
    throw new Error(
      `Cannot load ${values.target} WASM build ${url.pathname}: ${String(error)}`,
      { cause: error }
    );
  }
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
const importCases = [
  {
    name: "Chat Completions",
    expectedMessages: 2,
    spans: fixture("chat-completions-basic.spans.json"),
    compatible: true,
  },
  {
    name: "Responses tool loop",
    expectedMessages: 4,
    spans: fixture("openai-responses-real-world-tool-loop.spans.json"),
    compatible: true,
  },
  {
    name: "Anthropic tool use",
    expectedMessages: 1,
    spans: fixture("anthropic-tool-use-blocks.spans.json"),
    compatible: true,
  },
  {
    name: "Gemini",
    expectedMessages: 1,
    spans: fixture("gemini-basic-generate-content.spans.json"),
    compatible: true,
  },
  {
    name: "Pydantic",
    expectedMessages: 2,
    spans: fixture("pydantic-ai-wrapper-user-prompt.spans.json"),
    compatible: true,
  },
  {
    name: "LangChain",
    expectedMessages: 2,
    spans: fixture("langchain-human-ai-basic.spans.json"),
    compatible: true,
  },
  {
    name: "AI SDK tool use",
    expectedMessages: 8,
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

const cases = [
  ...importCases.map((workload) => ({ ...workload, suite: "import" })),
  ...importCases
    .filter((workload) => workload.expectedMessages !== undefined)
    .map((workload) => ({ ...workload, suite: "display" })),
  ...trajectoryCases(),
];

function run(wasm, workload, validate = false) {
  try {
    switch (workload.suite) {
      case "import":
        return { output: wasm.import_messages_from_spans(workload.spans) };
      case "display":
        return { output: runDisplay(wasm, workload.spans) };
      case "trajectory": {
        const result = runTrajectory(wasm, workload, validate);
        if (validate) validateTrajectory(result.output, workload.fixture);
        return result;
      }
      default:
        throw new Error(`Unknown suite ${workload.suite}`);
    }
  } catch (error) {
    throw new Error(`${workload.suite}: ${workload.name}: ${String(error)}`, {
      cause: error,
    });
  }
}

function trajectoryCounts(trajectories) {
  let turns = 0;
  let work = 0;
  for (const trajectory of trajectories)
    for (const turn of trajectory.turns) {
      turns++;
      work += turn.work.length;
      for (const step of turn.work)
        if (step.sub_agent) {
          const child = trajectoryCounts([step.sub_agent]);
          turns += child.turns;
          work += child.work;
        }
    }
  return { turns, work };
}

function summarize(times) {
  times.sort((a, b) => a - b);
  return {
    medianMs: times[Math.floor(times.length / 2)],
    p95Ms: times[Math.min(times.length - 1, Math.floor(times.length * 0.95))],
  };
}

const rows = [];
const filter = new RegExp(values.filter ?? "");
const selectedCases = cases.filter(
  ({ name, suite }) =>
    (values.suite === "all" || values.suite === suite) &&
    (filter.test(name) || filter.test(`${suite}: ${name}`))
);
assert.ok(selectedCases.length > 0, "No benchmark cases matched the filter");
for (const workload of selectedCases) {
  const { name, spans, suite } = workload;
  const result = run(candidate, workload, true);
  if (suite === "import")
    assert.equal(result.output.length > 0, workload.compatible, name);
  if (suite === "display") {
    assert.equal(
      result.output.reduce(
        (count, span) => count + span.input.length + span.output.length,
        0
      ),
      workload.expectedMessages,
      `${name}: display message count`
    );
    for (const span of result.output)
      for (const message of [...span.input, ...span.output]) {
        assert.equal(typeof message.role, "string", name);
      }
  }
  if (baseline) {
    const previous = run(baseline, workload, true);
    assert.deepEqual(
      result.output,
      previous.output,
      `${suite}: ${name}: complete output parity`
    );
    if (suite === "import") {
      assert.deepEqual(
        candidate.import_and_deduplicate_messages(spans),
        baseline.import_and_deduplicate_messages(spans),
        `${name}: import_and_deduplicate_messages`
      );
    }
  }

  for (let i = 0; i < warmup; i++) {
    if (baseline) run(baseline, workload);
    run(candidate, workload);
  }
  globalThis.gc?.();

  const candidateTimes = [];
  const baselineTimes = [];
  const candidateFirstTurn = [];
  const baselineFirstTurn = [];
  const runs = [
    { wasm: baseline, times: baselineTimes, firstTurn: baselineFirstTurn },
    { wasm: candidate, times: candidateTimes, firstTurn: candidateFirstTurn },
  ];
  for (let i = 0; i < samples; i++) {
    runs.reverse();
    for (const { wasm, times, firstTurn } of runs) {
      if (!wasm) continue;
      const start = performance.now();
      const sample = run(wasm, workload);
      times.push(performance.now() - start);
      if (sample.firstTurnMs !== undefined) firstTurn.push(sample.firstTurnMs);
    }
  }

  const current = summarize(candidateTimes);
  const previous = baseline ? summarize(baselineTimes) : undefined;
  rows.push({
    suite,
    target: values.target,
    case: name,
    bytes: Buffer.byteLength(JSON.stringify(spans)),
    spans: spans.length,
    ...(suite === "trajectory"
      ? {
          ...trajectoryCounts(result.output),
          events: result.events,
          snapshots: result.snapshots,
          "first turn median ms": candidateFirstTurn.length
            ? summarize(candidateFirstTurn).medianMs.toFixed(4)
            : undefined,
          "first turn p95 ms": candidateFirstTurn.length
            ? summarize(candidateFirstTurn).p95Ms.toFixed(4)
            : undefined,
          "baseline first turn median ms": baselineFirstTurn.length
            ? summarize(baselineFirstTurn).medianMs.toFixed(4)
            : undefined,
          "baseline first turn p95 ms": baselineFirstTurn.length
            ? summarize(baselineFirstTurn).p95Ms.toFixed(4)
            : undefined,
        }
      : {
          messages:
            suite === "import"
              ? result.output.length
              : result.output.reduce(
                  (count, span) =>
                    count + span.input.length + span.output.length,
                  0
                ),
        }),
    "baseline median ms": previous?.medianMs.toFixed(4),
    "candidate median ms": current.medianMs.toFixed(4),
    "baseline p95 ms": previous?.p95Ms.toFixed(4),
    "candidate p95 ms": current.p95Ms.toFixed(4),
    speedup: previous
      ? `${(previous.medianMs / current.medianMs).toFixed(2)}x`
      : undefined,
  });
}

if (baseline)
  console.log(
    `Complete output parity: ${selectedCases.length} selected workloads`
  );
console.log(`Samples: ${samples}; warmup: ${warmup}; Node: ${process.version}`);
for (const suite of ["import", "display", "trajectory"]) {
  const selected = rows.filter((row) => row.suite === suite);
  if (selected.length) console.table(selected);
}
