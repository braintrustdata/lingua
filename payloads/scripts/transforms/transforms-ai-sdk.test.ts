import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
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
} from "./helpers";

const AI_SDK_TEST_TIMEOUT = 30000;

// Test the responses → anthropic path: the captured response is from Anthropic,
// and Lingua transforms it back to responses format. The AI SDK consumes that
// transformed output, so it must pass the AI SDK's Zod validation.
const RESPONSES_TO_ANTHROPIC_PAIRS = TRANSFORM_PAIRS.filter(
  (p) => p.source === "responses" && p.target === "anthropic"
);

for (const pair of RESPONSES_TO_ANTHROPIC_PAIRS) {
  describe(`AI SDK validation: ${pair.target} response → ${pair.source} format`, () => {
    const cases = getTransformableCases(pair);

    for (const caseName of cases) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      // Skip cases where the captured response is an API error (not a valid provider response)
      const isErrorResponse =
        existsSync(responsePath) &&
        (() => {
          const raw = JSON.parse(readFileSync(responsePath, "utf-8"));
          return "error" in raw && !("id" in raw);
        })();

      test.skipIf(!existsSync(responsePath) || isErrorResponse)(
        caseName,
        async () => {
          // 1. Load the captured Anthropic response
          const anthropicResponse = loadAndValidateResponse(
            responsePath,
            pair.target
          );

          // 2. Transform it to responses format via wasm — this is what the gateway
          //    would return to the AI SDK
          const responsesOutput = transformResponseData(
            anthropicResponse,
            pair.wasmSource
          );

          // 3. Validate through the actual AI SDK by using a mock fetch that returns
          //    the wasm-transformed output. The AI SDK internally parses with its Zod
          //    schema and throws TypeValidationError when fields like `id` or
          //    `annotations` are missing.
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
            /* eslint-disable @typescript-eslint/consistent-type-assertions -- mock fetch for testing */
            fetch: (async () => {
              return new Response(JSON.stringify(responsesOutput), {
                status: 200,
                headers: { "content-type": "application/json" },
              });
            }) as unknown as typeof fetch,
            /* eslint-enable @typescript-eslint/consistent-type-assertions */
          });

          // generateText will parse the response through the AI SDK's Zod schema.
          // If the response is missing required fields (id, annotations, etc.),
          // it throws TypeValidationError — which is exactly what this test catches.
          await expect(
            generateText({
              model: provider.responses(model),
              prompt: "test",
            })
          ).resolves.toBeDefined();
        },
        AI_SDK_TEST_TIMEOUT
      );
    }
  });
}
