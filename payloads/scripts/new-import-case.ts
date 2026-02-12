#!/usr/bin/env tsx

import * as fs from "fs";
import * as path from "path";

interface CliOptions {
  name?: string;
  from?: string;
  keepFullSpan: boolean;
  force: boolean;
  dryRun: boolean;
}

interface ImportAssertions {
  expectedMessageCount: number;
  expectedRolesInOrder: string[];
  mustContainText: string[];
}

type JsonLike = unknown;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseArguments(): CliOptions {
  const args = process.argv.slice(2);
  const options: CliOptions = {
    keepFullSpan: false,
    force: false,
    dryRun: false,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case "--name":
        options.name = args[i + 1];
        i++;
        break;
      case "--from":
        options.from = args[i + 1];
        i++;
        break;
      case "--keep-full-span":
        options.keepFullSpan = true;
        break;
      case "--force":
        options.force = true;
        break;
      case "--dry-run":
        options.dryRun = true;
        break;
      case "--help":
        printHelp();
        process.exit(0);
      default:
        if (arg.startsWith("--")) {
          console.error(`Unknown option: ${arg}`);
          printHelp();
          process.exit(1);
        }
    }
  }

  if (!options.name) {
    console.error("--name is required");
    printHelp();
    process.exit(1);
  }

  if (!/^[a-z0-9][a-z0-9-]*$/.test(options.name)) {
    console.error(
      "--name must match: lowercase letters, numbers, and hyphens only"
    );
    process.exit(1);
  }

  return options;
}

function printHelp() {
  console.log(`Create import-case fixtures from a copied UI span.

Usage:
  pnpm new-import-case --name <case-name> [--from <path>] [--keep-full-span] [--force] [--dry-run]

Input source:
  --from <path>       Read JSON from file
  (default)           Read JSON from stdin

Options:
  --keep-full-span    Keep all span keys instead of trimming to {input, output}
  --force             Overwrite existing fixture files
  --dry-run           Print generated files without writing
`);
}

function readInputJson(options: CliOptions): string {
  if (options.from) {
    return fs.readFileSync(options.from, "utf-8");
  }

  const stdin = fs.readFileSync(0, "utf-8");
  if (!stdin.trim()) {
    throw new Error(
      "No input JSON found on stdin. Pass --from <path> or pipe/paste JSON into stdin."
    );
  }
  return stdin;
}

function normalizeSpans(raw: JsonLike): Record<string, unknown>[] {
  if (Array.isArray(raw)) {
    return raw.map((item) => {
      if (!isRecord(item)) {
        throw new Error("Array input must contain span objects");
      }
      return item;
    });
  }

  if (isRecord(raw)) {
    const obj = raw;
    if ("input" in obj || "output" in obj) {
      return [obj];
    }
  }

  throw new Error(
    "Input JSON must be either a single span object or an array of span objects"
  );
}

function trimSpan(span: Record<string, unknown>): Record<string, unknown> {
  const trimmed: Record<string, unknown> = {};
  if ("input" in span) trimmed.input = span.input;
  if ("output" in span) trimmed.output = span.output;
  return trimmed;
}

function collectRoles(value: unknown, roles: string[]) {
  if (Array.isArray(value)) {
    for (const item of value) {
      collectRoles(item, roles);
    }
    return;
  }

  if (!isRecord(value)) {
    return;
  }

  const obj = value;

  if (typeof obj.role === "string") {
    roles.push(obj.role);
  }

  if (obj.message && typeof obj.message === "object") {
    collectRoles(obj.message, roles);
  }

  for (const key of [
    "prompt",
    "messages",
    "input",
    "output",
    "choices",
    "result",
    "response",
  ]) {
    if (key in obj) {
      collectRoles(obj[key], roles);
    }
  }
}

function inferAssertions(spans: Record<string, unknown>[]): ImportAssertions {
  const roles: string[] = [];
  for (const span of spans) {
    collectRoles(span.input, roles);
    collectRoles(span.output, roles);
  }
  return {
    expectedMessageCount: roles.length,
    expectedRolesInOrder: roles,
    mustContainText: [],
  };
}

function main() {
  const options = parseArguments();
  const rawInput = readInputJson(options);
  const parsed: JsonLike = JSON.parse(rawInput);
  const spans = normalizeSpans(parsed);
  const outputSpans = options.keepFullSpan ? spans : spans.map(trimSpan);
  const assertions = inferAssertions(outputSpans);

  const importCasesDir = path.join(__dirname, "..", "import-cases");
  const spansPath = path.join(importCasesDir, `${options.name}.spans.json`);
  const assertionsPath = path.join(
    importCasesDir,
    `${options.name}.assertions.json`
  );

  const spansJson = JSON.stringify(outputSpans, null, 2) + "\n";
  const assertionsJson = JSON.stringify(assertions, null, 2) + "\n";

  if (options.dryRun) {
    console.log(`--- ${spansPath} ---`);
    console.log(spansJson);
    console.log(`--- ${assertionsPath} ---`);
    console.log(assertionsJson);
    return;
  }

  fs.mkdirSync(importCasesDir, { recursive: true });

  for (const filePath of [spansPath, assertionsPath]) {
    if (fs.existsSync(filePath) && !options.force) {
      throw new Error(
        `File already exists: ${filePath}. Use --force to overwrite.`
      );
    }
  }

  fs.writeFileSync(spansPath, spansJson, "utf-8");
  fs.writeFileSync(assertionsPath, assertionsJson, "utf-8");

  console.log(`Wrote: ${spansPath}`);
  console.log(`Wrote: ${assertionsPath}`);
  console.log(
    "Next: review assertions (especially expectedMessageCount/expectedRolesInOrder) and run tests."
  );
}

main();
