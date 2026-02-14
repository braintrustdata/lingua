#!/usr/bin/env tsx

import { existsSync, mkdirSync, writeFileSync } from "fs";
import { dirname } from "path";
import Anthropic from "@anthropic-ai/sdk";
import OpenAI from "openai";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  TRANSFORMS_DIR,
  RESPONSE_VALIDATORS,
  TARGET_MODELS,
  transformAndValidateRequest,
  getTransformableCases,
  getResponsePath,
  type SourceFormat,
} from "./helpers";

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
  }
}
/* eslint-enable @typescript-eslint/consistent-type-assertions */

export async function captureTransforms(
  filter?: string,
  force?: boolean,
  providers?: string[]
): Promise<{ captured: number; skipped: number; failed: number }> {
  mkdirSync(TRANSFORMS_DIR, { recursive: true });

  let captured = 0,
    skipped = 0,
    failed = 0;

  for (const pair of TRANSFORM_PAIRS) {
    // Skip pairs that don't involve any of the specified providers
    if (
      providers &&
      !providers.includes(pair.source) &&
      !providers.includes(pair.target)
    ) {
      continue;
    }
    const cases = getTransformableCases(pair, filter);

    for (const caseName of cases) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);
      mkdirSync(dirname(responsePath), { recursive: true });

      if (existsSync(responsePath) && !force) {
        skipped++;
        continue;
      }

      const input = getCaseForProvider(allTestCases, caseName, pair.source);

      try {
        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
        const request = transformAndValidateRequest(
          input,
          pair.wasmTarget,
          pair.target
        ) as Record<string, unknown>;

        const targetCase = getCaseForProvider(
          allTestCases,
          caseName,
          pair.target
        );
        request.model =
          targetCase && typeof targetCase === "object" && "model" in targetCase
            ? targetCase.model
            : TARGET_MODELS[pair.target];

        const response = await callProvider(pair.target, request);

        const responseJson = JSON.stringify(response, null, 2);
        RESPONSE_VALIDATORS[pair.target](responseJson);

        writeFileSync(responsePath, responseJson);
        console.log(`✅ ${pair.source} → ${pair.target} / ${caseName}`);
        captured++;
      } catch (e) {
        const errorObj = e && typeof e === "object" ? e : {};
        const errorData = {
          error: e instanceof Error ? e.message : String(e),
          name: e instanceof Error ? e.name : undefined,
          ...("response" in errorObj ? { response: errorObj.response } : {}),
        };
        writeFileSync(responsePath, JSON.stringify(errorData, null, 2));
        console.error(`❌ ${pair.source} → ${pair.target} / ${caseName}: ${e}`);
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
  const filter = args.find((a) => !a.startsWith("--"));

  const { failed } = await captureTransforms(filter, force);
  process.exit(failed > 0 ? 1 : 0);
}

if (require.main === module) {
  main();
}
