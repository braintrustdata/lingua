import { describe, test, expect } from "vitest";
import { existsSync } from "fs";
import { createOpenAI } from "@ai-sdk/openai";
import { generateText } from "ai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  TARGET_MODELS,
  loadAndValidateResponse,
  transformResponseData,
  getTransformableCases,
  getResponsePath,
  isErrorCapture,
  mockJsonFetch,
} from "./helpers";

const TIMEOUT = 30000;

const RESPONSES_TO_ANTHROPIC_PAIRS = TRANSFORM_PAIRS.filter(
  (p) => p.source === "responses" && p.target === "anthropic"
);

for (const pair of RESPONSES_TO_ANTHROPIC_PAIRS) {
  describe(`AI SDK validation: ${pair.target} response → ${pair.source} format`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(responsePath) || isErrorCapture(responsePath))(
        caseName,
        async () => {
          const anthropicResponse = loadAndValidateResponse(
            responsePath,
            pair.target
          );
          const responsesOutput = transformResponseData(
            anthropicResponse,
            pair.wasmSource
          );

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
            fetch: mockJsonFetch(responsesOutput),
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
