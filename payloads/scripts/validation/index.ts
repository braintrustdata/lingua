// Core validation library - runValidation()

import { readFileSync, existsSync } from "fs";
import { join } from "path";
import { compareResponses, DiffResult, hasOnlyMinorDiffs } from "./diff-utils";

// Import executors
import { openaiExecutor } from "../providers/openai";
import { openaiResponsesExecutor } from "../providers/openai-responses";
import { anthropicExecutor } from "../providers/anthropic";

// Import test cases from code
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  caseCollections,
} from "../../cases";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  ANTHROPIC_STRUCTURED_OUTPUT_MODEL,
  GOOGLE_MODEL,
  BEDROCK_MODEL,
} from "../../cases/models";

// Simplified executor interface for the registry (relaxes generic constraints)
interface ExecutorEntry {
  name: string;
  cases: Record<string, unknown>;
  execute: (
    caseName: string,
    payload: unknown,
    options?: { stream?: boolean; baseURL?: string; apiKey?: string }
  ) => Promise<{
    request: unknown;
    response?: unknown;
    streamingResponse?: unknown[];
    error?: string;
  }>;
  ignoredFields?: string[];
}

// Format registry - maps format names to executors
// Type assertions are necessary for heterogeneous executor types in the registry
/* eslint-disable @typescript-eslint/consistent-type-assertions */
const formatRegistry: Record<string, ExecutorEntry> = {
  "chat-completions": openaiExecutor as ExecutorEntry,
  responses: openaiResponsesExecutor as ExecutorEntry,
  anthropic: anthropicExecutor as ExecutorEntry,
};
/* eslint-enable @typescript-eslint/consistent-type-assertions */

/**
 * Type guard to check if value is a record with string keys.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Extract model name from actual API response.
 * Handles both streaming (array) and non-streaming (object) responses.
 */
function extractModelFromResponse(
  response: unknown,
  isStreaming?: boolean
): string | undefined {
  if (!response) return undefined;

  if (isStreaming && Array.isArray(response)) {
    // Streaming: model is in response[0].response.model
    const firstChunk: unknown = response[0];
    if (isRecord(firstChunk) && isRecord(firstChunk.response)) {
      const nested = firstChunk.response;
      if (typeof nested.model === "string") {
        return nested.model;
      }
    }
  } else if (isRecord(response)) {
    // Non-streaming: model is directly on response.model
    if (typeof response.model === "string") {
      return response.model;
    }
  }

  return undefined;
}

export interface ValidationOptions {
  proxyUrl: string;
  apiKey?: string; // API key to use (e.g., BRAINTRUST_API_KEY)
  formats?: string[]; // default: ['chat-completions']
  cases?: string[]; // default: DEFAULT_CASES (use `all: true` for all)
  providers?: string[]; // provider aliases to test (default: uses snapshot model)
  all?: boolean; // run all cases including slow ones
  stream?: boolean; // default: false (non-streaming only)
  verbose?: boolean; // include actual response in results
  onResult?: (result: ValidationResult) => void; // callback for streaming results
}

export interface ValidationResult {
  format: string;
  caseName: string;
  model: string; // model that was tested
  success: boolean;
  warning?: boolean; // true if success but only due to minor diffs (logprobs, tool args)
  durationMs: number;
  diff?: DiffResult; // only if success=false due to diff, or warning=true
  error?: string; // only if request failed
  actualResponse?: unknown; // the actual response from the proxy (when verbose)
}

/**
 * Get the snapshots directory path.
 */
function getSnapshotsDir(): string {
  return join(__dirname, "..", "..", "snapshots");
}

/**
 * Load a snapshot file (request.json or response.json).
 */
function loadSnapshotFile<T>(
  caseName: string,
  format: string,
  filename: string
): T | null {
  const filepath = join(getSnapshotsDir(), caseName, format, filename);
  if (!existsSync(filepath)) {
    return null;
  }
  const content = readFileSync(filepath, "utf-8");
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- JSON.parse returns unknown, caller provides expected type
  return JSON.parse(content) as T;
}

// Default cases to run (fast + representative)
const DEFAULT_CASES = ["simpleRequest", "toolCallRequest", "reasoningRequest"];

// Provider registry - maps provider aliases to actual model names (uses canonical models.ts)
const PROVIDER_REGISTRY: Record<string, string> = {
  openai: OPENAI_CHAT_COMPLETIONS_MODEL,
  anthropic: ANTHROPIC_STRUCTURED_OUTPUT_MODEL,
  google: GOOGLE_MODEL,
  bedrock: BEDROCK_MODEL,
};

/**
 * Get all available cases for a format from the cases definitions.
 */
function getAvailableCases(format: string): string[] {
  return getCaseNames(allTestCases).filter(
    (caseName) =>
      getCaseForProvider(
        allTestCases,
        caseName,
        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- format is a string key
        format as
          | "chat-completions"
          | "responses"
          | "anthropic"
          | "google"
          | "bedrock"
      ) != null
  );
}

/**
 * Get executor for a format.
 */
function getExecutorForFormat(format: string): ExecutorEntry | null {
  return formatRegistry[format] ?? null;
}

/**
 * Get list of available formats.
 */
export function getAvailableFormats(): string[] {
  return Object.keys(formatRegistry);
}

/**
 * Get list of available provider aliases.
 */
export function getAvailableProviders(): string[] {
  return Object.keys(PROVIDER_REGISTRY);
}

/**
 * Run validation against a proxy, comparing responses to snapshots.
 *
 * @param options - Validation options
 * @returns Array of validation results
 */
