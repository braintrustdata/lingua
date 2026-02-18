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
  RESPONSE_VALIDATORS,
  TARGET_MODELS,
  transformAndValidateRequest,
  getTransformableCases,
  getStreamingTransformableCases,
  getResponsePath,
  getStreamingResponsePath,
  type SourceFormat,
} from "./helpers";

const GOOGLE_API_BASE = "https://generativelanguage.googleapis.com/v1beta";

let _anthropic: Anthropic | undefined;
let _openai: OpenAI | undefined;

function getAnthropic(): Anthropic {
  if (!_anthropic) _anthropic = new Anthropic();
  return _anthropic;
}

function getOpenAI(): OpenAI {
  if (!_openai) _openai = new OpenAI();
  return _openai;
}

async function callGoogleProvider(
  request: Record<string, unknown>
): Promise<unknown> {
  const apiKey = process.env.GOOGLE_API_KEY;
  if (!apiKey) {
    throw new Error("GOOGLE_API_KEY environment variable is required");
  }

  const rawModel = request.model ?? GOOGLE_MODEL;
  const model = typeof rawModel === "string" ? rawModel : String(rawModel);
  const { model: _model, ...body } = request;

  const endpoint = `${GOOGLE_API_BASE}/models/${model}:generateContent`;
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

  return response.json();
}

/* eslint-disable @typescript-eslint/consistent-type-assertions -- SDK methods require specific param types, validation done by transformAndValidateRequest */
async function callProvider(
  format: SourceFormat,
  request: Record<string, unknown>
): Promise<unknown> {
  switch (format) {
    case "anthropic":
      return getAnthropic().messages.create(
        request as unknown as Anthropic.MessageCreateParams,
        { headers: { "anthropic-beta": "structured-outputs-2025-11-13" } }
      );
    case "chat-completions":
      return getOpenAI().chat.completions.create(
        request as unknown as OpenAI.ChatCompletionCreateParams
      );
    case "responses":
      return getOpenAI().responses.create(
        request as unknown as OpenAI.Responses.ResponseCreateParams
      );
    case "google":
      return callGoogleProvider(request);
  }
}
/* eslint-enable @typescript-eslint/consistent-type-assertions */

export async function captureTransforms(
  filter?: string,
  force?: boolean,
  pair?: { source: string; target: string }
): Promise<{ captured: number; skipped: number; failed: number }> {
  mkdirSync(TRANSFORMS_DIR, { recursive: true });

  let captured = 0,
    skipped = 0,
    failed = 0;

  for (const p of TRANSFORM_PAIRS) {
    if (pair && (p.source !== pair.source || p.target !== pair.target)) {
      continue;
    }
    const cases = getTransformableCases(p, filter);

    for (const caseName of cases) {
      const responsePath = getResponsePath(p.source, p.target, caseName);
      mkdirSync(dirname(responsePath), { recursive: true });

      const input = getCaseForProvider(allTestCases, caseName, p.source);

      // Capture non-streaming response
      if (existsSync(responsePath) && !force) {
        skipped++;
      } else {
        try {
          // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
          const request = transformAndValidateRequest(
            input,
            p.wasmTarget,
            p.target
          ) as Record<string, unknown>;

          const targetCase = getCaseForProvider(
            allTestCases,
            caseName,
            p.target
          );
          request.model =
            targetCase &&
            typeof targetCase === "object" &&
            "model" in targetCase
              ? targetCase.model
              : TARGET_MODELS[p.target];

          const response = await callProvider(p.target, request);

          const responseJson = JSON.stringify(response, null, 2);
          RESPONSE_VALIDATORS[p.target](responseJson);

          writeFileSync(responsePath, responseJson);
          console.log(`✅ ${p.source} → ${p.target} / ${caseName}`);
          captured++;
        } catch (e) {
          const errorObj = e && typeof e === "object" ? e : {};
          const errorData = {
            error: e instanceof Error ? e.message : String(e),
            name: e instanceof Error ? e.name : undefined,
            ...("response" in errorObj ? { response: errorObj.response } : {}),
          };
          writeFileSync(responsePath, JSON.stringify(errorData, null, 2));
          console.error(`❌ ${p.source} → ${p.target} / ${caseName}: ${e}`);
          failed++;
        }
      }
    }
  }

  // Capture streaming responses (chat-completions → anthropic, simple cases only)
  for (const pair of STREAMING_PAIRS) {
    const streamingCases = getStreamingTransformableCases(pair, filter);

    for (const caseName of streamingCases) {
      const streamingPath = getStreamingResponsePath(
        pair.source,
        pair.target,
        caseName
      );
      mkdirSync(dirname(streamingPath), { recursive: true });

      if (existsSync(streamingPath) && !force) {
        skipped++;
        continue;
      }

      const input = getCaseForProvider(allTestCases, caseName, pair.source);

      try {
        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
        const streamRequest = transformAndValidateRequest(
          input,
          pair.wasmTarget,
          pair.target
        ) as Record<string, unknown>;

        const targetCase = getCaseForProvider(
          allTestCases,
          caseName,
          pair.target
        );
        streamRequest.model =
          targetCase && typeof targetCase === "object" && "model" in targetCase
            ? targetCase.model
            : TARGET_MODELS[pair.target];

        /* eslint-disable @typescript-eslint/consistent-type-assertions -- SDK requires specific param type */
        const streamResponse = await getAnthropic().messages.create(
          {
            ...(streamRequest as unknown as Anthropic.MessageCreateParams),
            stream: true,
          },
          {
            headers: { "anthropic-beta": "structured-outputs-2025-11-13" },
          }
        );
        /* eslint-enable @typescript-eslint/consistent-type-assertions */

        const chunks: unknown[] = [];
        for await (const chunk of streamResponse) {
          chunks.push(chunk);
        }

        writeFileSync(streamingPath, JSON.stringify(chunks, null, 2));
        console.log(
          `✅ ${pair.source} → ${pair.target} / ${caseName} (streaming)`
        );
        captured++;
      } catch (e) {
        console.error(
          `❌ ${pair.source} → ${pair.target} / ${caseName} (streaming): ${e}`
        );
        failed++;
      }
    }
  }

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
