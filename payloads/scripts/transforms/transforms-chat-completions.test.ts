import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import OpenAI from "openai";
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
  buildOpenAISse,
  mockJsonFetch,
  mockSseFetch,
} from "./helpers";

const TIMEOUT = 30000;

for (const pair of TRANSFORM_PAIRS.filter(
  (p) => p.source === "chat-completions"
)) {
  describe(`chat completions SDK: ${pair.target} → ${pair.source}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const path = getResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(path) || isErrorCapture(path))(
        caseName,
        async () => {
          const response = loadAndValidateResponse(path, pair.target);
          const output = transformResponseData(response, pair.wasmSource);

          const client = new OpenAI({
            apiKey: "test-key",
            fetch: mockJsonFetch(output),
          });
          await expect(
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

for (const pair of STREAMING_PAIRS.filter(
  (p) => p.source === "chat-completions"
)) {
  describe(`chat completions SDK streaming: ${pair.target} → ${pair.source}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const path = getStreamingResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(path))(
        caseName,
        async () => {
          const rawChunks = JSON.parse(readFileSync(path, "utf-8"));
          const events = flattenStreamChunks(rawChunks, pair.wasmSource);

          const client = new OpenAI({
            apiKey: "test-key",
            fetch: mockSseFetch(buildOpenAISse(events)),
          });
          const stream = await client.chat.completions.create({
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
