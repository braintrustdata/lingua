#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { readFile, rename, stat, unlink, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const JSON_NUMBER = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;

// JSON.parse coerces every number to an IEEE-754 value. Keep number tokens in
// the syntax tree so formatting cannot round or otherwise change them.
class LosslessJsonParser {
  constructor(source) {
    this.source = source;
    this.position = 0;
  }

  parse() {
    const value = this.parseValue();
    this.skipWhitespace();
    if (this.position !== this.source.length) {
      this.fail("Unexpected token");
    }
    return value;
  }

  parseValue() {
    this.skipWhitespace();
    const token = this.source[this.position];

    switch (token) {
      case "{":
        return this.parseObject();
      case "[":
        return this.parseArray();
      case '"':
        return { type: "string", value: this.parseString() };
      case "t":
        return this.parseLiteral("true", true);
      case "f":
        return this.parseLiteral("false", false);
      case "n":
        return this.parseLiteral("null", null);
      default:
        return this.parseNumber();
    }
  }

  parseObject() {
    this.position += 1;
    this.skipWhitespace();
    const entries = [];

    if (this.consume("}")) {
      return { type: "object", entries };
    }

    while (true) {
      if (this.source[this.position] !== '"') {
        this.fail("Expected an object key");
      }
      const key = this.parseString();
      this.skipWhitespace();
      this.expect(":");
      entries.push([key, this.parseValue()]);
      this.skipWhitespace();
      if (this.consume("}")) {
        break;
      }
      this.expect(",");
      this.skipWhitespace();
    }

    return { type: "object", entries };
  }

  parseArray() {
    this.position += 1;
    this.skipWhitespace();
    const values = [];

    if (this.consume("]")) {
      return { type: "array", values };
    }

    while (true) {
      values.push(this.parseValue());
      this.skipWhitespace();
      if (this.consume("]")) {
        break;
      }
      this.expect(",");
    }

    return { type: "array", values };
  }

  parseString() {
    const start = this.position;
    this.position += 1;

    while (this.position < this.source.length) {
      const token = this.source[this.position];
      this.position += 1;
      if (token === '"') {
        return JSON.parse(this.source.slice(start, this.position));
      }
      if (token === "\\") {
        this.position += 1;
      }
    }

    this.fail("Unterminated string");
  }

  parseLiteral(literal, value) {
    if (!this.source.startsWith(literal, this.position)) {
      this.fail("Unexpected token");
    }
    this.position += literal.length;
    return { type: "literal", value };
  }

  parseNumber() {
    JSON_NUMBER.lastIndex = this.position;
    const match = JSON_NUMBER.exec(this.source);
    if (!match) {
      this.fail("Expected a JSON value");
    }
    this.position = JSON_NUMBER.lastIndex;
    return { type: "number", source: match[0] };
  }

  skipWhitespace() {
    while (
      this.source[this.position] === " " ||
      this.source[this.position] === "\t" ||
      this.source[this.position] === "\n" ||
      this.source[this.position] === "\r"
    ) {
      this.position += 1;
    }
  }

  consume(token) {
    if (this.source[this.position] !== token) {
      return false;
    }
    this.position += 1;
    return true;
  }

  expect(token) {
    if (!this.consume(token)) {
      this.fail(`Expected ${JSON.stringify(token)}`);
    }
  }

  fail(message) {
    throw new SyntaxError(`${message} at position ${this.position}`);
  }
}

function formatLosslessJson(value, depth = 0) {
  const indent = "  ".repeat(depth);
  const childIndent = "  ".repeat(depth + 1);

  switch (value.type) {
    case "object": {
      if (value.entries.length === 0) {
        return "{}";
      }
      const entries = [...value.entries].sort(([left], [right]) =>
        left < right ? -1 : left > right ? 1 : 0
      );
      return `{
${entries
  .map(
    ([key, child]) =>
      `${childIndent}${JSON.stringify(key)}: ${formatLosslessJson(child, depth + 1)}`
  )
  .join(",\n")}
${indent}}`;
    }
    case "array":
      if (value.values.length === 0) {
        return "[]";
      }
      return `[
${value.values
  .map((child) => `${childIndent}${formatLosslessJson(child, depth + 1)}`)
  .join(",\n")}
${indent}]`;
    case "string":
      return JSON.stringify(value.value);
    case "number":
      return value.source;
    case "literal":
      return JSON.stringify(value.value);
    default:
      throw new TypeError(`Unknown JSON value type: ${value.type}`);
  }
}

export function canonicalizeJson(source) {
  return `${formatLosslessJson(new LosslessJsonParser(source).parse())}\n`;
}

export async function cleanupTempFile(
  tempPath,
  originalError,
  remove = unlink
) {
  try {
    await remove(tempPath);
  } catch (cleanupError) {
    if (cleanupError?.code === "ENOENT") {
      return;
    }

    const originalMessage =
      originalError instanceof Error
        ? originalError.message
        : String(originalError);
    const cleanupMessage =
      cleanupError instanceof Error
        ? cleanupError.message
        : String(cleanupError);
    throw new AggregateError(
      [originalError, cleanupError],
      `Failed to update JSON file: ${originalMessage}; failed to remove temporary file ${tempPath}: ${cleanupMessage}`,
      { cause: originalError }
    );
  }
}

export async function canonicalizeJsonFile(filePath) {
  const source = await readFile(filePath, "utf8");
  const canonical = canonicalizeJson(source);

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
    await cleanupTempFile(tempPath, error);
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
