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

if (providerIndex !== -1 && !supportedProviders.has(provider)) {
  console.error(
    `--provider must be one of ${[...supportedProviders].join(", ")}`
  );
  process.exit(2);
}

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
  process.exit(0);
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
