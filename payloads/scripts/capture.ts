#!/usr/bin/env tsx

import { existsSync, mkdirSync } from "fs";
import { join } from "path";
import { execSync } from "child_process";
import { saveAllFiles } from "./file-manager";
import { captureTransforms } from "./transforms/capture-transforms";

// Import all providers
import { openaiExecutor } from "./providers/openai";
import { openaiResponsesExecutor } from "./providers/openai-responses";
import { anthropicExecutor } from "./providers/anthropic";
import { googleExecutor } from "./providers/google";
import { bedrockExecutor } from "./providers/bedrock";
import { bedrockAnthropicExecutor } from "./providers/bedrock-anthropic";
import { vertexAnthropicExecutor } from "./providers/vertex-anthropic";
import { type ProviderExecutor } from "./types";

// Update provider names to be more descriptive
const allProviders = [
  { ...openaiExecutor, name: "chat-completions" },
  openaiResponsesExecutor,
  anthropicExecutor,
  googleExecutor,
  bedrockExecutor,
  bedrockAnthropicExecutor,
  vertexAnthropicExecutor,
] as const;

interface CaptureOptions {
  list: boolean;
  force: boolean;
  filter?: string;
  providers?: string[];
  cases?: string[];
  stream?: boolean; // undefined = both, true = streaming only, false = non-streaming only
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
        if (i + 1 < args.length) {
          options.providers = args[i + 1].split(",");
          i++;
        }
        break;
      case "--cases":
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
            "Available options: --list, --force, --filter, --providers, --cases, --stream"
          );
          process.exit(1);
        }
    }
  }

  return options;
}

interface CaseToRun {
  provider: string;
  caseName: string;
  payload: unknown;
  executor: ProviderExecutor<unknown, unknown, unknown>;
}

function getAllCases(options: CaptureOptions): CaseToRun[] {
  const cases: CaseToRun[] = [];

  for (const executor of allProviders) {
    // Filter by provider if specified
    if (options.providers && !options.providers.includes(executor.name)) {
      continue;
    }

    for (const [caseName, payload] of Object.entries(executor.cases)) {
      // Filter by case name if specified
      if (options.cases && !options.cases.includes(caseName)) {
        continue;
      }

      // Filter by general filter if specified
      if (options.filter && !caseName.includes(options.filter)) {
        continue;
      }

      cases.push({
        provider: executor.name,
        caseName,
        payload,
        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Runtime type safety guaranteed by executor design
        executor: executor as ProviderExecutor<unknown, unknown, unknown>,
      });
    }
  }

  return cases;
}

async function captureProviderSnapshots(
  cases: CaseToRun[],
  options: CaptureOptions
): Promise<void> {
  const outputDir = join(__dirname, "..", "snapshots");
  mkdirSync(outputDir, { recursive: true });

  const casesToRun: CaseToRun[] = [];
  const skippedCases: CaseToRun[] = [];

  for (const case_ of cases) {
    const snapshotDir = join(outputDir, case_.caseName, case_.provider);
    if (!options.force && existsSync(snapshotDir)) {
      skippedCases.push(case_);
    } else {
      casesToRun.push(case_);
    }
  }

  if (skippedCases.length > 0) {
    console.log(
      `Skipping ${skippedCases.length} already captured cases (use --force to re-capture)`
    );
  }

  if (casesToRun.length === 0) {
    console.log("No cases to run!");
    return;
  }

  console.log(`Running ${casesToRun.length} cases in parallel...`);

  const casePromises = casesToRun.map(async (case_) => {
    const startTime = Date.now();
    console.log(`🚀 Starting ${case_.provider}/${case_.caseName}...`);

    try {
      const result = await case_.executor.execute(
        case_.caseName,
        case_.payload,
        { stream: options.stream }
      );

      const savedFiles = saveAllFiles(
        outputDir,
        case_.caseName,
        case_.provider,
        result
      );

      const duration = Date.now() - startTime;
      console.log(
        `✓ Completed ${case_.provider}/${case_.caseName} in ${duration}ms - saved ${savedFiles.length} files`
      );

      return { case_, success: true, duration, filesCount: savedFiles.length };
    } catch (error) {
      const duration = Date.now() - startTime;
      console.error(
        `✗ Failed ${case_.provider}/${case_.caseName} in ${duration}ms:`,
        error
      );

      return { case_, success: false, duration, error: String(error) };
    }
  });

  const results = await Promise.all(casePromises);

  const successful = results.filter((r) => r.success);
  const failed = results.filter((r) => !r.success);
  const totalDuration = Math.max(...results.map((r) => r.duration));
  const totalFiles = successful.reduce(
    (sum, r) => sum + (r.filesCount || 0),
    0
  );

  console.log(`\n📊 Execution Summary:`);
  console.log(`  ✅ Successful: ${successful.length}/${results.length}`);
  if (failed.length > 0) {
    console.log(`  ❌ Failed: ${failed.length}/${results.length}`);
  }
  console.log(`  ⏱️  Total time: ${totalDuration}ms (parallelized)`);
  console.log(`  📁 Total files saved: ${totalFiles}`);

  if (failed.length > 0) {
    console.log(`\n❌ Failed cases:`);
    for (const failure of failed) {
      console.log(
        `  - ${failure.case_.provider}/${failure.case_.caseName}: ${failure.error}`
      );
    }
  }
}

function updateVitestSnapshots(): void {
  console.log(`\n--- Updating vitest snapshots ---`);
  try {
    execSync("pnpm vitest run scripts/transforms -u", {
      cwd: join(__dirname, ".."),
      stdio: "inherit",
    });
  } catch {
    console.error("Failed to update vitest snapshots");
  }
}

async function main() {
  const options = parseArguments();
  const allCases = getAllCases(options);

  if (options.list) {
    console.log(`Found ${allCases.length} cases:`);
    for (const case_ of allCases) {
      console.log(`  ${case_.provider}/${case_.caseName}`);
    }
    return;
  }

  console.log(`\n--- Provider snapshots ---`);
  await captureProviderSnapshots(allCases, options);

  console.log(`\n--- Transform captures ---`);
  const forceTransforms = options.force && !options.providers;
  const transformResult = await captureTransforms(
    options.filter,
    forceTransforms
  );

  if (transformResult.captured > 0) {
    updateVitestSnapshots();
  }
}

if (require.main === module) {
  main().catch(console.error);
}
