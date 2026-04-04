import { describe, test, expect } from "vitest";
import Anthropic from "@anthropic-ai/sdk";
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

for (const pair of TRANSFORM_PAIRS.filter((p) => p.source === "anthropic")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`messages SDK: ${pairLabel}`, () => {
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
          getServer().useJsonFixture({
            path: "/v1/messages",
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const client = new Anthropic({
            apiKey: "test-key",
            baseURL: getServer().anthropicBaseUrl,
          });
          await expect(
            // This request is only a valid SDK entrypoint invocation. The
            // transformed response fixture selected by pair + caseName is what
            // this test is actually asserting.
            client.messages.create({
              model: "test",
              max_tokens: 1024,
              messages: [{ role: "user", content: "test" }],
            })
          ).resolves.toBeDefined();
        },
        TIMEOUT
      );
    }
  });
}

for (const pair of STREAMING_PAIRS.filter((p) => p.source === "anthropic")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`messages SDK streaming: ${pairLabel}`, () => {
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
          getServer().useStreamingFixture({
            path: "/v1/messages",
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const client = new Anthropic({
            apiKey: "test-key",
            baseURL: getServer().anthropicBaseUrl,
          });
          const stream = client.messages.stream({
            // This request is only a valid SDK entrypoint invocation. The
            // transformed streaming fixture selected by pair + caseName is what
            // this test is actually asserting.
            model: "test",
            max_tokens: 1024,
            messages: [{ role: "user", content: "test" }],
          });

          const message = await stream.finalMessage();
          expect(message).toBeDefined();
        },
        TIMEOUT
      );
    }
  });
}
