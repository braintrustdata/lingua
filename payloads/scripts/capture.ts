#!/usr/bin/env tsx

import { existsSync, mkdirSync } from "fs";
import { join } from "path";
import { needsRegeneration } from "./cache-utils";

// Import individual capture functions
import { openaiPayloads, captureSinglePayload } from "./capture-openai";
import {
  openaiResponsesPayloads,
  captureSingleResponsesPayload,
} from "./capture-openai-responses";
import {
  anthropicPayloads,
  captureSingleAnthropicPayload,
} from "./capture-anthropic";
import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// Type guards for different payload types
function isOpenAIChatPayload(
  payload: unknown,
): payload is OpenAI.ChatCompletionCreateParams {
  return (
    typeof payload === "object" && payload !== null && "messages" in payload
  );
}

function isOpenAIResponsesPayload(
  payload: unknown,
): payload is Parameters<typeof OpenAI.prototype.responses.create>[0] {
  return typeof payload === "object" && payload !== null && "input" in payload;
}

function isAnthropicPayload(
  payload: unknown,
): payload is Anthropic.MessageCreateParams {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "messages" in payload &&
    "max_tokens" in payload
  );
}

interface CaptureCase {
  name: string;
  provider: "openai-chat" | "openai-responses" | "anthropic";
  payload: unknown; // Different providers have incompatible payload types
}

interface CaptureOptions {
  list: boolean;
  force: boolean;
  filter?: string;
  providers?: string[];
  cases?: string[];
  stream?: boolean; // undefined = both, true = streaming only, false = non-streaming only
}

function getAllCases(): CaptureCase[] {
  const cases: CaptureCase[] = [];

  // Add OpenAI Chat Completions cases
  for (const [name, payload] of Object.entries(openaiPayloads)) {
    cases.push({ name, provider: "openai-chat", payload });
  }

  // Add OpenAI Responses cases
  for (const [name, payload] of Object.entries(openaiResponsesPayloads)) {
    cases.push({ name, provider: "openai-responses", payload });
  }

  // Add Anthropic cases
  for (const [name, payload] of Object.entries(anthropicPayloads)) {
    cases.push({ name, provider: "anthropic", payload });
  }

  return cases;
}

function parseArguments(): CaptureOptions {
  const args = process.argv.slice(2);
  const options: CaptureOptions = {
    list: false,
    force: false,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    switch (arg) {
      case "--list":
        options.list = true;
        break;
      case "--force":
        options.force = true;
        break;
      case "--filter":
        if (i + 1 < args.length) {
          options.filter = args[i + 1];
          i++;
        }
        break;
      case "--providers":
      case "--provider": // Support both singular and plural
        if (i + 1 < args.length) {
          options.providers = args[i + 1].split(",");
          i++;
        }
        break;
      case "--cases":
      case "--case": // Support both singular and plural
        if (i + 1 < args.length) {
          options.cases = args[i + 1].split(",");
          i++;
        }
        break;
      case "--stream":
        if (i + 1 < args.length) {
          const streamValue = args[i + 1].toLowerCase();
          if (streamValue === "true") {
            options.stream = true;
          } else if (streamValue === "false") {
            options.stream = false;
          } else {
            console.error("--stream must be 'true' or 'false'");
            process.exit(1);
          }
          i++;
        } else {
          console.error("--stream requires a value (true or false)");
          process.exit(1);
        }
        break;
      default:
        if (arg.startsWith("--")) {
          console.error(`Unknown option: ${arg}`);
          console.error(
            "Available options: --list, --force, --filter, --providers, --cases, --stream",
          );
          process.exit(1);
        }
    }
  }

  return options;
}

function filterCases(
  allCases: CaptureCase[],
  options: CaptureOptions,
): CaptureCase[] {
  let filtered = allCases;

  // Filter by providers
  if (options.providers && options.providers.length > 0) {
    const validProviders = ["openai-chat", "openai-responses", "anthropic"];
    const invalidProviders = options.providers.filter(
      (p) => !validProviders.includes(p),
    );

    if (invalidProviders.length > 0) {
      console.error(`Invalid provider(s): ${invalidProviders.join(", ")}`);
      console.error(`Valid providers: ${validProviders.join(", ")}`);
      process.exit(1);
    }

    filtered = filtered.filter((c) => options.providers!.includes(c.provider));
  }

  // Filter by case names
  if (options.cases && options.cases.length > 0) {
    filtered = filtered.filter((c) =>
      options.cases!.some((caseName) => c.name.includes(caseName)),
    );
  }

  // Filter by general filter
  if (options.filter) {
    filtered = filtered.filter(
      (c) =>
        c.name.includes(options.filter!) ||
        c.provider.includes(options.filter!),
    );
  }

  return filtered;
}

function getSnapshotPath(caseName: string, provider: string): string {
  const outputDir = join(__dirname, "..", "snapshots");
  mkdirSync(outputDir, { recursive: true });
  return outputDir;
}

