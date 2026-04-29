import { readFileSync } from "fs";
import { join } from "path";
import { describe, expect, test } from "vitest";
import OpenAI from "openai";
import { buildSse } from "./helpers";
import { useTransformTestServer } from "./vitest-helpers";

const TIMEOUT = 30000;
const getServer = useTransformTestServer();
const vertexAnthropicStreamingPath = join(
  __dirname,
  "../../snapshots/simpleRequest/vertex-anthropic/response-streaming.json"
);

function vertexAnthropicStreamingPathForCase(caseName: string): string {
  return join(
    __dirname,
    `../../snapshots/${caseName}/vertex-anthropic/response-streaming.json`
  );
}

describe("vertex anthropic streaming snapshots", () => {
  test(
    "full message chunks transform to chat completions stream",
    async () => {
      const rawChunks: unknown[] = JSON.parse(
        readFileSync(vertexAnthropicStreamingPath, "utf-8")
      );
      const sse = buildSse(rawChunks, "OpenAI");

      expect(sse).toContain("data:");
      expect(sse).toContain("The capital of France is Paris.");

      getServer().useStreamingFixture({
        path: "/v1/chat/completions",
        targetFormat: "anthropic",
        wasmSource: "OpenAI",
        responsePath: vertexAnthropicStreamingPath,
      });

      const client = new OpenAI({
        apiKey: "test-key",
        baseURL: getServer().openaiBaseUrl,
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
      expect(JSON.stringify(collected)).toContain(
        "The capital of France is Paris."
      );
    },
    TIMEOUT
  );

  test("max_tokens stop reason transforms to length finish reason", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(
        vertexAnthropicStreamingPathForCase("reasoningRequestTruncated"),
        "utf-8"
      )
    );
    const sse = buildSse(rawChunks, "OpenAI");

    expect(sse).toContain('"finish_reason":"length"');
  });

  test("reasoning request text response transforms to chat completions content", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(
        vertexAnthropicStreamingPathForCase("reasoningRequest"),
        "utf-8"
      )
    );
    const sse = buildSse(rawChunks, "OpenAI");

    expect(sse).toContain('"finish_reason":"stop"');
    expect(sse).toContain("Average Speed Problem");
  });

  test("tool_use content transforms to chat completions tool calls", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(
        vertexAnthropicStreamingPathForCase("toolCallRequest"),
        "utf-8"
      )
    );
    const sse = buildSse(rawChunks, "OpenAI");

    expect(sse).toContain('"finish_reason":"tool_calls"');
    expect(sse).toContain('"tool_calls":[');
    expect(sse).toContain('"name":"get_weather"');
  });
});
