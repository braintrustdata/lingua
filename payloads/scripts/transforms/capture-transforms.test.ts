import { describe, expect, test } from "vitest";
import {
  parseCaptureTransformArgs,
  selectRequestedCases,
} from "./capture-transforms";

describe("capture transform arguments", () => {
  test("keeps the first positional argument as the filter without a pair", () => {
    expect(parseCaptureTransformArgs(["streamParam", "--force"])).toEqual({
      filter: "streamParam",
      force: true,
      pair: undefined,
      cases: undefined,
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
      cases: undefined,
    });
  });

  test("parses an exact comma-separated case list", () => {
    expect(
      parseCaptureTransformArgs([
        "--cases",
        "streamParam,streamOptionsParam",
        "--force",
      ])
    ).toEqual({
      filter: undefined,
      force: true,
      pair: undefined,
      cases: ["streamParam", "streamOptionsParam"],
    });
  });
});

describe("capture transform case selection", () => {
  test("selects requested cases by exact name", () => {
    expect(
      selectRequestedCases(
        ["streamParam", "streamOptionsParam", "otherParam"],
        ["streamParam"]
      )
    ).toEqual(["streamParam"]);
  });

  test("keeps all cases when no exact case list is provided", () => {
    expect(selectRequestedCases(["streamParam", "otherParam"])).toEqual([
      "streamParam",
      "otherParam",
    ]);
  });
});
