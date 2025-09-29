/**
 * TypeScript Roundtrip Tests
 *
 * These tests validate that:
 * 1. SDK data can be converted to LLMIR format
 * 2. LLMIR data can be converted back to SDK format
 * 3. Data is preserved through the roundtrip conversion
 */

import { describe, test, expect } from "vitest";
import * as fs from "fs";
import * as path from "path";

// Import our generated types and conversion functions
import type { Message as LLMIRMessage } from "../src";
import {
  ConversionError,
  openAIMessageToLLMIR,
  anthropicMessageToLLMIR,
  llmirToOpenAIMessage,
  llmirToAnthropicMessage,
} from "../src";

interface TestSnapshot {
  name: string;
  provider: "openai-chat-completions" | "openai-responses" | "anthropic";
  turn: "first_turn" | "followup_turn";
  request?: unknown;
  response?: unknown;
  streamingResponse?: unknown;
}

/**
 * Load all snapshots for a given test case
 */
function loadTestSnapshots(testCaseName: string): TestSnapshot[] {
  const snapshots: TestSnapshot[] = [];
  // Snapshots are in the payloads directory
  const snapshotsDir = path.join(
    __dirname,
    "../../../payloads/snapshots",
    testCaseName,
  );

  const providers = [
    "openai-chat-completions",
    "openai-responses",
    "anthropic",
  ] as const;
  const turns = ["first_turn", "followup_turn"] as const;

  for (const provider of providers) {
    const providerDir = path.join(snapshotsDir, provider);

    if (!fs.existsSync(providerDir)) continue;

    for (const turn of turns) {
      const prefix = turn === "followup_turn" ? "followup-" : "";

      const snapshot: TestSnapshot = {
        name: testCaseName,
        provider,
        turn,
      };

      // Load request
      const requestPath = path.join(providerDir, `${prefix}request.json`);
      if (fs.existsSync(requestPath)) {
        snapshot.request = JSON.parse(fs.readFileSync(requestPath, "utf-8"));
      }

      // Load response
      const responsePath = path.join(providerDir, `${prefix}response.json`);
      if (fs.existsSync(responsePath)) {
        snapshot.response = JSON.parse(fs.readFileSync(responsePath, "utf-8"));
      }

      // Load streaming response
      const streamingPath = path.join(
        providerDir,
        `${prefix}response-streaming.json`,
      );
      if (fs.existsSync(streamingPath)) {
        const content = fs.readFileSync(streamingPath, "utf-8");
        try {
          // Try parsing as JSON array first (most common format)
          snapshot.streamingResponse = JSON.parse(content);
        } catch (e) {
          // If that fails, try newline-delimited JSON
          snapshot.streamingResponse = content
            .split("\n")
            .filter((line) => line.trim())
            .map((line) => {
              try {
                return JSON.parse(line);
              } catch (e) {
                return null;
              }
            })
            .filter((item) => item !== null);
        }
      }

      if (snapshot.request || snapshot.response || snapshot.streamingResponse) {
        snapshots.push(snapshot);
      }
    }
  }

  return snapshots;
}

