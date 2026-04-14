#!/usr/bin/env tsx

import { existsSync, mkdirSync, writeFileSync } from "fs";
import { dirname } from "path";
import Anthropic from "@anthropic-ai/sdk";
import OpenAI from "openai";
import { allTestCases, getCaseForProvider, GOOGLE_MODEL } from "../../cases";
import {
  TRANSFORM_PAIRS,
  STREAMING_PAIRS,
  TRANSFORMS_DIR,
  parseGoogleSseStream,
  RESPONSE_VALIDATORS,
  getTargetModelForCase,
  transformAndValidateRequest,
  getTransformableCases,
  getStreamingTransformableCases,
  getResponsePath,
  getStreamingResponsePath,
  type SourceFormat,
} from "./helpers";

const GOOGLE_API_BASE = "https://generativelanguage.googleapis.com/v1beta";
const CONCURRENCY = 5;

async function runConcurrently(tasks: (() => Promise<void>)[]): Promise<void> {
  let i = 0;
  async function worker() {
    while (i < tasks.length) {
      const idx = i++;
      await tasks[idx]();
    }
  }
  await Promise.all(Array.from({ length: CONCURRENCY }, worker));
}

let _anthropic: Anthropic | undefined;
let _openai: OpenAI | undefined;

type CallProviderOptions = {
  stream?: boolean;
};

function getAnthropic(): Anthropic {
  if (!_anthropic) _anthropic = new Anthropic();
  return _anthropic;
}

function getOpenAI(): OpenAI {
  if (!_openai) _openai = new OpenAI();
  return _openai;
}

