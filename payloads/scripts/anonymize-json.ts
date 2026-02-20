#!/usr/bin/env tsx

import { readFile, writeFile } from "fs/promises";

type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

const DEFAULT_PRESERVE_KEYS = ["role", "type"];
const DEFAULT_TOKEN_PREFIX = "anon";

export interface AnonymizeOptions {
  allStrings?: boolean;
  preserveKeys?: Set<string>;
  tokenPrefix?: string;
}

export interface AnonymizeResult {
  value: JsonValue;
  replacedStringCount: number;
  uniqueReplacementCount: number;
}

function isMetadataModelPath(path: ReadonlyArray<string>): boolean {
  const n = path.length;
  return (
    n >= 2 &&
    path[n - 2].toLowerCase() === "metadata" &&
    path[n - 1].toLowerCase() === "model"
  );
}

function normalizeKeySet(keys: Set<string>): Set<string> {
  return new Set(Array.from(keys).map((key) => key.toLowerCase()));
}

export function anonymizeJsonValue(
  input: JsonValue,
  options: AnonymizeOptions = {}
): AnonymizeResult {
  const allStrings = options.allStrings ?? false;
  const tokenPrefix = options.tokenPrefix ?? DEFAULT_TOKEN_PREFIX;
  const preserveKeys = normalizeKeySet(
    options.preserveKeys ?? new Set(DEFAULT_PRESERVE_KEYS)
  );

  const replacements = new Map<string, string>();
  let nextToken = 1;
  let replacedStringCount = 0;

  function replaceString(value: string, currentKey?: string): string {
    if (value.length === 0) {
      return value;
    }

    if (
      !allStrings &&
      currentKey &&
      preserveKeys.has(currentKey.toLowerCase())
    ) {
      return value;
    }

    let token = replacements.get(value);
    if (!token) {
      token = `${tokenPrefix}_${nextToken++}`;
      replacements.set(value, token);
    }
    replacedStringCount += 1;
    return token;
  }

  function walk(
    value: JsonValue,
    path: string[],
    currentKey?: string,
    withinContent = false,
    withinMetadata = false,
    withinContext = false,
    withinOutput = false
  ): JsonValue {
    if (typeof value === "string") {
      if (isMetadataModelPath(path)) {
        return value;
      }

      if (
        !allStrings &&
        !withinContent &&
        !withinMetadata &&
        !withinContext &&
        !withinOutput
      ) {
        return value;
      }
      return replaceString(value, currentKey);
    }

    if (Array.isArray(value)) {
      return value.map((item, index) =>
        walk(
          item,
          [...path, String(index)],
          currentKey,
          withinContent,
          withinMetadata,
          withinContext,
          withinOutput
        )
      );
    }

    if (value && typeof value === "object") {
      const out: { [key: string]: JsonValue } = {};
      for (const [key, nested] of Object.entries(value)) {
        const lowerKey = key.toLowerCase();
        const childWithinContent = withinContent || lowerKey === "content";
        const childWithinMetadata =
          withinMetadata || lowerKey.startsWith("metadata");
        const childWithinContext = withinContext || lowerKey === "context";
        const childWithinOutput = withinOutput || lowerKey === "output";

        // Remove metadata.prompt entirely: prompt key names/shape can leak sensitive context.
        if (childWithinMetadata && lowerKey === "prompt") {
          continue;
        }

        out[key] = walk(
          nested,
          [...path, key],
          key,
          childWithinContent,
          childWithinMetadata,
          childWithinContext,
          childWithinOutput
        );
      }
      return out;
    }

    return value;
  }

  const value = walk(input, []);
  return {
    value,
    replacedStringCount,
    uniqueReplacementCount: replacements.size,
  };
}

interface CliOptions {
  inputPath?: string;
  outputPath?: string;
  allStrings: boolean;
  preserveKeys: Set<string>;
  tokenPrefix: string;
}

