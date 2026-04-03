import { describe, test, expect } from "vitest";
import OpenAI from "openai";
import {
  TRANSFORM_PAIRS,
  STREAMING_PAIRS,
  getTransformableCases,
  getStreamingTransformableCases,
  getResponsePath,
  getStreamingResponsePath,
  getFixtureSkipReason,
  registerSkippedFixtureTest,
  useTransformTestServer,
} from "./helpers";

const TIMEOUT = 30000;
const getServer = useTransformTestServer();

for (const pair of TRANSFORM_PAIRS.filter(
  (p) => p.source === "chat-completions"
)) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`chat completions SDK: ${pairLabel}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const path = getResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path);

      describe(caseName, () => {
        if (skipReason) {
          registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        } else {
          test(
            "parses transformed JSON response",
            async () => {
              getServer().useJsonFixture({
                path: "/v1/chat/completions",
                targetFormat: pair.target,
                wasmSource: pair.wasmSource,
                responsePath: path,
              });

              const client = new OpenAI({
                apiKey: "test-key",
                baseURL: getServer().openaiBaseUrl,
              });
              await expect(
                // This request is only a valid SDK entrypoint invocation. The
                // transformed response fixture selected by pair + caseName is what
                // this test is actually asserting.
                client.chat.completions.create({
                  model: "test",
                  messages: [{ role: "user", content: "test" }],
                })
              ).resolves.toBeDefined();
            },
            TIMEOUT
          );
        }
      });
    }
  });
}

for (const pair of STREAMING_PAIRS.filter(
  (p) => p.source === "chat-completions"
)) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`chat completions SDK streaming: ${pairLabel}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const path = getStreamingResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path, { streaming: true });

      describe(caseName, () => {
        if (skipReason) {
          registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        } else {
          test(
            "parses transformed streaming response",
            async () => {
              getServer().useStreamingFixture({
                path: "/v1/chat/completions",
                targetFormat: pair.target,
                wasmSource: pair.wasmSource,
                responsePath: path,
              });

              const client = new OpenAI({
                apiKey: "test-key",
                baseURL: getServer().openaiBaseUrl,
              });
              const stream = await client.chat.completions.create({
                // This request is only a valid SDK entrypoint invocation. The
                // transformed streaming fixture selected by pair + caseName is what
                // this test is actually asserting.
                model: "test",
                messages: [{ role: "user", content: "test" }],
                stream: true,
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
  });
}
