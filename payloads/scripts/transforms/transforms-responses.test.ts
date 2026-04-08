import { describe, test, expect } from "vitest";
import OpenAI from "openai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  STREAMING_PAIRS,
  TARGET_MODELS,
  TRANSFORM_PAIRS,
  getFixtureSkipReason,
  getResponsePath,
  getStreamingResponsePath,
  getStreamingTransformableCases,
  getTransformableCases,
} from "./helpers";
import {
  registerSkippedFixtureTest,
  useTransformTestServer,
} from "./vitest-helpers";

const TIMEOUT = 30000;
const getServer = useTransformTestServer();

function getResponsesModel(caseName: string): string {
  const responsesCase = getCaseForProvider(allTestCases, caseName, "responses");
  return responsesCase &&
    typeof responsesCase === "object" &&
    "model" in responsesCase &&
    typeof responsesCase.model === "string"
    ? responsesCase.model
    : TARGET_MODELS.responses;
}

// These tests exercise the source=responses path: captured provider responses
// are transformed back into OpenAI Responses format, then parsed by the OpenAI
// SDK to verify the transformed payload still satisfies its schema.
for (const pair of TRANSFORM_PAIRS.filter((p) => p.source === "responses")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`responses SDK: ${pairLabel}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const path = getResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path);

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          const model = getResponsesModel(caseName);

          // 1. Serve the transformed fixture from the responses endpoint so
          //    the SDK receives Lingua's wasm output.
          getServer().useJsonFixture({
            path: "/v1/responses",
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const client = new OpenAI({
            apiKey: "test-key",
            baseURL: getServer().openaiBaseUrl,
          });
          await expect(
            // 2. This request only provides a valid SDK entrypoint invocation.
            //    The transformed fixture selected by pair + caseName is the
            //    actual subject under test.
            client.responses.create({
              model,
              input: "test",
              stream: false,
            })
          ).resolves.toBeDefined();
        },
        TIMEOUT
      );
    }
  });
}

for (const pair of STREAMING_PAIRS.filter((p) => p.source === "responses")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`responses SDK streaming: ${pairLabel}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const path = getStreamingResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path, { streaming: true });

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          const model = getResponsesModel(caseName);

          // 1. Serve the transformed streaming fixture from the responses
          //    endpoint so the SDK parses Lingua's stream output.
          getServer().useStreamingFixture({
            path: "/v1/responses",
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const client = new OpenAI({
            apiKey: "test-key",
            baseURL: getServer().openaiBaseUrl,
          });
          // 2. This request only opens a valid SDK streaming entrypoint. The
          //    transformed stream fixture selected by pair + caseName is the
          //    actual subject under test.
          const stream = await client.responses.create({
            model,
            input: "test",
            stream: true,
          });

          const collected: unknown[] = [];
          for await (const event of stream) {
            collected.push(event);
          }
          expect(collected.length).toBeGreaterThan(0);
        },
        TIMEOUT
      );
    }
  });
}