function isAlreadyCaptured(
  caseName: string,
  provider: string,
  payload: unknown,
): boolean {
  const outputDir = getSnapshotPath(caseName, provider);
  return !needsRegeneration(outputDir, provider, caseName, payload);
}

function listCases(cases: CaptureCase[]): void {
  console.log("\nAvailable cases:\n");

  const grouped = cases.reduce<Record<string, CaptureCase[]>>((acc, c) => {
    if (!acc[c.provider]) acc[c.provider] = [];
    acc[c.provider].push(c);
    return acc;
  }, {});

  for (const [provider, providerCases] of Object.entries(grouped)) {
    console.log(`📦 ${provider}:`);

    for (const c of providerCases) {
      const captured = isAlreadyCaptured(c.name, c.provider, c.payload);
      const status = captured ? "✓" : "○";
      console.log(`  ${status} ${c.name}`);
    }
    console.log("");
  }

  console.log("Legend: ✓ captured, ○ not captured");
  console.log("\nUsage examples:");
  console.log("  pnpm capture --list");
  console.log("  pnpm capture --providers openai-chat,anthropic");
  console.log("  pnpm capture --cases toolCall,simple");
  console.log("  pnpm capture --filter reasoning");
  console.log("  pnpm capture --stream true  # Streaming only");
  console.log("  pnpm capture --stream false # Non-streaming only");
  console.log("  pnpm capture --force  # Re-capture everything");
}

async function captureCases(
  cases: CaptureCase[],
  options: CaptureOptions,
): Promise<void> {
  // Initialize clients
  const openaiClient = process.env.OPENAI_API_KEY
    ? new OpenAI({ apiKey: process.env.OPENAI_API_KEY })
    : null;
  const anthropicClient = process.env.ANTHROPIC_API_KEY
    ? new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY })
    : null;

  const outputDir = join(__dirname, "..", "snapshots");
  mkdirSync(outputDir, { recursive: true });

  console.log(`\nStarting capture of ${cases.length} cases...`);

  // Group by provider for better logging
  const uniqueProviders = new Set(cases.map((c) => c.provider));
  const providers = Array.from(uniqueProviders);
  console.log(`Providers: ${providers.join(", ")}`);

  // Filter out already captured unless force mode
  let casesToRun = cases;
  if (!options.force) {
    const skipped = cases.filter((c) =>
      isAlreadyCaptured(c.name, c.provider, c.payload),
    );
    casesToRun = cases.filter(
      (c) => !isAlreadyCaptured(c.name, c.provider, c.payload),
    );

    if (skipped.length > 0) {
      console.log(
        `Skipping ${skipped.length} already captured cases (use --force to re-capture)`,
      );
    }
  }

  if (casesToRun.length === 0) {
    console.log("No cases to run!");
    return;
  }

  console.log(`Running ${casesToRun.length} cases...\n`);

  // Run all cases in parallel
  const promises = casesToRun.map(async (caseItem) => {
    const { name, provider, payload } = caseItem;

    try {
      switch (provider) {
        case "openai-chat":
          if (!openaiClient) throw new Error("OPENAI_API_KEY not provided");
          if (!isOpenAIChatPayload(payload)) {
            throw new Error(`Invalid OpenAI chat payload for ${name}`);
          }
          await captureSinglePayload(
            openaiClient,
            name,
            payload,
            outputDir,
            options.stream,
          );
          break;
        case "openai-responses":
          if (!openaiClient) throw new Error("OPENAI_API_KEY not provided");
          if (!isOpenAIResponsesPayload(payload)) {
            throw new Error(`Invalid OpenAI responses payload for ${name}`);
          }
          await captureSingleResponsesPayload(
            openaiClient,
            name,
            payload,
            outputDir,
            options.stream,
          );
          break;
        case "anthropic":
          if (!anthropicClient)
            throw new Error("ANTHROPIC_API_KEY not provided");
          if (!isAnthropicPayload(payload)) {
            throw new Error(`Invalid Anthropic payload for ${name}`);
          }
          await captureSingleAnthropicPayload(
            anthropicClient,
            name,
            payload,
            outputDir,
            options.stream,
          );
          break;
      }
    } catch (error) {
      console.error(`✗ Failed ${provider}/${name}:`, error);
    }
  });

  await Promise.allSettled(promises);
  console.log(`\nCapture complete! Results saved to: ${outputDir}`);
}

async function main() {
  const options = parseArguments();
  const allCases = getAllCases();
  const filteredCases = filterCases(allCases, options);

  if (options.list) {
    listCases(filteredCases);
    return;
  }

  if (filteredCases.length === 0) {
    console.error("No cases match the specified filters");
    listCases(allCases);
    process.exit(1);
  }

  await captureCases(filteredCases, options);
}

if (require.main === module) {
  main().catch(console.error);
}
