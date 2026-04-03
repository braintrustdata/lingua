import { describe, test, expect } from "vitest";
import { createOpenAI } from "@ai-sdk/openai";
import { generateText } from "ai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  TARGET_MODELS,
  getTransformableCases,
  getResponsePath,
  getFixtureSkipReason,
  registerSkippedFixtureTest,
  useTransformTestServer,
} from "./helpers";

const TIMEOUT = 30000;
const getServer = useTransformTestServer();

const RESPONSES_TO_ANTHROPIC_PAIRS = TRANSFORM_PAIRS.filter(
  (p) => p.source === "responses" && p.target === "anthropic"
);

for (const pair of RESPONSES_TO_ANTHROPIC_PAIRS) {
  const pairLabel = `${pair.target} response → ${pair.source} format`;
  describe(`AI SDK validation: ${pairLabel}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(responsePath);

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          getServer().useJsonFixture({
            path: "/v1/responses",
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath,
          });

          const targetCase = getCaseForProvider(
            allTestCases,
            caseName,
            pair.source
          );
          const model =
            targetCase &&
            typeof targetCase === "object" &&
            "model" in targetCase
              ? String(targetCase.model)
              : TARGET_MODELS[pair.source];

          const provider = createOpenAI({
            apiKey: "test-key",
            baseURL: getServer().openaiBaseUrl,
          });

          await expect(
            generateText({
              model: provider.responses(model),
              prompt: "test",
            })
          ).resolves.toBeDefined();
        },
        TIMEOUT
      );
    }
  });
}
