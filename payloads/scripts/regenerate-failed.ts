#!/usr/bin/env tsx
import { execSync } from "child_process";

console.log("Running tests to detect failures...\n");

try {
  // Run tests and capture output
  execSync("pnpm vitest run scripts/transforms --reporter=verbose", {
    cwd: __dirname + "/..",
    stdio: "inherit",
  });

  console.log("\n✅ All tests passed! Nothing to regenerate.");
  process.exit(0);
} catch {
  // Tests failed, parse output to find failed cases
  console.log("\n❌ Tests failed. Detecting failed test cases...\n");
}

// Run tests again with JSON reporter to get structured output
let testOutput: string;
try {
  testOutput = execSync("pnpm vitest run scripts/transforms --reporter=json", {
    cwd: __dirname + "/..",
    encoding: "utf-8",
    stdio: ["pipe", "pipe", "pipe"],
  });
} catch (error: unknown) {
  // execSync throws when exit code is non-zero, but we still get stdout
  if (
    error &&
    typeof error === "object" &&
    "stdout" in error &&
    typeof error.stdout === "string"
  ) {
    testOutput = error.stdout;
  } else {
    testOutput = "";
  }
  if (!testOutput) {
    console.error("Failed to get test output");
    process.exit(1);
  }
}

// Parse JSON output
const results = JSON.parse(testOutput);

// Extract failed test cases grouped by transform pair
const failedByPair: Map<string, Set<string>> = new Map();
for (const testFile of results.testResults) {
  for (const assertion of testFile.assertionResults) {
    if (assertion.status === "failed") {
      const pair = assertion.ancestorTitles[0] ?? "unknown";
      const existing = failedByPair.get(pair);
      if (existing) {
        existing.add(assertion.title);
      } else {
        failedByPair.set(pair, new Set([assertion.title]));
      }
    }
  }
}

if (failedByPair.size === 0) {
  console.log("No failed tests found.");
  process.exit(1);
}

let totalCases = 0;
console.log(`\n🔄 Regenerating failed case(s):`);
for (const [pair, cases] of failedByPair) {
  for (const caseName of cases) {
    console.log(`  - ${pair} / ${caseName}`);
    totalCases++;
  }
}
console.log(`\n${totalCases} case(s) across ${failedByPair.size} pair(s)\n`);

// Recapture transforms scoped to the specific provider pair
try {
  for (const [pair, cases] of failedByPair) {
    const [source, target] = pair.split(" → ");
    for (const caseName of cases) {
      console.log(`\n📦 Recapturing: ${pair} / ${caseName}`);
      execSync(
        `pnpm tsx scripts/transforms/capture-transforms.ts ${caseName} --force --pair ${source},${target}`,
        {
          cwd: __dirname + "/..",
          stdio: "inherit",
        }
      );
    }
  }

  console.log("\n✅ Transform captures complete! Updating snapshots...\n");

  // Update vitest snapshots
  execSync("pnpm vitest run scripts/transforms -u", {
    cwd: __dirname + "/..",
    stdio: "inherit",
  });

  console.log("\n✅ Snapshots updated! Re-running tests to verify...\n");

  // Re-run tests to verify
  execSync("pnpm vitest run scripts/transforms", {
    cwd: __dirname + "/..",
    stdio: "inherit",
  });

  console.log("\n✅ All tests passed!");
} catch {
  console.error("\n❌ Regeneration failed.");
  process.exit(1);
}
