import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import fixture from "../../../crates/lingua/src/processing/voice/fixtures/livekit-call.json";

test.each(["node", "browser"])(
  "%s reads the original LiveKit fixture",
  async (target) => {
    const entry =
      target === "node"
        ? await import("@braintrust/lingua")
        : await import("@braintrust/lingua/browser");
    if ("init" in entry) {
      await entry.init(
        readFileSync(
          new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)
        )
      );
    }
    const call = entry.importVoiceCall(fixture);
    expect(call).toMatchSnapshot();
    expect(entry.importVoiceCall([...fixture].reverse())).toEqual(call);
    expect(entry.importVoiceCall([])).toEqual({
      recordings: [],
      utterances: [],
      messages: [],
    });
    expect(() =>
      entry.importVoiceCall([
        { span_id: "invalid", metadata: { "lk.interrupted": "yes" } },
      ])
    ).toThrow();
  }
);
