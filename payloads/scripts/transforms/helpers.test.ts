import { describe, expect, test } from "vitest";
import { paramsCases } from "../../cases/params";
import {
  STREAMING_PAIRS,
  TRANSFORM_PAIRS,
  getStreamingTransformableCases,
  getTransformableCases,
  getTargetModelForCase,
  transformAndValidateRequest,
  type TransformPair,
} from "./helpers";

test("Haiku 5.5 Bedrock captures select Haiku 5.5 and omit sampling parameters", () => {
  const input = paramsCases.haiku55SamplingParams?.["chat-completions"];
  if (!input) {
    throw new Error("Missing Haiku 5.5 sampling regression input");
  }
  const model = getTargetModelForCase("bedrock", "haiku55SamplingParams");
  expect(model).toBe("global.anthropic.claude-haiku-5-5");
  const request = transformAndValidateRequest(
    input,
    "Converse",
    "bedrock",
    model
  );
  expect(request).toMatchObject({ modelId: model });
  expect(request).not.toHaveProperty("inferenceConfig.temperature");
  expect(request).not.toHaveProperty("inferenceConfig.topP");
});

test("Haiku 5.5 sampling captures exclude Vertex", () => {
  const pair = findPair(
    STREAMING_PAIRS,
    "chat-completions",
    "vertex-anthropic"
  );
  expect(getStreamingTransformableCases(pair)).not.toContain(
    "haiku55SamplingParams"
  );
  expect(getTransformableCases(pair)).not.toContain("haiku55SamplingParams");
  expect(getStreamingTransformableCases(pair)).toContain("simpleRequest");
  expect(paramsCases.haiku55SamplingParams?.["vertex-anthropic"]).toBeNull();
});

test("Haiku 5.5 requests omit deprecated sampling parameters", () => {
  const testCase = paramsCases.haiku55SamplingParams;
  if (!testCase?.["chat-completions"] || !testCase.anthropic) {
    throw new Error("Missing Haiku 5.5 sampling regression inputs");
  }
  const request = transformAndValidateRequest(
    testCase["chat-completions"],
    "Anthropic",
    "anthropic",
    testCase.anthropic.model
  );
  expect(request).toMatchObject({
    model: "claude-haiku-5-5",
    messages: [{ role: "user", content: "Say hi." }],
  });
  for (const parameter of ["temperature", "top_p", "top_k"]) {
    expect(request).not.toHaveProperty(parameter);
  }
});

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
