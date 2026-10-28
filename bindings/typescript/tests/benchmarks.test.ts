import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const cwd = fileURLToPath(new URL("../", import.meta.url));

function benchmark(...args: string[]) {
  return spawnSync(
    process.execPath,
    [
      "--expose-gc",
      "benchmarks/import-spans.mjs",
      "--samples",
      "2",
      "--warmup",
      "1",
      ...args,
    ],
    { cwd, encoding: "utf8", timeout: 30_000 }
  );
}

test.each(["node", "browser"])("%s benchmark runs all suites", (target) => {
  const result = benchmark(
    "--target",
    target,
    "--filter",
    "Chat Completions|chat-tool-cycle"
  );
  expect(result.stderr, result.stdout).not.toContain("Error");
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain("'import'");
  expect(result.stdout).toContain("'display'");
  expect(result.stdout).toContain("'trajectory'");
  expect(result.stdout).toContain("first turn median ms");
});

test.each(["node", "browser"])(
  "%s benchmark compares identical builds",
  (target) => {
    const build = target === "node" ? "nodejs" : "web";
    const result = benchmark(
      "--target",
      target,
      "--baseline",
      `../lingua-wasm/${build}/lingua.js`,
      "--candidate",
      `../lingua-wasm/${build}/lingua.js`,
      "--filter",
      "Chat Completions|chat-tool-cycle"
    );
    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout).toContain("Complete output parity:");
    expect(result.stdout).toContain("speedup");
  }
);

test.each(["import", "display", "trajectory"])(
  "selects only the %s suite",
  (suite) => {
    const result = benchmark(
      "--suite",
      suite,
      "--filter",
      "Chat Completions|chat-tool-cycle"
    );
    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout).toContain(`'${suite}'`);
    for (const other of ["import", "display", "trajectory"].filter(
      (value) => value !== suite
    )) {
      expect(result.stdout).not.toContain(`'${other}'`);
    }
  }
);

test("preserves anchored case-name filters", () => {
  const result = benchmark(
    "--suite",
    "display",
    "--filter",
    "^Chat Completions$"
  );
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain("'Chat Completions'");
  expect(result.stdout).not.toContain("'Anthropic tool use'");
});

test("Node and browser builds produce identical display messages and progressive trajectories", () => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "--eval",
      `
    import assert from "node:assert/strict";
    import { readFileSync } from "node:fs";
    import * as node from "@braintrust/lingua-wasm";
    import * as browser from "@braintrust/lingua-wasm/browser";
    import { runDisplay, runTrajectory, trajectoryCases, validateTrajectory } from "./benchmarks/workloads.mjs";

    await browser.default({ module_or_path: readFileSync(new URL(import.meta.resolve("@braintrust/lingua-wasm/browser/lingua_bg.wasm"))) });
    const spans = [{
      input: [{ role: "user", content: "What is the weather?" }],
      output: { role: "assistant", content: [{ type: "tool_use", id: "call_weather", name: "weather", input: { city: "Paris" } }] },
    }];
    const expected = [{
      input: [{ role: "user", content: "What is the weather?" }],
      output: [{ role: "assistant", tool_calls: [{ type: "function", id: "call_weather", function: { name: "weather", arguments: '{"city":"Paris"}' } }] }],
    }];
    for (const wasm of [node, browser]) assert.deepEqual(runDisplay(wasm, spans), expected);
    assert.throws(() => runDisplay(browser, [{ input: "Hello", metadata: () => null }]));

    const byBatch = new Map();
    for (const workload of trajectoryCases()) {
      assert.equal(new Set(workload.spans.map((span) => span.id)).size, workload.spans.length);
      const reference = runTrajectory(node, workload, true);
      const current = runTrajectory(browser, workload, true);
      validateTrajectory(reference.output, workload.fixture);
      validateTrajectory(current.output, workload.fixture);
      assert.deepEqual(current.output, reference.output, workload.name);
      assert.deepEqual(runTrajectory(browser, workload).output, current.output, workload.name);
      assert.ok(Number.isFinite(current.firstTurnMs));
      assert.ok(current.events > 0 && current.snapshots >= 2);
      const name = workload.name.replace(/, batch \\d+$/, "");
      if (byBatch.has(name)) assert.deepEqual(current.output, byBatch.get(name), workload.name);
      else byBatch.set(name, current.output);
    }
    console.log("Validated display conversion and 18 trajectory workloads across both builds");
  `,
    ],
    { cwd, encoding: "utf8", timeout: 30_000 }
  );
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain(
    "Validated display conversion and 18 trajectory workloads"
  );
});

test.each([
  ["--suite", "unknown"],
  ["--target", "unknown"],
  ["--samples", "0"],
  ["--warmup", "-1"],
  ["--filter", "no-such-benchmark"],
  ["--candidate", "missing-lingua-build.js"],
  ["--candidate", "dist/index.mjs"],
  ["--target", "browser", "--candidate", "../lingua-wasm/nodejs/lingua.js"],
  ["--target", "node", "--candidate", "../lingua-wasm/web/lingua.js"],
])("rejects invalid benchmark arguments %j", (...args) => {
  const result = benchmark(...args);
  expect(result.status).not.toBe(0);
  expect(result.stderr).toMatch(/Error|Invalid|Missing/);
});
