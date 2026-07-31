import { describe, expect, test } from "vitest";
import { findUnmatchedRequestedCases } from "./capture";

describe("requested capture case validation", () => {
  test("rejects every requested case omitted by provider selection", () => {
    expect(
      findUnmatchedRequestedCases(
        ["supportedCase", "incompatibleCase", "misspelledCase"],
        ["supportedCase"]
      )
    ).toEqual(["incompatibleCase", "misspelledCase"]);
  });

  test("accepts requested cases matched by at least one selected provider", () => {
    expect(
      findUnmatchedRequestedCases(
        ["sharedCase", "providerCase"],
        ["sharedCase", "sharedCase", "providerCase"]
      )
    ).toEqual([]);
  });

  test("does not require explicit cases for unfiltered captures", () => {
    expect(findUnmatchedRequestedCases(undefined, [])).toEqual([]);
  });
});
