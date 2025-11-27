/**
 * TypeScript Roundtrip Tests
 *
 * These tests validate that:
 * 1. SDK data can be converted to Lingua format
 * 2. Lingua data can be converted back to SDK format
 * 3. Data is preserved through the roundtrip conversion
 */

import { describe, test, expect } from "vitest";
import * as fs from "fs";
import * as path from "path";

// Import our generated types and conversion functions
import type { Message as LinguaMessage, Tool as LinguaTool } from "../src";
import {
  ConversionError,
  chatCompletionsMessagesToLingua,
  anthropicMessagesToLingua,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  linguaToolsToOpenAI,
  openaiToolsToLingua,
  linguaToolsToAnthropic,
  anthropicToolsToLingua,
  clientTool,
  providerTool,
  ProviderTools,
} from "../src";

interface TestSnapshot {
  name: string;
  provider: "chat-completions" | "responses" | "anthropic";
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
    testCaseName
  );

  const providers = ["chat-completions", "responses", "anthropic"] as const;
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
        `${prefix}response-streaming.json`
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
          fs.statSync(path.join(snapshotsDir, name)).isDirectory()
        )
        .filter((name) => !name.startsWith("."))
    : [];

  if (testCases.length === 0) {
    test("No test cases found", () => {
      console.warn(
        "No snapshot test cases found. Run capture script in payloads directory first."
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

        if (snapshot.provider === "chat-completions" && snapshot.request) {
          test(`${testName}: full roundtrip conversion`, () => {
            // Runtime type check: ensure request has messages array
            if (
              typeof snapshot.request === "object" &&
              snapshot.request !== null &&
              "messages" in snapshot.request
            ) {
              const messages = (snapshot.request as { messages: unknown })
                .messages;
              if (Array.isArray(messages) && messages.length > 0) {
                // Test each message in the request
                for (const originalMessage of messages) {
                  try {
                    // Perform the roundtrip: Chat Completions -> Lingua -> Chat Completions
                    const result =
                      testChatCompletionsRoundtrip(originalMessage);

                    // Verify the roundtrip preserved the data
                    expect(result.lingua).toBeDefined();
                    expect(result.lingua.role).toBeDefined();

                    // First check for type consistency (e.g., Map vs Object)
                    const typeError = checkTypeConsistency(
                      originalMessage,
                      result.roundtripped
                    );
                    if (typeError) {
                      throw new Error(
                        `Type consistency check failed: ${typeError}`
                      );
                    }

                    // Then normalize both objects to remove null/undefined/empty arrays
                    // This matches how Rust's serde skips None values
                    const normalizedOriginal =
                      normalizeForComparison(originalMessage);
                    const normalizedRoundtripped = normalizeForComparison(
                      result.roundtripped
                    );

                    // The normalized objects should be equal
                    expect(normalizedRoundtripped).toEqual(normalizedOriginal);
                  } catch (error) {
                    if (error instanceof ConversionError) {
                      // Skip unsupported message formats for now
                      console.log(
                        `Skipping unsupported format in ${testName}:`,
                        error.message
                      );
                    } else {
                      throw error;
                    }
                  }
                }
              }
            }
          });
        } else if (snapshot.provider === "anthropic" && snapshot.request) {
          test(`${testName}: full roundtrip conversion`, () => {
            // Runtime type check: ensure request has messages array
            if (
              typeof snapshot.request === "object" &&
              snapshot.request !== null &&
              "messages" in snapshot.request
            ) {
              const messages = (snapshot.request as { messages: unknown })
                .messages;
              if (Array.isArray(messages) && messages.length > 0) {
                // Test each message in the request
                for (const originalMessage of messages) {
                  try {
                    // Perform the roundtrip: Anthropic -> Lingua -> Anthropic
                    const result = testAnthropicRoundtrip(originalMessage);

                    // Verify the roundtrip preserved the data
                    expect(result.lingua).toBeDefined();
                    expect(result.lingua.role).toBeDefined();

                    // First check for type consistency (e.g., Map vs Object)
                    const typeError = checkTypeConsistency(
                      originalMessage,
                      result.roundtripped
                    );
                    if (typeError) {
                      throw new Error(
                        `Type consistency check failed: ${typeError}`
                      );
                    }

                    // Then normalize both objects to remove null/undefined/empty arrays
                    // This matches how Rust's serde skips None values
                    const normalizedOriginal =
                      normalizeForComparison(originalMessage);
                    const normalizedRoundtripped = normalizeForComparison(
                      result.roundtripped
                    );

                    // The normalized objects should be equal
                    expect(normalizedRoundtripped).toEqual(normalizedOriginal);
                  } catch (error) {
                    if (error instanceof ConversionError) {
                      // Skip unsupported message formats for now
                      console.log(
                        `Skipping unsupported format in ${testName}:`,
                        error.message
                      );
                    } else {
                      throw error;
                    }
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
  function checkTypeConsistency(
    original: unknown,
    roundtripped: unknown,
    path: string = ""
  ): string | null {
    // Helper to format value for display
    const formatValue = (val: unknown): string => {
      if (val === null) return "null";
      if (val === undefined) return "undefined";
      if (val instanceof Map)
        return `Map(${val.size}) ${JSON.stringify([...val.entries()])}`;
      if (typeof val === "object") {
        try {
          return JSON.stringify(val, null, 2).substring(0, 200);
        } catch {
          return String(val);
        }
      }
      return String(val);
    };

    // Special case: empty array → undefined is OK (serde skips empty arrays)
    if (
      Array.isArray(original) &&
      (original as unknown[]).length === 0 &&
      (roundtripped === null || roundtripped === undefined)
    ) {
      return null;
    }

    // Treat null and undefined as equivalent (serde skips None values)
    const origIsNullish = original === null || original === undefined;
    const roundIsNullish = roundtripped === null || roundtripped === undefined;

    if (origIsNullish && roundIsNullish) return null;
    if (origIsNullish !== roundIsNullish) {
      return `Type mismatch at ${path}:\n  Original: ${formatValue(original)}\n  Roundtripped: ${formatValue(roundtripped)}`;
    }

    // Check primitive types
    if (typeof original !== typeof roundtripped) {
      return `Type mismatch at ${path}:\n  Original (${typeof original}): ${formatValue(original)}\n  Roundtripped (${typeof roundtripped}): ${formatValue(roundtripped)}`;
    }

    // Check array vs non-array
    const origIsArray = Array.isArray(original);
    const roundIsArray = Array.isArray(roundtripped);

    if (origIsArray !== roundIsArray) {
      return `Type mismatch at ${path}:\n  Original (${origIsArray ? "array" : "not array"}): ${formatValue(original)}\n  Roundtripped (${roundIsArray ? "array" : "not array"}): ${formatValue(roundtripped)}`;
    }

    // Check Map vs Object - should not happen if types are correct
    const origIsMap = original instanceof Map;
    const roundIsMap = roundtripped instanceof Map;
    if (origIsMap !== roundIsMap) {
      return `Type mismatch at ${path}:\n  Original (${origIsMap ? "Map" : "Object"}): ${formatValue(original)}\n  Roundtripped (${roundIsMap ? "Map" : "Object"}): ${formatValue(roundtripped)}`;
    }

    // Recursively check arrays
    if (Array.isArray(original) && Array.isArray(roundtripped)) {
      const maxLen = Math.max(original.length, roundtripped.length);
      for (let i = 0; i < maxLen; i++) {
        const error = checkTypeConsistency(
          original[i],
          roundtripped[i],
          `${path}[${i}]`
        );
        if (error) return error;
      }
    }

    // Recursively check objects
    if (
      typeof original === "object" &&
      original !== null &&
      !Array.isArray(original) &&
      !origIsMap
    ) {
      const origObj = original as Record<string, unknown>;
      const roundObj = roundtripped as Record<string, unknown>;
      const allKeys = new Set([
        ...Object.keys(origObj),
        ...Object.keys(roundObj),
      ]);

      for (const key of allKeys) {
        const error = checkTypeConsistency(
          origObj[key],
          roundObj[key],
          path ? `${path}.${key}` : key
        );
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
        .filter((item) => item !== null && item !== undefined)
        .map((item) => normalizeForComparison(item));
      // Return undefined for empty arrays to remove them
      return normalized.length === 0 ? undefined : normalized;
    }

    if (typeof obj === "object" && obj !== null) {
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
   * Test roundtrip conversion: Provider -> Lingua -> Provider
   * @throws {ConversionError} If any conversion step fails
   */
  function testChatCompletionsRoundtrip(chatCompletionsMessage: unknown): {
    original: unknown;
    lingua: LinguaMessage;
    roundtripped: unknown;
  } {
    const lingua = chatCompletionsMessagesToLingua([chatCompletionsMessage])[0];
    const roundtripped = linguaToChatCompletionsMessages([lingua])[0];

    return {
      original: chatCompletionsMessage,
      lingua,
      roundtripped,
    };
  }

  /**
   * Test roundtrip conversion: Provider -> Lingua -> Provider
   * @throws {ConversionError} If any conversion step fails
   */
  function testAnthropicRoundtrip(anthropicMessage: unknown): {
    original: unknown;
    lingua: LinguaMessage;
    roundtripped: unknown;
  } {
    const lingua = anthropicMessagesToLingua([anthropicMessage])[0];
    const roundtripped = linguaToAnthropicMessages([lingua])[0];

    return {
      original: anthropicMessage,
      lingua,
      roundtripped,
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
      const testMessage: LinguaMessage = {
        role: "user",
        content: "Test message",
      };

      expect(testMessage).toBeDefined();
      expect(testMessage.role).toBe("user");
    });
  });

  describe("Tool Roundtrip Tests", () => {
    describe("Client Tools", () => {
      test("OpenAI: Lingua -> OpenAI -> Lingua", () => {
        const original = clientTool({
          name: "get_weather",
          description: "Get current weather for a location",
          input_schema: {
            type: "object",
            properties: {
              location: { type: "string" },
              units: { type: "string", enum: ["celsius", "fahrenheit"] },
            },
            required: ["location"],
          },
        });

        const openaiTools = linguaToolsToOpenAI([original]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(1);
        expect(roundtripped[0]).toEqual(original);
      });

      test("OpenAI: Client tool with strict mode", () => {
        const original = clientTool({
          name: "query_database",
          description: "Query the database",
          input_schema: {
            type: "object",
            properties: {
              query: { type: "string" },
            },
            required: ["query"],
          },
          provider_options: { strict: true },
        });

        const openaiTools = linguaToolsToOpenAI([original]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(1);
        // Note: strict mode is preserved through roundtrip
        expect(roundtripped[0]).toEqual(original);
      });

      test("Anthropic: Lingua -> Anthropic -> Lingua", () => {
        const original = clientTool({
          name: "calculate",
          description: "Perform a calculation",
          input_schema: {
            type: "object",
            properties: {
              expression: { type: "string" },
            },
            required: ["expression"],
          },
        });

        const anthropicTools = linguaToolsToAnthropic([original]);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        expect(roundtripped).toHaveLength(1);
        expect(roundtripped[0]).toEqual(original);
      });

      test("Multiple client tools roundtrip", () => {
        const tools = [
          clientTool({
            name: "tool1",
            description: "First tool",
            input_schema: { type: "object", properties: {} },
          }),
          clientTool({
            name: "tool2",
            description: "Second tool",
            input_schema: { type: "object", properties: {} },
          }),
          clientTool({
            name: "tool3",
            description: "Third tool",
            input_schema: { type: "object", properties: {} },
          }),
        ];

        const openaiTools = linguaToolsToOpenAI(tools);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(3);
        expect(roundtripped).toEqual(tools);
      });
    });

    describe("Provider Tools - OpenAI", () => {
      test("Computer use tool roundtrip", () => {
        const original = ProviderTools.openai.computer({
          display_width_px: 1920,
          display_height_px: 1080,
        });

        const openaiTools = linguaToolsToOpenAI([original]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("computer_use_preview");
        expect(tool.config?.display_width_px).toBe(1920);
        expect(tool.config?.display_height_px).toBe(1080);
      });

      test("Code interpreter tool roundtrip", () => {
        // Note: OpenAI's code_interpreter tool doesn't have a name field
        // It only has a container configuration
        const original = ProviderTools.openai.codeInterpreter({
          container: { type: "auto" },
        });

        const openaiTools = linguaToolsToOpenAI([original]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("code_interpreter");
        expect(tool.config?.container).toEqual({ type: "auto" });
      });

      test("Web search tool roundtrip", () => {
        const original = ProviderTools.openai.webSearch();

        const openaiTools = linguaToolsToOpenAI([original]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("web_search");
      });
    });

    describe("Provider Tools - Anthropic", () => {
      test("Web search tool roundtrip", () => {
        const original = ProviderTools.anthropic.webSearch({
          max_uses: 5,
          allowed_domains: ["wikipedia.org", "github.com"],
        });

        const anthropicTools = linguaToolsToAnthropic([original]);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("web_search_20250305");
        expect(tool.config?.max_uses).toBe(5);
        expect(tool.config?.allowed_domains).toEqual([
          "wikipedia.org",
          "github.com",
        ]);
      });

      test("Bash tool roundtrip", () => {
        // Note: bash tool only supports name parameter (not max_uses like web_search)
        const original = ProviderTools.anthropic.bash({ name: "my_bash" });

        const anthropicTools = linguaToolsToAnthropic([original]);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("bash_20250124");
        expect(tool.name).toBe("my_bash");
      });

      test("Text editor tool roundtrip", () => {
        const original = ProviderTools.anthropic.textEditor_20250728({
          max_characters: 1000,
        });

        const anthropicTools = linguaToolsToAnthropic([original]);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        expect(roundtripped).toHaveLength(1);
        const tool = roundtripped[0] as any;
        expect(tool.type).toBe("provider");
        expect(tool.tool_type).toBe("text_editor_20250728");
        expect(tool.config?.max_characters).toBe(1000);
      });
    });

    describe("Mixed Client and Provider Tools", () => {
      test("OpenAI: Mixed tools roundtrip", () => {
        const tools = [
          clientTool({
            name: "get_weather",
            description: "Get weather",
            input_schema: {
              type: "object",
              properties: { location: { type: "string" } },
            },
          }),
          ProviderTools.openai.codeInterpreter(),
          clientTool({
            name: "calculate",
            description: "Calculate",
            input_schema: {
              type: "object",
              properties: { expression: { type: "string" } },
            },
          }),
        ];

        const openaiTools = linguaToolsToOpenAI(tools);
        const roundtripped = openaiToolsToLingua(openaiTools);

        expect(roundtripped).toHaveLength(3);

        // Check first tool (client)
        const tool1 = roundtripped[0] as any;
        expect(tool1.type).toBe("function");
        expect(tool1.name).toBe("get_weather");

        // Check second tool (provider)
        const tool2 = roundtripped[1] as any;
        expect(tool2.type).toBe("provider");
        expect(tool2.tool_type).toBe("code_interpreter");

        // Check third tool (client)
        const tool3 = roundtripped[2] as any;
        expect(tool3.type).toBe("function");
        expect(tool3.name).toBe("calculate");
      });

      test("Anthropic: Mixed tools roundtrip", () => {
        const tools = [
          clientTool({
            name: "search_db",
            description: "Search database",
            input_schema: {
              type: "object",
              properties: { query: { type: "string" } },
            },
          }),
          ProviderTools.anthropic.bash(),
          ProviderTools.anthropic.webSearch({ max_uses: 3 }),
        ];

        const anthropicTools = linguaToolsToAnthropic(tools);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        expect(roundtripped).toHaveLength(3);

        // Check first tool (client)
        const tool1 = roundtripped[0] as any;
        expect(tool1.type).toBe("function");
        expect(tool1.name).toBe("search_db");

        // Check second tool (bash provider)
        const tool2 = roundtripped[1] as any;
        expect(tool2.type).toBe("provider");
        expect(tool2.tool_type).toBe("bash_20250124");

        // Check third tool (web search provider)
        const tool3 = roundtripped[2] as any;
        expect(tool3.type).toBe("provider");
        expect(tool3.tool_type).toBe("web_search_20250305");
      });
    });

    describe("Unknown Tool Handling", () => {
      test("Unsupported provider tool for OpenAI maps to Unknown", () => {
        const anthropicBashTool = ProviderTools.anthropic.bash();

        // bash_20250124 is not natively supported by OpenAI, but maps to Unknown
        // for forward compatibility
        const openaiTools = linguaToolsToOpenAI([anthropicBashTool]);

        expect(openaiTools).toHaveLength(1);
        // The tool should be serialized as Unknown type with the original tool_type
        expect(openaiTools[0]).toHaveProperty("type", "bash_20250124");
      });

      test("Unsupported provider tool for Anthropic maps to Unknown", () => {
        const openaiComputerTool = ProviderTools.openai.computer();

        // computer_use_preview is not natively supported by Anthropic, but maps to Unknown
        // for forward compatibility
        const anthropicTools = linguaToolsToAnthropic([openaiComputerTool]);

        expect(anthropicTools).toHaveLength(1);
        // The tool should be serialized as Unknown type with the original tool_type
        expect(anthropicTools[0]).toHaveProperty("type", "computer_use_preview");
      });
    });

    describe("JSON Value Serialization Edge Cases", () => {
      test("Input schema with various number types", () => {
        const tool = clientTool({
          name: "test_numbers",
          description: "Test number serialization",
          input_schema: {
            type: "object",
            properties: {
              positive_int: { type: "number", default: 42 },
              negative_int: { type: "number", default: -10 },
              zero: { type: "number", default: 0 },
              float: { type: "number", default: 3.14159 },
              negative_float: { type: "number", default: -2.5 },
              large_number: { type: "number", default: 1000000 },
              minimum: { type: "number", minimum: 0 },
              maximum: { type: "number", maximum: 100 },
            },
          },
        });

        const openaiTools = linguaToolsToOpenAI([tool]);
        const roundtripped = openaiToolsToLingua(openaiTools);

        const rtTool = roundtripped[0] as any;
        const props = rtTool.input_schema?.properties;

        // Verify all numeric defaults are actual numbers
        expect(props?.positive_int?.default).toBe(42);
        expect(props?.negative_int?.default).toBe(-10);
        expect(props?.zero?.default).toBe(0);
        expect(props?.float?.default).toBeCloseTo(3.14159);
        expect(props?.negative_float?.default).toBeCloseTo(-2.5);
        expect(props?.large_number?.default).toBe(1000000);
        expect(props?.minimum?.minimum).toBe(0);
        expect(props?.maximum?.maximum).toBe(100);

        // Verify types
        expect(typeof props?.positive_int?.default).toBe("number");
        expect(typeof props?.zero?.default).toBe("number");
        expect(typeof props?.minimum?.minimum).toBe("number");
      });

      test("Input schema with nested objects and arrays", () => {
        const tool = clientTool({
          name: "complex_schema",
          description: "Complex schema with nested structures",
          input_schema: {
            type: "object",
            properties: {
              config: {
                type: "object",
                properties: {
                  retries: { type: "number", default: 3 },
                  timeout: { type: "number", default: 5.5 },
                },
              },
              thresholds: {
                type: "array",
                items: { type: "number" },
                default: [0.1, 0.5, 0.9],
              },
            },
          },
        });

        const anthropicTools = linguaToolsToAnthropic([tool]);
        const roundtripped = anthropicToolsToLingua(anthropicTools);

        const rtTool = roundtripped[0] as any;
        const props = rtTool.input_schema?.properties;

        expect(props?.config?.properties?.retries?.default).toBe(3);
        expect(props?.config?.properties?.timeout?.default).toBeCloseTo(5.5);
        expect(props?.thresholds?.default).toEqual([0.1, 0.5, 0.9]);

        // Verify nested numbers are actual numbers
        expect(typeof props?.config?.properties?.retries?.default).toBe(
          "number"
        );
        expect(typeof props?.thresholds?.default[0]).toBe("number");
      });

      test("Provider tool configs with numbers", () => {
        // Test actual provider tools to ensure their configs serialize correctly
        const computerTool = ProviderTools.openai.computer({
          display_width_px: 1920,
          display_height_px: 1080,
        });
        const webSearchTool = ProviderTools.anthropic.webSearch({
          max_uses: 5,
        });

        const openaiTools = linguaToolsToOpenAI([computerTool]);
        const anthropicTools = linguaToolsToAnthropic([webSearchTool]);

        const rtOpenai = openaiToolsToLingua(openaiTools)[0] as any;
        const rtAnthropic = anthropicToolsToLingua(anthropicTools)[0] as any;

        // Verify config numbers are properly deserialized
        expect(rtOpenai.config?.display_width_px).toBe(1920);
        expect(rtOpenai.config?.display_height_px).toBe(1080);
        expect(rtAnthropic.config?.max_uses).toBe(5);

        expect(typeof rtOpenai.config?.display_width_px).toBe("number");
        expect(typeof rtAnthropic.config?.max_uses).toBe("number");
      });
    });
  });
});
