#!/usr/bin/env tsx

import { mkdirSync } from "fs";
import { join } from "path";
import { needsRegeneration, updateCache } from "./cache-utils";
import { saveAllFiles } from "./file-manager";

// Import all providers
import { openaiExecutor } from "./providers/openai";
import { openaiResponsesExecutor } from "./providers/openai-responses";
import { anthropicExecutor } from "./providers/anthropic";
import { ProviderExecutor } from "./types";

const allProviders: ProviderExecutor[] = [
  openaiExecutor,
  openaiResponsesExecutor,
  anthropicExecutor,
];

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
            "Available options: --list, --force, --filter, --providers, --cases, --stream",
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
  executor: ProviderExecutor;
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
        executor,
      });
    }
  }

  return cases;
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

  console.log(`\nStarting capture of ${allCases.length} cases...`);
  console.log(
    `Providers: ${[...new Set(allCases.map((c) => c.provider))].join(", ")}`,
  );

  const outputDir = join(__dirname, "..", "snapshots");
  mkdirSync(outputDir, { recursive: true });

  // Filter cases that need to be run
  const casesToRun: CaseToRun[] = [];
  const skippedCases: CaseToRun[] = [];

  for (const case_ of allCases) {
    if (
      !options.force &&
      !needsRegeneration(
        outputDir,
        case_.provider,
        case_.caseName,
        case_.payload,
      )
    ) {
      skippedCases.push(case_);
    } else {
      casesToRun.push(case_);
    }
  }

  if (skippedCases.length > 0) {
    console.log(
      `Skipping ${skippedCases.length} already captured cases (use --force to re-capture)`,
    );
  }

  if (casesToRun.length === 0) {
    console.log("No cases to run!");
    return;
  }

  console.log(`Running ${casesToRun.length} cases in parallel...`);

  // Execute all cases in parallel across providers
  const casePromises = casesToRun.map(async (case_) => {
    const startTime = Date.now();
    console.log(`🚀 Starting ${case_.provider}/${case_.caseName}...`);

    try {
      const result = await case_.executor.execute(
        case_.caseName,
        case_.payload,
        options.stream,
      );

      const savedFiles = saveAllFiles(
        outputDir,
        case_.caseName,
        case_.provider,
        result,
      );

      // Update cache with the files that were actually saved
      const relativeFiles = savedFiles.map((f) =>
        f.replace(outputDir + "/", ""),
      );
      updateCache(
        outputDir,
        case_.provider,
        case_.caseName,
        case_.payload,
        relativeFiles,
      );

      const duration = Date.now() - startTime;
      console.log(
        `✓ Completed ${case_.provider}/${case_.caseName} in ${duration}ms - saved ${savedFiles.length} files`,
      );

      return { case_, success: true, duration, filesCount: savedFiles.length };
    } catch (error) {
      const duration = Date.now() - startTime;
      console.error(
        `✗ Failed ${case_.provider}/${case_.caseName} in ${duration}ms:`,
        error,
      );

      return { case_, success: false, duration, error: String(error) };
    }
  });

  // Wait for all cases to complete
  const results = await Promise.all(casePromises);

  // Print summary
  const successful = results.filter((r) => r.success);
  const failed = results.filter((r) => !r.success);
  const totalDuration = Math.max(...results.map((r) => r.duration));
  const totalFiles = successful.reduce(
    (sum, r) => sum + (r.filesCount || 0),
    0,
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
        `  - ${failure.case_.provider}/${failure.case_.caseName}: ${failure.error}`,
      );
    }
  }

  console.log(`\nCapture complete! Results saved to: ${outputDir}`);
}

if (require.main === module) {
  main().catch(console.error);
}

