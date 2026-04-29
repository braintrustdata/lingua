import { readFileSync } from "fs";
import { join } from "path";
import { describe, expect, test } from "vitest";
import OpenAI from "openai";
import {
  buildSse,
  findJsonString,
  hasJsonString,
  parseSseEvents,
} from "./helpers";
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

  test("full text message response transforms to anthropic stream events", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(vertexAnthropicStreamingPath, "utf-8")
    );
    const sse = buildSse(rawChunks, "Anthropic");
    const events = parseSseEvents(sse);
    const eventTypes = events.map((event) => event.event);

    expect(eventTypes).toContain("message_start");
    expect(eventTypes).toContain("content_block_start");
    expect(eventTypes).toContain("content_block_delta");
    expect(eventTypes).toContain("message_delta");
    expect(eventTypes).toContain("message_stop");
    expect(
      events.some((event) =>
        hasJsonString(event.data, "text", "The capital of France is Paris.")
      )
    ).toBe(true);
    expect(
      events.some((event) =>
        hasJsonString(event.data, "stop_reason", "end_turn")
      )
    ).toBe(true);
  });

  test("full tool message response transforms to anthropic stream events", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(
        vertexAnthropicStreamingPathForCase("toolCallRequest"),
        "utf-8"
      )
    );
    const sse = buildSse(rawChunks, "Anthropic");
    const events = parseSseEvents(sse);
    const eventTypes = events.map((event) => event.event);

    expect(eventTypes).toContain("message_start");
    expect(eventTypes).toContain("content_block_start");
    expect(eventTypes).toContain("message_delta");
    expect(eventTypes).toContain("message_stop");
    expect(
      events.some((event) => hasJsonString(event.data, "type", "tool_use"))
    ).toBe(true);
    expect(
      events.some((event) => hasJsonString(event.data, "name", "get_weather"))
    ).toBe(true);
    expect(
      events.some((event) =>
        hasJsonString(event.data, "stop_reason", "tool_use")
      )
    ).toBe(true);
  });

  test("full thinking response preserves signature in anthropic stream", () => {
    const rawChunks: unknown[] = JSON.parse(
      readFileSync(
        vertexAnthropicStreamingPathForCase("thinkingSignatureRequest"),
        "utf-8"
      )
    );
    const capturedThinking = findJsonString(rawChunks, "thinking");
    const capturedSignature = findJsonString(rawChunks, "signature");
    const sse = buildSse(rawChunks, "Anthropic");
    const events = parseSseEvents(sse);

    expect(capturedThinking).toBeDefined();
    expect(capturedSignature).toBeDefined();
    expect(
      events.some((event) =>
        hasJsonString(event.data, "thinking", capturedThinking ?? "")
      )
    ).toBe(true);
    expect(
      events.some((event) =>
        hasJsonString(event.data, "signature", capturedSignature ?? "")
      )
    ).toBe(true);
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

  test(
    "full tool response parses through the OpenAI SDK",
    async () => {
      getServer().useStreamingFixture({
        path: "/v1/chat/completions",
        targetFormat: "anthropic",
        wasmSource: "OpenAI",
        responsePath: vertexAnthropicStreamingPathForCase("toolCallRequest"),
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
      expect(
        collected.some((chunk) => hasJsonString(chunk, "name", "get_weather"))
      ).toBe(true);
      expect(
        collected.some((chunk) =>
          hasJsonString(chunk, "finish_reason", "tool_calls")
        )
      ).toBe(true);
    },
    TIMEOUT
  );
});
