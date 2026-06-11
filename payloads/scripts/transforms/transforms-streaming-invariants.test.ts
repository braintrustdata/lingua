import { describe, test, expect } from "vitest";
import { readFileSync } from "fs";
import {
  STREAMING_PAIRS,
  flattenStreamChunks,
  getFixtureSkipReason,
  getStreamingTransformableCases,
  getStreamingResponsePath,
} from "./helpers";
import { registerSkippedFixtureTest } from "./vitest-helpers";

interface StreamChunk {
  eventType?: string;
  data: unknown;
}

interface AnthropicEventData {
  index?: number;
}

function eventData(chunk: StreamChunk): AnthropicEventData {
  if (typeof chunk.data !== "object" || chunk.data === null) {
    return {};
  }

  return chunk.data;
}

function assertAnthropicStreamInvariants(chunks: StreamChunk[]): void {
  let messageStarted = false;
  let openBlockIndex: number | undefined;
  let sawAnthropicEvent = false;

  for (const [i, chunk] of chunks.entries()) {
    const eventType = chunk.eventType;
    const data = eventData(chunk);

    if (eventType === "message_start") {
      sawAnthropicEvent = true;
      expect(messageStarted, `duplicate message_start at chunk ${i}`).toBe(
        false
      );
      messageStarted = true;
      expect(
        openBlockIndex,
        `message_start while block is open at chunk ${i}`
      ).toBeUndefined();
      continue;
    }

    if (eventType === "content_block_start") {
      sawAnthropicEvent = true;
      expect(
        messageStarted,
        `content_block_start before message_start at chunk ${i}`
      ).toBe(true);
      expect(
        openBlockIndex,
        `content_block_start while block ${openBlockIndex} is open at chunk ${i}`
      ).toBeUndefined();
      expect(
        data.index,
        `content_block_start missing index at chunk ${i}`
      ).toEqual(expect.any(Number));
      openBlockIndex = data.index;
      continue;
    }

    if (eventType === "content_block_delta") {
      sawAnthropicEvent = true;
      expect(
        openBlockIndex,
        `content_block_delta without open block at chunk ${i}`
      ).toEqual(expect.any(Number));
      expect(
        data.index,
        `content_block_delta index mismatch at chunk ${i}`
      ).toBe(openBlockIndex);
      continue;
    }

    if (eventType === "content_block_stop") {
      sawAnthropicEvent = true;
      expect(
        openBlockIndex,
        `content_block_stop without open block at chunk ${i}`
      ).toEqual(expect.any(Number));
      expect(
        data.index,
        `content_block_stop index mismatch at chunk ${i}`
      ).toBe(openBlockIndex);
      openBlockIndex = undefined;
      continue;
    }

    if (eventType === "message_delta") {
      sawAnthropicEvent = true;
      expect(
        messageStarted,
        `message_delta before message_start at chunk ${i}`
      ).toBe(true);
      expect(
        openBlockIndex,
        `message_delta while block is open at chunk ${i}`
      ).toBeUndefined();
      continue;
    }

    if (eventType === "message_stop") {
      sawAnthropicEvent = true;
      expect(
        messageStarted,
        `message_stop before message_start at chunk ${i}`
      ).toBe(true);
      expect(
        openBlockIndex,
        `message_stop while block is open at chunk ${i}`
      ).toBeUndefined();
      messageStarted = false;
      continue;
    }
  }

  expect(sawAnthropicEvent, "expected Anthropic stream events").toBe(true);
  expect(
    openBlockIndex,
    "stream ended while content block is open"
  ).toBeUndefined();
}

for (const pair of STREAMING_PAIRS) {
  if (pair.wasmSource !== "Anthropic") {
    continue;
  }

  describe(`streaming invariants: ${pair.source} -> ${pair.target}`, () => {
    const cases = getStreamingTransformableCases(pair);

    for (const caseName of cases) {
      const streamingPath = getStreamingResponsePath(
        pair.source,
        pair.target,
        caseName
      );
      const pairLabel = `${pair.source} -> ${pair.target}`;
      const skipReason = getFixtureSkipReason(streamingPath, {
        streaming: true,
      });

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(caseName, () => {
        const rawChunks = JSON.parse(readFileSync(streamingPath, "utf-8"));
        const chunks = flattenStreamChunks(rawChunks, pair.wasmSource);
        assertAnthropicStreamInvariants(chunks);
      });
    }
  });
}
