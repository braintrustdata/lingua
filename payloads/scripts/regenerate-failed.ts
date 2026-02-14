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

// Extract failed test cases from all test files
const failedCases: string[] = [];
for (const testFile of results.testResults) {
  for (const assertion of testFile.assertionResults) {
    if (assertion.status === "failed") {
      // The title field contains the case name directly
      failedCases.push(assertion.title);
    }
  }
}

if (failedCases.length === 0) {
  console.log("No failed tests found.");
  process.exit(1);
}

// Remove duplicates
const uniqueCases = [...new Set(failedCases)];

console.log(`\n🔄 Regenerating ${uniqueCases.length} failed case(s):`);
uniqueCases.forEach((caseName) => console.log(`  - ${caseName}`));
console.log();

// Recapture transforms for each failed case
try {
  for (const caseName of uniqueCases) {
    console.log(`\n📦 Recapturing transforms for: ${caseName}`);
    execSync(
      `pnpm tsx scripts/transforms/capture-transforms.ts ${caseName} --force`,
      {
        cwd: __dirname + "/..",
        stdio: "inherit",
      }
    );
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