function usage(): string {
  return [
    "Usage:",
    "  pnpm anonymize -- <input.json> [--output <path>] [--all-strings]",
    "  pnpm anonymize -- --input <input.json> [--output <path>] [--preserve-keys role,type]",
    "",
    "Defaults:",
    "  --output same path as input (in-place)",
    "  anonymizes strings under 'content', 'metadata*', 'context', and 'output' subtrees",
    "  removes metadata*.prompt",
    "  preserves metadata.model",
    "  --preserve-keys role,type (ignored when --all-strings is set)",
    `  --prefix ${DEFAULT_TOKEN_PREFIX}`,
  ].join("\n");
}

function parsePreserveKeys(value: string): Set<string> {
  const keys = value
    .split(",")
    .map((item) => item.trim().toLowerCase())
    .filter((item) => item.length > 0);

  if (keys.length === 0) {
    throw new Error("Expected at least one key in --preserve-keys");
  }

  return new Set(keys);
}

function parseArgs(argv: string[]): CliOptions {
  const options: CliOptions = {
    allStrings: false,
    preserveKeys: new Set(DEFAULT_PRESERVE_KEYS),
    tokenPrefix: DEFAULT_TOKEN_PREFIX,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    switch (arg) {
      case "--input":
      case "-i":
        if (i + 1 >= argv.length) {
          throw new Error("Missing value for --input");
        }
        options.inputPath = argv[++i];
        break;
      case "--output":
      case "-o":
        if (i + 1 >= argv.length) {
          throw new Error("Missing value for --output");
        }
        options.outputPath = argv[++i];
        break;
      case "--all-strings":
        options.allStrings = true;
        break;
      case "--preserve-keys":
        if (i + 1 >= argv.length) {
          throw new Error("Missing value for --preserve-keys");
        }
        options.preserveKeys = parsePreserveKeys(argv[++i]);
        break;
      case "--prefix":
        if (i + 1 >= argv.length) {
          throw new Error("Missing value for --prefix");
        }
        options.tokenPrefix = argv[++i];
        if (options.tokenPrefix.length === 0) {
          throw new Error("--prefix cannot be empty");
        }
        break;
      case "--help":
      case "-h":
        console.log(usage());
        process.exit(0);
        break;
      default:
        if (arg.startsWith("-")) {
          throw new Error(`Unknown option: ${arg}`);
        }

        if (!options.inputPath) {
          options.inputPath = arg;
        } else {
          throw new Error(`Unexpected positional argument: ${arg}`);
        }
        break;
    }
  }

  if (!options.inputPath) {
    throw new Error("Missing input file path");
  }

  return options;
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  let options: CliOptions;
  try {
    options = parseArgs(args);
  } catch (error) {
    console.error(String(error));
    console.error("");
    console.error(usage());
    process.exit(1);
    return;
  }

  const inputPath = options.inputPath;
  if (!inputPath) {
    throw new Error("Missing input file path");
  }
  const outputPath = options.outputPath ?? inputPath;

  const raw = await readFile(inputPath, "utf8");

  let parsed: JsonValue;
  try {
    parsed = JSON.parse(raw) as JsonValue;
  } catch (error) {
    throw new Error(
      `Failed to parse JSON from '${inputPath}': ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }

  const anonymized = anonymizeJsonValue(parsed, {
    allStrings: options.allStrings,
    preserveKeys: options.preserveKeys,
    tokenPrefix: options.tokenPrefix,
  });

  const pretty = JSON.stringify(anonymized.value, null, 2) + "\n";
  await writeFile(outputPath, pretty, "utf8");

  console.log(`Wrote anonymized JSON to ${outputPath}`);
  console.log(
    `Replaced ${anonymized.replacedStringCount} string value(s) across ${anonymized.uniqueReplacementCount} unique value(s)`
  );
  if (!options.allStrings) {
    console.log("Scope: content + metadata + context + output");
  }
  if (!options.allStrings) {
    console.log(
      `Preserved keys: ${Array.from(options.preserveKeys).sort().join(", ")}`
    );
    console.log("Preserved path: metadata.model");
  }
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