export async function runValidation(
  options: ValidationOptions
): Promise<ValidationResult[]> {
  const results: ValidationResult[] = [];
  const formats = options.formats ?? ["chat-completions"];
  // "default" means use the snapshot's model as-is
  const providersToTest = options.providers ?? ["default"];

  for (const format of formats) {
    const executor = getExecutorForFormat(format);
    if (!executor) {
      console.error(`Unknown format: ${format}`);
      continue;
    }

    // Get cases to run
    const availableCases = getAvailableCases(format);
    let caseNames: string[];
    if (options.cases) {
      // User specified explicit cases or collection names - expand collections
      const expandedCases = options.cases.flatMap(
        (c) => caseCollections[c] ?? [c]
      );
      caseNames = expandedCases.filter((c) => availableCases.includes(c));
    } else if (options.all) {
      // Run all available cases
      caseNames = availableCases;
    } else {
      // Use default cases (filtered to available)
      caseNames = DEFAULT_CASES.filter((c) => availableCases.includes(c));
    }

    if (caseNames.length === 0) {
      console.warn(`No cases found for format: ${format}`);
      continue;
    }

    // Run all (case, provider) combinations in parallel
    const testCombinations: Array<{ caseName: string; providerAlias: string }> =
      [];
    for (const caseName of caseNames) {
      for (const providerAlias of providersToTest) {
        testCombinations.push({ caseName, providerAlias });
      }
    }

    const caseResults = await Promise.all(
      testCombinations.map(
        async ({ caseName, providerAlias }): Promise<ValidationResult> => {
          const start = Date.now();

          // Resolve model name from provider alias
          const modelName =
            providerAlias === "default"
              ? "default"
              : (PROVIDER_REGISTRY[providerAlias] ?? providerAlias);

          try {
            // Get request from cases definitions (single source of truth)
            const caseRequest = getCaseForProvider(
              allTestCases,
              caseName,
              // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- format is a string key
              format as
                | "chat-completions"
                | "responses"
                | "anthropic"
                | "google"
                | "bedrock"
            );
            // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Need to type the request for model override
            let request = caseRequest as Record<string, unknown> | null;
            // Load expected response from snapshots for comparison
            const snapshotFilename = options.stream
              ? "response-streaming.json"
              : "response.json";
            const expectedResponse = loadSnapshotFile(
              caseName,
              format,
              snapshotFilename
            );

            if (!request) {
              const result: ValidationResult = {
                format,
                caseName,
                model: modelName,
                success: false,
                durationMs: Date.now() - start,
                error: `Case ${caseName} not found for format ${format}`,
              };
              options.onResult?.(result);
              return result;
            }

            if (!expectedResponse) {
              const result: ValidationResult = {
                format,
                caseName,
                model: modelName,
                success: false,
                durationMs: Date.now() - start,
                error: `Missing ${snapshotFilename} for ${caseName}/${format}`,
              };
              options.onResult?.(result);
              return result;
            }

            // Override model only for cross-provider testing
            // OpenAI formats (chat-completions, responses) with non-OpenAI providers
            if (
              providerAlias !== "default" &&
              providerAlias !== "openai" && // Don't override for OpenAI - tests have correct models
              PROVIDER_REGISTRY[providerAlias]
            ) {
              const isOpenAIFormat =
                format === "chat-completions" || format === "responses";
              if (isOpenAIFormat) {
                // Override for cross-provider translation testing
                request = {
                  ...request,
                  model: PROVIDER_REGISTRY[providerAlias],
                };
              }
            }

            // Execute through proxy
            const actual = await executor.execute(caseName, request, {
              stream: options.stream,
              baseURL: options.proxyUrl,
              apiKey: options.apiKey,
            });

            if (actual.error) {
              const result: ValidationResult = {
                format,
                caseName,
                model: modelName,
                success: false,
                durationMs: Date.now() - start,
                error: actual.error,
              };
              options.onResult?.(result);
              return result;
            }

            // Compare response (use streamingResponse array when streaming)
            const actualResponse = options.stream
              ? actual.streamingResponse
              : actual.response;

            // Extract actual model from response (fallback to registry-based name)
            const actualModel =
              extractModelFromResponse(actualResponse, options.stream) ??
              modelName;

            const diff = compareResponses(
              expectedResponse,
              actualResponse,
              executor.ignoredFields ?? []
            );

            // Determine success/warning state:
            // - success=true, warning=undefined: perfect match (no diffs)
            // - success=true, warning=true: only minor diffs (logprobs, tool args)
            // - success=false: major diffs or errors
            const onlyMinorDiffs = hasOnlyMinorDiffs(diff);
            const result: ValidationResult = {
              format,
              caseName,
              model: actualModel,
              success: diff.match || onlyMinorDiffs,
              warning: onlyMinorDiffs ? true : undefined,
              durationMs: Date.now() - start,
              diff: diff.match ? undefined : diff, // Include diff for warnings too
              actualResponse: options.verbose ? actualResponse : undefined,
            };
            options.onResult?.(result);
            return result;
          } catch (error) {
            const result: ValidationResult = {
              format,
              caseName,
              model: modelName,
              success: false,
              durationMs: Date.now() - start,
              error: String(error),
            };
            options.onResult?.(result);
            return result;
          }
        }
      )
    );

    results.push(...caseResults);
  }

  return results;
}

// Re-export types for convenience
export type { DiffResult, DiffEntry, DiffSeverity } from "./diff-utils";
export { compareResponses, formatDiff, hasOnlyMinorDiffs } from "./diff-utils";
