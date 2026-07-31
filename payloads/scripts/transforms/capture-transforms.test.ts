import { describe, expect, test } from "vitest";
import {
  isCaptureProviderSelected,
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
      captureProviders: undefined,
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
      captureProviders: undefined,
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
      captureProviders: undefined,
    });
  });

  test("parses credentialed capture providers without treating them as a filter", () => {
    expect(
      parseCaptureTransformArgs([
        "--capture-providers",
        "anthropic,chat-completions,responses,google",
        "--cases",
        "streamParam",
        "--force",
      ])
    ).toEqual({
      filter: undefined,
      force: true,
      pair: undefined,
      cases: ["streamParam"],
      captureProviders: [
        "anthropic",
        "chat-completions",
        "responses",
        "google",
      ],
    });
  });
});

describe("capture provider selection", () => {
  test("keeps pairs whose actual capture provider is credentialed", () => {
    expect(
      isCaptureProviderSelected({ target: "anthropic" }, [
        "anthropic",
        "google",
      ])
    ).toBe(true);
    expect(
      isCaptureProviderSelected(
        { target: "anthropic", captureProvider: "baseten" },
        ["anthropic", "google"]
      )
    ).toBe(false);
  });

  test("keeps every pair when no credential filter is provided", () => {
    expect(isCaptureProviderSelected({ target: "vertex-anthropic" })).toBe(
      true
    );
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
