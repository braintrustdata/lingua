import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import fixture from "../../../crates/lingua/src/processing/trajectory/fixtures/responses-tool-cycle.json";
import voiceFixture from "../../../crates/lingua/src/processing/trajectory/fixtures/voice-cascade.json";

async function loadWasm(target: string) {
  const entry =
    target === "node"
      ? await import("@braintrust/lingua")
      : await import("@braintrust/lingua/browser");
  if ("init" in entry) {
    await entry.init(
      readFileSync(new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)),
    );
  }
  return entry.getWasm();
}

test.each(["node", "browser"])("%s entry point assembles a streamed trajectory", async (target) => {
  const lingua = await loadWasm(target);
  const stream = new lingua.TrajectoryStream(
    fixture.spans.map(({ input: _input, output: _output, ...header }) =>
      lingua.import_span(header),
    ),
    false,
  );
  const collector = new lingua.TrajectoryCollector();
  try {
    while (true) {
      const ids = stream.pendingIds(16);
      if (ids.length === 0) {
        break;
      }
      for (const id of ids.reverse()) {
        const span = fixture.spans.find((span) => span.id === id);
        for (const event of stream.push(lingua.import_span(span))) {
          collector.push(event);
        }
      }
    }
    for (const event of stream.finish()) {
      collector.push(event);
    }
    expect(collector.isComplete()).toBe(true);
    expect(collector.snapshot()).toMatchObject([
      {
        turns: [
          {
            request_id: "call",
            response_id: "final",
            work: [{ id: "call" }, { id: "tool" }],
          },
        ],
      },
    ]);
  } finally {
    stream.free();
    collector.free();
  }
});

test.each(["node", "browser"])("%s entry point preserves constructor import failures", async (target) => {
  const lingua = await loadWasm(target);
  const stream = new lingua.TrajectoryStream([], false, [
    { root_span_id: "root", span_id: "broken", message: "Invalid header" },
  ]);
  const collector = new lingua.TrajectoryCollector();
  try {
    for (const event of stream.finish()) {
      collector.push(event);
    }
    expect(collector.isComplete()).toBe(true);
    expect(collector.snapshot()).toMatchObject([
      {
        turns: [],
        metadata: {
          import_failures: [{ span_id: "broken", message: "Invalid header" }],
        },
      },
    ]);
  } finally {
    stream.free();
    collector.free();
  }
});

test.each(["node", "browser"])("%s entry point streams cascaded voice", async (target) => {
  const lingua = await loadWasm(target);
  const spans = voiceFixture.spans;
  const stream = new lingua.TrajectoryStream(
    spans.map(({ input: _input, output: _output, ...header }) => lingua.import_span(header)),
    false,
  );
  const collector = new lingua.TrajectoryCollector();
  try {
    while (true) {
      const ids = stream.pendingIds(8);
      if (ids.length === 0) {
        break;
      }
      for (const id of ids.reverse()) {
        const span = spans.find((span) => span.id === id);
        for (const event of stream.push(lingua.import_span(span))) {
          collector.push(event);
        }
      }
    }
    for (const event of stream.finish()) {
      collector.push(event);
    }
    expect(collector.isComplete()).toBe(true);
    const [trajectory] = collector.snapshot();
    expect(trajectory.voice.recordings).toEqual([]);
    expect(trajectory.voice.utterances.find((utterance) => utterance.id === "r-u1-b")).not.toHaveProperty("text");
    expect(trajectory.voice.utterances.find((utterance) => utterance.id === "r-a1-speech")).toMatchObject({
      speaker: "agent",
      interrupted: true,
      text: "The diagram shows: 1. User speaks. 2. Recognized",
      audio: [],
    });
    expect(trajectory.voice.turns.find((turn) => turn.id === "r-u1")).toMatchObject({
      utterance_ids: ["r-u1-a", "r-u1-b", "r-u1-c"],
      step_ids: ["r-stt1a", "r-stt1b"],
    });
    expect(trajectory.voice.links).toContainEqual({ from: "r-tool1", relationship: "requested_by", to: "r-llm1" });
    expect(trajectory.turns.length).toBeGreaterThan(0);
  } finally {
    stream.free();
    collector.free();
  }
});
