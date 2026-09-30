import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import fixture from "../../../crates/lingua/src/processing/trajectory/fixtures/responses-tool-cycle.json";

test.each(["node", "browser"])("%s entry point assembles a streamed trajectory", async (target) => {
  const entry =
    target === "node"
      ? await import("@braintrust/lingua")
      : await import("@braintrust/lingua/browser");
  if ("init" in entry) {
    await entry.init(
      readFileSync(new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)),
    );
  }
  const lingua = entry.getWasm();
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
