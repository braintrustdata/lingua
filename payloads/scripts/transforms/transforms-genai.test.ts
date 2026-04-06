import { describe, test, expect } from "vitest";
import { GoogleGenAI } from "@google/genai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  STREAMING_PAIRS,
  TARGET_MODELS,
  TRANSFORM_PAIRS,
  getFixtureSkipReason,
  getGenAiGenerateContentPath,
  getGenAiStreamGenerateContentPath,
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

function getGoogleModel(caseName: string): string {
  const googleCase = getCaseForProvider(allTestCases, caseName, "google");
  return googleCase &&
    typeof googleCase === "object" &&
    "model" in googleCase &&
    typeof googleCase.model === "string"
    ? googleCase.model
    : TARGET_MODELS.google;
}

function createGenAiClient() {
  return new GoogleGenAI({
    apiKey: "test-key",
    apiVersion: "v1beta",
    httpOptions: {
      baseUrl: getServer().genaiBaseUrl,
    },
  });
}

// These tests exercise the source=google path: captured provider responses are
// transformed back into Google format, then parsed by the Google GenAI SDK to
// verify the transformed payload still satisfies its schema.
for (const pair of TRANSFORM_PAIRS.filter((p) => p.source === "google")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`genai SDK: ${pairLabel}`, () => {
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
          const model = getGoogleModel(caseName);

          // 1. Serve the transformed fixture from the Gemini generateContent
          //    endpoint so the SDK receives Lingua's wasm output.
          getServer().useJsonFixture({
            path: getGenAiGenerateContentPath(model),
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const ai = createGenAiClient();
          await expect(
            // 2. This request only provides a valid SDK entrypoint invocation.
            //    The transformed fixture selected by pair + caseName is the
            //    actual subject under test.
            ai.models.generateContent({
              model,
              contents: "test",
            })
          ).resolves.toBeDefined();
        },
        TIMEOUT
      );
    }
  });
}

for (const pair of STREAMING_PAIRS.filter((p) => p.source === "google")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`genai SDK streaming: ${pairLabel}`, () => {
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
          const model = getGoogleModel(caseName);

          // 1. Serve the transformed streaming fixture from the Gemini
          //    streamGenerateContent endpoint so the SDK parses Lingua's
          //    stream output.
          getServer().useStreamingFixture({
            path: getGenAiStreamGenerateContentPath(model),
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
            requireStream: false,
          });

          const ai = createGenAiClient();
          // 2. This request only opens a valid SDK streaming entrypoint. The
          //    transformed stream fixture selected by pair + caseName is the
          //    actual subject under test.
          const stream = await ai.models.generateContentStream({
            model,
            contents: "test",
          });

          const collected: unknown[] = [];
          for await (const chunk of stream) {
            collected.push(chunk);
          }
          expect(collected.length).toBeGreaterThan(0);
        },
        TIMEOUT
      );
    }
  });
}
