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
  transformAndValidateRequest,
  getTransformableCases,
  getResponsePath,
  type SourceFormat,
} from "./helpers";

// Models to substitute (source models may not exist on target provider)
const TARGET_MODELS: Record<SourceFormat, string> = {
  anthropic: "claude-sonnet-4-20250514",
  "chat-completions": "gpt-5-nano",
  responses: "gpt-5-nano",
};

// Direct SDK clients
const anthropic = new Anthropic();
const openai = new OpenAI();

/* eslint-disable @typescript-eslint/consistent-type-assertions -- SDK methods require specific param types, validation done by transformAndValidateRequest */
// Direct SDK calls (request already validated by transformAndValidateRequest)
async function callProvider(
  format: SourceFormat,
  request: Record<string, unknown>
): Promise<unknown> {
  switch (format) {
    case "anthropic":
      return anthropic.messages.create(
        request as unknown as Anthropic.MessageCreateParams,
        { headers: { "anthropic-beta": "structured-outputs-2025-11-13" } }
      );
    case "chat-completions":
      return openai.chat.completions.create(
        request as unknown as OpenAI.ChatCompletionCreateParams
      );
    case "responses":
      return openai.responses.create(
        request as unknown as OpenAI.Responses.ResponseCreateParams
      );
  }
}
/* eslint-enable @typescript-eslint/consistent-type-assertions */

async function main() {
  const args = process.argv.slice(2);
  const force = args.includes("--force");
  const filter = args.find((a) => !a.startsWith("--"));

  mkdirSync(TRANSFORMS_DIR, { recursive: true });

  let captured = 0,
    skipped = 0,
    failed = 0;

  for (const pair of TRANSFORM_PAIRS) {
    const cases = getTransformableCases(pair, filter);

    for (const caseName of cases) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);
      mkdirSync(dirname(responsePath), { recursive: true });

      if (existsSync(responsePath) && !force) {
        console.log(
          `⏭️  ${pair.source} → ${pair.target} / ${caseName} (exists)`
        );
        skipped++;
        continue;
      }

      const input = getCaseForProvider(allTestCases, caseName, pair.source);

      try {
        // 1. Transform + validate request against target schema
        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- transformAndValidateRequest returns validated object
        const request = transformAndValidateRequest(
          input,
          pair.wasmTarget,
          pair.target
        ) as Record<string, unknown>;

        // 2. Override model for target provider
        request.model = TARGET_MODELS[pair.target];

        // 3. Call SDK directly
        const response = await callProvider(pair.target, request);

        // 4. Validate response against target schema before saving
        const responseJson = JSON.stringify(response, null, 2);
        RESPONSE_VALIDATORS[pair.target](responseJson);

        // 5. Save validated response
        writeFileSync(responsePath, responseJson);
        console.log(`✅ ${pair.source} → ${pair.target} / ${caseName}`);
        captured++;
      } catch (e) {
        // Save error response
        const errorObj = e && typeof e === "object" ? e : {};
        const errorData = {
          error: e instanceof Error ? e.message : String(e),
          name: e instanceof Error ? e.name : undefined,
          // Include response body if available (API errors often have useful details)
          ...("response" in errorObj ? { response: errorObj.response } : {}),
        };
        writeFileSync(responsePath, JSON.stringify(errorData, null, 2));
        console.error(`❌ ${pair.source} → ${pair.target} / ${caseName}: ${e}`);
        failed++;
      }
    }
  }

  console.log(
    `\nDone: ${captured} captured, ${skipped} skipped, ${failed} failed`
  );
  process.exit(failed > 0 ? 1 : 0);
}

main();