async function callGoogleProvider(
  request: Record<string, unknown>,
  options?: CallProviderOptions
): Promise<unknown> {
  const apiKey = process.env.GOOGLE_API_KEY;
  if (!apiKey) {
    throw new Error("GOOGLE_API_KEY environment variable is required");
  }

  const rawModel = request.model ?? GOOGLE_MODEL;
  const model = typeof rawModel === "string" ? rawModel : String(rawModel);
  const { model: _model, ...body } = request;
  const stream = options?.stream === true;

  const endpoint = stream
    ? `${GOOGLE_API_BASE}/models/${model}:streamGenerateContent?alt=sse`
    : `${GOOGLE_API_BASE}/models/${model}:generateContent`;
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-goog-api-key": apiKey,
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Google API error (${response.status}): ${text}`);
  }

  return stream ? parseGoogleSseStream(response) : response.json();
}

/* eslint-disable @typescript-eslint/consistent-type-assertions -- SDK methods require specific param types, validation done by transformAndValidateRequest */
async function callProvider(
  format: SourceFormat,
  request: Record<string, unknown>,
  options?: CallProviderOptions
): Promise<unknown> {
  const stream = options?.stream === true;
  switch (format) {
    case "anthropic":
      if (stream) {
        return getAnthropic().messages.create(
          {
            ...(request as unknown as Anthropic.MessageCreateParams),
            stream: true,
          },
          { headers: { "anthropic-beta": "structured-outputs-2025-11-13" } }
        );
      }
      return getAnthropic().messages.create(
        request as unknown as Anthropic.MessageCreateParams,
        { headers: { "anthropic-beta": "structured-outputs-2025-11-13" } }
      );
    case "chat-completions":
      if (stream) {
        return getOpenAI().chat.completions.create({
          ...(request as unknown as OpenAI.ChatCompletionCreateParams),
          stream: true,
        } as OpenAI.ChatCompletionCreateParams);
      }
      return getOpenAI().chat.completions.create(
        request as unknown as OpenAI.ChatCompletionCreateParams
      );
    case "responses":
      if (stream) {
        return getOpenAI().responses.create({
          ...(request as unknown as OpenAI.Responses.ResponseCreateParams),
          stream: true,
        });
      }
      return getOpenAI().responses.create(
        request as unknown as OpenAI.Responses.ResponseCreateParams
      );
    case "google":
      return callGoogleProvider(request, { stream });
  }
}
/* eslint-enable @typescript-eslint/consistent-type-assertions */

function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
  return (
    typeof value === "object" &&
    value !== null &&
    Symbol.asyncIterator in value &&
    typeof value[Symbol.asyncIterator] === "function"
  );
}

async function collectStreamChunks(
  streamResponse: unknown
): Promise<unknown[]> {
  if (Array.isArray(streamResponse)) {
    return streamResponse;
  }

  if (!isAsyncIterable(streamResponse)) {
    throw new Error(
      "Expected streaming provider response to be async iterable"
    );
  }

  const chunks: unknown[] = [];
  for await (const chunk of streamResponse) {
    chunks.push(chunk);
  }
  return chunks;
}

export async function captureTransforms(
  filter?: string,
  force?: boolean,
  requestedPair?: { source: string; target: string }
): Promise<{ captured: number; skipped: number; failed: number }> {
  mkdirSync(TRANSFORMS_DIR, { recursive: true });

  let captured = 0,
    skipped = 0,
    failed = 0;

  const nonStreamingTasks: (() => Promise<void>)[] = [];

  for (const p of TRANSFORM_PAIRS) {
    if (
      requestedPair &&
      (p.source !== requestedPair.source || p.target !== requestedPair.target)
    ) {
      continue;
    }
    const cases = getTransformableCases(p, filter);

    for (const caseName of cases) {
      const responsePath = getResponsePath(p.source, p.target, caseName);
      mkdirSync(dirname(responsePath), { recursive: true });

      if (existsSync(responsePath) && !force) {
        skipped++;
        continue;
      }

      const input = getCaseForProvider(allTestCases, caseName, p.source);

      nonStreamingTasks.push(async () => {
        let request: Record<string, unknown> | undefined;
        try {
          const targetModel = getTargetModelForCase(p.target, caseName);
          // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
          request = transformAndValidateRequest(
            input,
            p.wasmTarget,
            p.target,
            targetModel
          ) as Record<string, unknown>;

          const response = await callProvider(p.target, request);

          const responseJson = JSON.stringify(response, null, 2);
          RESPONSE_VALIDATORS[p.target](responseJson);

          writeFileSync(responsePath, responseJson);
          console.log(`✅ ${p.source} → ${p.target} / ${caseName}`);
          captured++;
        } catch (e) {
          const errorObj = e && typeof e === "object" ? e : {};
          const errorData = {
            ...(request ? { request } : {}),
            error: e instanceof Error ? e.message : String(e),
            name: e instanceof Error ? e.name : undefined,
            ...("response" in errorObj ? { response: errorObj.response } : {}),
          };
          writeFileSync(responsePath, JSON.stringify(errorData, null, 2));
          console.error(`❌ ${p.source} → ${p.target} / ${caseName}: ${e}`);
          failed++;
        }
      });
    }
  }

  await runConcurrently(nonStreamingTasks);

  // Capture streaming responses (chat-completions → anthropic, simple cases only)
  const streamingTasks: (() => Promise<void>)[] = [];

  for (const streamingPair of STREAMING_PAIRS) {
    if (
      requestedPair &&
      (streamingPair.source !== requestedPair.source ||
        streamingPair.target !== requestedPair.target)
    ) {
      continue;
    }

    const streamingCases = getStreamingTransformableCases(
      streamingPair,
      filter
    );

    for (const caseName of streamingCases) {
      const streamingPath = getStreamingResponsePath(
        streamingPair.source,
        streamingPair.target,
        caseName
      );
      mkdirSync(dirname(streamingPath), { recursive: true });

      if (existsSync(streamingPath) && !force) {
        skipped++;
        continue;
      }

      const input = getCaseForProvider(
        allTestCases,
        caseName,
        streamingPair.source
      );

      streamingTasks.push(async () => {
        try {
          const streamInput =
            input && typeof input === "object" && !Array.isArray(input)
              ? { ...input, stream: true }
              : input;
          const targetModel = getTargetModelForCase(
            streamingPair.target,
            caseName
          );
          // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
          const streamRequest = transformAndValidateRequest(
            streamInput,
            streamingPair.wasmTarget,
            streamingPair.target,
            targetModel
          ) as Record<string, unknown>;

          const streamResponse = await callProvider(
            streamingPair.target,
            streamRequest,
            { stream: true }
          );
          const chunks = await collectStreamChunks(streamResponse);

          writeFileSync(streamingPath, JSON.stringify(chunks, null, 2));
          console.log(
            `✅ ${streamingPair.source} → ${streamingPair.target} / ${caseName} (streaming)`
          );
          captured++;
        } catch (e) {
          const errorObj = e && typeof e === "object" ? e : {};
          const errorData = {
            error: e instanceof Error ? e.message : String(e),
            name: e instanceof Error ? e.name : undefined,
            ...("response" in errorObj ? { response: errorObj.response } : {}),
          };
          writeFileSync(streamingPath, JSON.stringify(errorData, null, 2));
          console.error(
            `❌ ${streamingPair.source} → ${streamingPair.target} / ${caseName} (streaming): ${e}`
          );
          failed++;
        }
      });
    }
  }

  await runConcurrently(streamingTasks);

  if (skipped > 0 && captured === 0 && failed === 0) {
    console.log(
      `Skipping ${skipped} already captured transforms (use --force to re-capture)`
    );
  } else {
    console.log(
      `Transforms: ${captured} captured, ${skipped} skipped, ${failed} failed`
    );
  }
  return { captured, skipped, failed };
}

async function main() {
  const args = process.argv.slice(2);
  const force = args.includes("--force");
  const pairIdx = args.indexOf("--pair");
  const pairArg = pairIdx !== -1 ? args[pairIdx + 1] : undefined;
  const pair = pairArg
    ? { source: pairArg.split(",")[0], target: pairArg.split(",")[1] }
    : undefined;
  const filter = args.find((a, i) => !a.startsWith("--") && i !== pairIdx + 1);

  const { failed } = await captureTransforms(filter, force, pair);
  process.exit(failed > 0 ? 1 : 0);
}

if (require.main === module) {
  main();
}
