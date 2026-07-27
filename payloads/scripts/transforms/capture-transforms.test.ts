import { describe, expect, test } from "vitest";
import { parseCaptureTransformArgs } from "./capture-transforms";

describe("capture transform arguments", () => {
  test("keeps the first positional argument as the filter without a pair", () => {
    expect(parseCaptureTransformArgs(["streamParam", "--force"])).toEqual({
      filter: "streamParam",
      force: true,
      pair: undefined,
    });
  });

  test("does not treat the pair value as the filter", () => {
    expect(
      parseCaptureTransformArgs([
        "--pair",
        "anthropic,responses",
        "streamParam",
        "--force",
      ])
    ).toEqual({
      filter: "streamParam",
      force: true,
      pair: { source: "anthropic", target: "responses" },
    });
  });
});
