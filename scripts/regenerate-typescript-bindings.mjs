#!/usr/bin/env node

import { existsSync, mkdirSync, renameSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const generatedRoot = join(
  workspaceRoot,
  "bindings",
  "typescript",
  "src",
  "generated"
);
const providerIndex = process.argv.indexOf("--provider");
const provider =
  providerIndex === -1 ? undefined : process.argv[providerIndex + 1];
const supportedProviders = new Set(["anthropic", "google", "openai"]);
// `git diff` cannot see new files, so a renamed generated type passes a plain diff
// gate while its replacement is left uncommitted. `--check` regenerates and then
// fails on ANY reported change under the generated trees, additions included.
const checkOnly = process.argv.includes("--check");

if (providerIndex !== -1 && !supportedProviders.has(provider)) {
  console.error(
    `--provider must be one of ${[...supportedProviders].join(", ")}`
  );
  process.exit(2);
}

// Paths whose contents are fully derived from the generators.
const generatedPaths = [
  "bindings/typescript/src/generated",
  ...[...supportedProviders].map(
    (name) => `crates/lingua/src/providers/${name}/generated.rs`
  ),
];

const generatedDir = provider ? join(generatedRoot, provider) : undefined;
const backupDir = generatedDir
  ? `${generatedDir}.backup-${process.pid}`
  : undefined;
const hadGeneratedDir = generatedDir ? existsSync(generatedDir) : false;

if (generatedDir && backupDir) {
  rmSync(backupDir, { recursive: true, force: true });
  if (hadGeneratedDir) {
    renameSync(generatedDir, backupDir);
  }
  mkdirSync(generatedDir, { recursive: true });
}

const result = spawnSync(
  "cargo",
  ["test", "export_bindings", "--lib", "--quiet"],
  {
    cwd: workspaceRoot,
    stdio: "inherit",
  }
);

if (result.status === 0) {
  if (backupDir) {
    rmSync(backupDir, { recursive: true, force: true });
  }
  process.exit(checkOnly ? reportGeneratedDrift() : 0);
}

if (generatedDir && backupDir) {
  rmSync(generatedDir, { recursive: true, force: true });
  if (hadGeneratedDir) {
    renameSync(backupDir, generatedDir);
  }
}

if (result.error) {
  console.error(`Failed to generate TypeScript bindings: ${result.error}`);
}
process.exit(result.status ?? 1);

/**
 * Reports whether the generated trees differ from what is committed.
 *
 * Uses `git status --porcelain` rather than `git diff` so untracked additions
 * ("??" entries) fail too.
 *
 * @returns {number} process exit code
 */
function reportGeneratedDrift() {
  const status = spawnSync(
    "git",
    ["status", "--porcelain", "--", ...generatedPaths],
    { cwd: workspaceRoot, encoding: "utf8" }
  );

  if (status.status !== 0) {
    console.error(
      `Failed to inspect generated artifacts: ${
        status.error ?? status.stderr ?? "unknown git error"
      }`
    );
    return status.status ?? 1;
  }

  const entries = status.stdout
    .split("\n")
    .filter((line) => line.trim() !== "");
  if (entries.length === 0) {
    console.log("Generated artifacts are up to date.");
    return 0;
  }

  console.error(
    "Generated artifacts are out of date. Regenerate them and commit the result:"
  );
  for (const entry of entries) {
    console.error(`  ${entry}`);
  }
  console.error(
    "\nRun `make generate-all-providers` and `make generate-types`, then commit every\n" +
      "change under the generated trees (including newly added files)."
  );
  return 1;
}
