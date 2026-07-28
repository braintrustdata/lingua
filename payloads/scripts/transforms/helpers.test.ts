import { describe, expect, test } from "vitest";
import {
  STREAMING_PAIRS,
  TRANSFORM_PAIRS,
  getStreamingTransformableCases,
  getTransformableCases,
  type TransformPair,
} from "./helpers";

function findPair(
  pairs: TransformPair[],
  source: TransformPair["source"],
  target: TransformPair["target"]
): TransformPair {
  const pair = pairs.find(
    (candidate) => candidate.source === source && candidate.target === target
  );
  if (!pair) {
    throw new Error(`Missing transform pair: ${source} -> ${target}`);
  }
  return pair;
}

describe("transform case selection", () => {
  const explicitlyStreamingCases = [
    "streamParam",
    "anthropicOpus5AdaptiveThinkingMaxEffortParam",
  ];

  test("excludes explicitly streaming cases from non-streaming transforms", () => {
    for (const target of ["chat-completions", "responses", "google"] as const) {
      const pair = findPair(TRANSFORM_PAIRS, "anthropic", target);
      expect(getTransformableCases(pair)).not.toEqual(
        expect.arrayContaining(explicitlyStreamingCases)
      );
    }
  });

  test("includes explicitly streaming parameter cases in streaming transforms", () => {
    for (const target of ["chat-completions", "responses", "google"] as const) {
      const pair = findPair(STREAMING_PAIRS, "anthropic", target);
      expect(getStreamingTransformableCases(pair)).toEqual(
        expect.arrayContaining(explicitlyStreamingCases)
      );
    }
  });

  test("limits explicit-only streaming pairs to opted-in cases", () => {
    for (const target of ["responses", "google"] as const) {
      const pair = findPair(STREAMING_PAIRS, "anthropic", target);
      expect(getStreamingTransformableCases(pair)).not.toContain(
        "simpleRequest"
      );
    }
  });
});
