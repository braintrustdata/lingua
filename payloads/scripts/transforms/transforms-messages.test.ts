import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import Anthropic from "@anthropic-ai/sdk";
import {
  TRANSFORM_PAIRS,
  STREAMING_PAIRS,
  loadAndValidateResponse,
  transformResponseData,
  getTransformableCases,
  getStreamingTransformableCases,
  getResponsePath,
  getStreamingResponsePath,
  isErrorCapture,
  flattenStreamChunks,
  buildAnthropicSse,
  mockJsonFetch,
  mockSseFetch,
} from "./helpers";

const TIMEOUT = 30000;

for (const pair of TRANSFORM_PAIRS.filter((p) => p.source === "anthropic")) {
  describe(`messages SDK: ${pair.target} → ${pair.source}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const path = getResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(path) || isErrorCapture(path))(
        caseName,
        async () => {
          const response = loadAndValidateResponse(path, pair.target);
          const output = transformResponseData(response, pair.wasmSource);

          const client = new Anthropic({
            apiKey: "test-key",
            fetch: mockJsonFetch(output),
          });
          await expect(
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
  describe(`messages SDK streaming: ${pair.target} → ${pair.source}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const path = getStreamingResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(path))(
        caseName,
        async () => {
          const rawChunks = JSON.parse(readFileSync(path, "utf-8"));
          const events = flattenStreamChunks(rawChunks, pair.wasmSource);

          const client = new Anthropic({
            apiKey: "test-key",
            fetch: mockSseFetch(buildAnthropicSse(events)),
          });
          const stream = client.messages.stream({
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
