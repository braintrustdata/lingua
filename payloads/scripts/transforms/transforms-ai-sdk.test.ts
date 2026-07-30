import { describe, test, expect } from "vitest";
import { createOpenAI } from "@ai-sdk/openai";
import { generateText, streamText } from "ai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  STREAMING_PAIRS,
  TARGET_MODELS,
  getTransformableCases,
  getStreamingTransformableCases,
  getResponsePath,
  getStreamingResponsePath,
  getFixtureSkipReason,
} from "./helpers";
import {
  registerSkippedFixtureTest,
  useTransformTestServer,
} from "./vitest-helpers";

const TIMEOUT = 30000;
const getServer = useTransformTestServer();

// Test the responses → anthropic path: the captured response is from Anthropic,
// and Lingua transforms it back to responses format. The AI SDK consumes that
// transformed output, so it must pass the AI SDK's Zod validation.
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
          // 1. Serve the captured Anthropic response through the transform test
          //    server so the SDK receives Lingua's wasm-transformed output.
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

          // 2. Validate through the actual AI SDK. It parses the transformed
          //    response with its Zod schema and throws if required fields such
          //    as `id` or `annotations` are missing.
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

// Same path, but streamed. A malformed Responses stream does not throw -- the AI
// SDK simply drops content it cannot attach to an output item -- so these tests
// assert the SDK actually surfaces the content rather than just resolving.
const STREAMING_RESPONSES_TO_ANTHROPIC_PAIRS = STREAMING_PAIRS.filter(
  (p) => p.source === "responses" && p.target === "anthropic"
);

for (const pair of STREAMING_RESPONSES_TO_ANTHROPIC_PAIRS) {
  const pairLabel = `${pair.target} stream → ${pair.source} format`;
  describe(`AI SDK validation (streaming): ${pairLabel}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const responsePath = getStreamingResponsePath(
        pair.source,
        pair.target,
        caseName
      );
      const skipReason = getFixtureSkipReason(responsePath, { streaming: true });

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          getServer().useStreamingFixture({
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

          const result = streamText({
            model: provider.responses(model),
            prompt: "test",
          });

          let text = "";
          for await (const delta of result.textStream) {
            text += delta;
          }
          const [finishReason, toolCalls] = await Promise.all([
            result.finishReason,
            result.toolCalls,
          ]);

          expect(finishReason).toBeDefined();
          // Every fixture in this set produces assistant text, tool calls, or
          // both; surfacing neither means the transformed stream was malformed.
          expect(
            text.length > 0 || toolCalls.length > 0,
            `AI SDK surfaced no content: text=${JSON.stringify(text)} toolCalls=${toolCalls.length}`
          ).toBe(true);
        },
        TIMEOUT
      );
    }
  });
}