describe("TypeScript Roundtrip Tests", () => {
  const snapshotsDir = path.join(__dirname, "../../../payloads/snapshots");

  // Get all test cases
  const testCases = fs.existsSync(snapshotsDir)
    ? fs
        .readdirSync(snapshotsDir)
        .filter((name) =>
          fs.statSync(path.join(snapshotsDir, name)).isDirectory(),
        )
        .filter((name) => !name.startsWith("."))
    : [];

  if (testCases.length === 0) {
    test("No test cases found", () => {
      console.warn(
        "No snapshot test cases found. Run capture script in payloads directory first.",
      );
      expect(testCases.length).toBeGreaterThan(0);
    });
    return;
  }

  for (const testCase of testCases) {
    describe(testCase, () => {
      const snapshots = loadTestSnapshots(testCase);

      if (snapshots.length === 0) {
        test.skip("No snapshots found for this test case", () => {});
        return;
      }

      for (const snapshot of snapshots) {
        const testName = `${snapshot.provider} - ${snapshot.turn}`;

        if (
          snapshot.provider === "openai-chat-completions" &&
          snapshot.request
        ) {
          test(`${testName}: full roundtrip conversion`, () => {
            const messages = snapshot.request.messages;
            if (Array.isArray(messages) && messages.length > 0) {
              // Test each message in the request
              for (const originalMessage of messages) {
                try {
                  // Perform the roundtrip: OpenAI -> LLMIR -> OpenAI
                  const result = testOpenAIRoundtrip(originalMessage);

                  // Verify the roundtrip preserved the data
                  expect(result.llmir).toBeDefined();
                  expect(result.llmir.role).toBeDefined();

                  // First check for type consistency (e.g., Map vs Object)
                  const typeError = checkTypeConsistency(originalMessage, result.roundtripped);
                  if (typeError) {
                    throw new Error(`Type consistency check failed: ${typeError}`);
                  }

                  // Then normalize both objects to remove null/undefined/empty arrays
                  // This matches how Rust's serde skips None values
                  const normalizedOriginal = normalizeForComparison(originalMessage);
                  const normalizedRoundtripped = normalizeForComparison(result.roundtripped);

                  // The normalized objects should be equal
                  expect(normalizedRoundtripped).toEqual(normalizedOriginal);
                } catch (error) {
                  if (error instanceof ConversionError) {
                    // Skip unsupported message formats for now
                    console.log(
                      `Skipping unsupported format in ${testName}:`,
                      error.message,
                    );
                  } else {
                    throw error;
                  }
                }
              }
            }
          });
        } else if (snapshot.provider === "anthropic" && snapshot.request) {
          test(`${testName}: full roundtrip conversion`, () => {
            const messages = snapshot.request.messages;
            if (Array.isArray(messages) && messages.length > 0) {
              // Test each message in the request
              for (const originalMessage of messages) {
                try {
                  // Perform the roundtrip: Anthropic -> LLMIR -> Anthropic
                  const result = testAnthropicRoundtrip(originalMessage);

                  // Verify the roundtrip preserved the data
                  expect(result.llmir).toBeDefined();
                  expect(result.llmir.role).toBeDefined();

                  // First check for type consistency (e.g., Map vs Object)
                  const typeError = checkTypeConsistency(originalMessage, result.roundtripped);
                  if (typeError) {
                    throw new Error(`Type consistency check failed: ${typeError}`);
                  }

                  // Then normalize both objects to remove null/undefined/empty arrays
                  // This matches how Rust's serde skips None values
                  const normalizedOriginal = normalizeForComparison(originalMessage);
                  const normalizedRoundtripped = normalizeForComparison(result.roundtripped);

                  // The normalized objects should be equal
                  expect(normalizedRoundtripped).toEqual(normalizedOriginal);
                } catch (error) {
                  if (error instanceof ConversionError) {
                    // Skip unsupported message formats for now
                    console.log(
                      `Skipping unsupported format in ${testName}:`,
                      error.message,
                    );
                  } else {
                    throw error;
                  }
                }
              }
            }
          });
        } else {
          test.skip(`${testName}: provider not yet supported`, () => {});
        }
      }
    });
  }

  describe("Test Coverage", () => {
    test("All test cases have snapshots", () => {
      const coverage: Record<string, { providers: string[]; turns: string[] }> =
        {};

      for (const testCase of testCases) {
        const snapshots = loadTestSnapshots(testCase);
        coverage[testCase] = {
          providers: [...new Set(snapshots.map((s) => s.provider))],
          turns: [...new Set(snapshots.map((s) => s.turn))],
        };
      }

      console.log("Test coverage by case:");
      for (const [testCase, data] of Object.entries(coverage)) {
        console.log(`  ${testCase}:`);
        console.log(`    Providers: ${data.providers.join(", ")}`);
        console.log(`    Turns: ${data.turns.join(", ")}`);
      }

      // Ensure each test case has at least some snapshots
      for (const testCase of testCases) {
        expect(coverage[testCase].providers.length).toBeGreaterThan(0);
      }
    });
  });

  // ============================================================================
// Test Helper Functions
// ============================================================================

/**
 * Check if two values have the same types recursively
 * Returns an error message if types don't match, or null if they do
 */
function checkTypeConsistency(original: unknown, roundtripped: unknown, path: string = ''): string | null {
  // Both null or undefined is OK
  if (original === null && roundtripped === null) return null;
  if (original === undefined && roundtripped === undefined) return null;

  // One is null/undefined but not the other
  if ((original === null || original === undefined) !== (roundtripped === null || roundtripped === undefined)) {
    return `Type mismatch at ${path}: original is ${original}, roundtripped is ${roundtripped}`;
  }

  // Check primitive types
  if (typeof original !== typeof roundtripped) {
    return `Type mismatch at ${path}: original is ${typeof original}, roundtripped is ${typeof roundtripped}`;
  }

  // Check array vs non-array
  if (Array.isArray(original) !== Array.isArray(roundtripped)) {
    return `Type mismatch at ${path}: original is ${Array.isArray(original) ? 'array' : 'not array'}, roundtripped is ${Array.isArray(roundtripped) ? 'array' : 'not array'}`;
  }

  // Check Map vs Object
  const origIsMap = original instanceof Map;
  const roundIsMap = roundtripped instanceof Map;
  if (origIsMap !== roundIsMap) {
    return `Type mismatch at ${path}: original is ${origIsMap ? 'Map' : 'Object'}, roundtripped is ${roundIsMap ? 'Map' : 'Object'}`;
  }

  // Recursively check arrays
  if (Array.isArray(original) && Array.isArray(roundtripped)) {
    const maxLen = Math.max(original.length, roundtripped.length);
    for (let i = 0; i < maxLen; i++) {
      const error = checkTypeConsistency(original[i], roundtripped[i], `${path}[${i}]`);
      if (error) return error;
    }
  }

  // Recursively check objects
  if (typeof original === 'object' && original !== null && !Array.isArray(original) && !origIsMap) {
    const origObj = original as Record<string, unknown>;
    const roundObj = roundtripped as Record<string, unknown>;
    const allKeys = new Set([...Object.keys(origObj), ...Object.keys(roundObj)]);

    for (const key of allKeys) {
      const error = checkTypeConsistency(origObj[key], roundObj[key], path ? `${path}.${key}` : key);
      if (error) return error;
    }
  }

  return null;
}

/**
 * Recursively normalize an object by removing null, undefined, and empty array values
 * This mimics how Rust's serde skips None values during serialization
 */
function normalizeForComparison(obj: unknown): unknown {
  if (obj === null || obj === undefined) {
    return undefined;
  }

  if (Array.isArray(obj)) {
    // Remove null/undefined from arrays and recursively normalize
    const normalized = obj
      .filter(item => item !== null && item !== undefined)
      .map(item => normalizeForComparison(item));
    // Return undefined for empty arrays to remove them
    return normalized.length === 0 ? undefined : normalized;
  }

  // Handle Map objects
  if (obj instanceof Map) {
    const normalized: Record<string, unknown> = {};
    for (const [key, value] of obj.entries()) {
      const normalizedValue = normalizeForComparison(value);
      if (normalizedValue !== undefined) {
        normalized[key] = normalizedValue;
      }
    }
    return Object.keys(normalized).length === 0 ? undefined : normalized;
  }

  if (typeof obj === 'object' && obj !== null) {
    const normalized: Record<string, unknown> = {};

    for (const [key, value] of Object.entries(obj)) {
      const normalizedValue = normalizeForComparison(value);

      // Only include the property if it's not undefined and not an empty array
      if (normalizedValue !== undefined) {
        normalized[key] = normalizedValue;
      }
    }

    // Return undefined for empty objects to remove them
    return Object.keys(normalized).length === 0 ? undefined : normalized;
  }

  // Primitive values are returned as-is
  return obj;
}

/**
 * Test roundtrip conversion: Provider -> LLMIR -> Provider
 * @throws {ConversionError} If any conversion step fails
 */
function testOpenAIRoundtrip(openAIMessage: unknown): {
  original: unknown;
  llmir: LLMIRMessage;
  roundtripped: unknown;
} {
  const llmir = openAIMessageToLLMIR(openAIMessage);
  const roundtripped = llmirToOpenAIMessage(llmir);

  return {
    original: openAIMessage,
    llmir,
    roundtripped
  };
}

/**
 * Test roundtrip conversion: Provider -> LLMIR -> Provider
 * @throws {ConversionError} If any conversion step fails
 */
function testAnthropicRoundtrip(anthropicMessage: unknown): {
  original: unknown;
  llmir: LLMIRMessage;
  roundtripped: unknown;
} {
  const llmir = anthropicMessageToLLMIR(anthropicMessage);
  const roundtripped = llmirToAnthropicMessage(llmir);

  return {
    original: anthropicMessage,
    llmir,
    roundtripped
  };
}

describe("Generated Types", () => {
    test("Module exports are available", async () => {
      const module = await import("../src");

      // Check that VERSION constant is exported
      expect(module.VERSION).toBeDefined();
      expect(module.VERSION).toBe("0.1.0");
    });

    test("TypeScript types compile correctly", () => {
      // This test just verifies that we can import the types
      // The actual type checking happens at compile time
      const testMessage: LLMIRMessage = {
        role: "user",
        content: "Test message",
      };

      expect(testMessage).toBeDefined();
      expect(testMessage.role).toBe("user");
    });
  });
});
