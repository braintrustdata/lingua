#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { readFile, rename, stat, unlink, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function sortJsonKeys(value) {
  if (Array.isArray(value)) {
    return value.map(sortJsonKeys);
  }

  if (value === null || typeof value !== "object") {
    return value;
  }

  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, sortJsonKeys(value[key])])
  );
}

export async function canonicalizeJsonFile(filePath) {
  const source = await readFile(filePath, "utf8");
  const canonical = `${JSON.stringify(sortJsonKeys(JSON.parse(source)), null, 2)}\n`;

  if (source === canonical) {
    return false;
  }

  const sourceStat = await stat(filePath);
  const tempPath = join(
    dirname(filePath),
    `.${basename(filePath)}.${process.pid}.${randomUUID()}.tmp`
  );

  try {
    await writeFile(tempPath, canonical, { mode: sourceStat.mode });
    await rename(tempPath, filePath);
  } catch (error) {
    await unlink(tempPath).catch(() => {});
    throw error;
  }

  return true;
}

async function main() {
  const [filePath] = process.argv.slice(2);
  if (!filePath) {
    throw new Error("Usage: canonicalize-json.mjs <json-file>");
  }

  const changed = await canonicalizeJsonFile(resolve(filePath));
  console.log(
    changed ? `Canonicalized ${filePath}` : `${filePath} is canonical`
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
