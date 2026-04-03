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

// These tests exercise the source=chat-completions path: captured provider
// responses are transformed back into OpenAI chat completions format, then
// parsed by the OpenAI SDK to verify the transformed payload still satisfies
// its schema.
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
              // 1. Serve the transformed fixture from the chat completions
              //    endpoint so the SDK receives Lingua's wasm output.
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
                // 2. This request only provides a valid SDK entrypoint
                //    invocation. The transformed fixture selected by pair +
                //    caseName is the actual subject under test.
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
              // 1. Serve the transformed streaming fixture from the chat
              //    completions endpoint so the SDK parses Lingua's stream
              //    output.
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
              // 2. This request only opens a valid SDK streaming entrypoint.
              //    The transformed stream fixture selected by pair + caseName
              //    is the actual subject under test.
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
