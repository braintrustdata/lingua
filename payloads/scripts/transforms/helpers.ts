import { readFileSync } from "fs";
import { join } from "path";
import {
  transform_request,
  transform_response,
  validate_anthropic_request,
  validate_anthropic_response,
  validate_chat_completions_request,
  validate_chat_completions_response,
  validate_responses_request,
  validate_responses_response,
} from "@braintrust/lingua-wasm";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";
import {
  ANTHROPIC_MODEL,
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
} from "../../cases/models";

export type SourceFormat = "chat-completions" | "responses" | "anthropic";
export type WasmFormat = "OpenAI" | "Responses" | "Anthropic";

export interface TransformPair {
  source: SourceFormat;
  target: SourceFormat;
  wasmSource: WasmFormat;
  wasmTarget: WasmFormat;
}

export const TRANSFORM_PAIRS: TransformPair[] = [
  {
    source: "chat-completions",
    target: "anthropic",
    wasmSource: "OpenAI",
    wasmTarget: "Anthropic",
  },
  {
    source: "responses",
    target: "anthropic",
    wasmSource: "Responses",
    wasmTarget: "Anthropic",
  },
  {
    source: "anthropic",
    target: "chat-completions",
    wasmSource: "Anthropic",
    wasmTarget: "OpenAI",
  },
  {
    source: "anthropic",
    target: "responses",
    wasmSource: "Anthropic",
    wasmTarget: "Responses",
  },
];

// Validation functions by format
const REQUEST_VALIDATORS: Record<SourceFormat, (json: string) => unknown> = {
  "chat-completions": validate_chat_completions_request,
  responses: validate_responses_request,
  anthropic: validate_anthropic_request,
};

const RESPONSE_VALIDATORS: Record<SourceFormat, (json: string) => unknown> = {
  "chat-completions": validate_chat_completions_response,
  responses: validate_responses_response,
  anthropic: validate_anthropic_response,
};

interface TransformResultData {
  passThrough?: boolean;
  transformed?: boolean;
  data: unknown;
  sourceFormat?: string;
}

function isTransformResultData(value: unknown): value is TransformResultData {
  return typeof value === "object" && value !== null && "data" in value;
}

// Transform and validate request
export function transformAndValidateRequest(
  input: unknown,
  wasmTarget: WasmFormat,
  targetFormat: SourceFormat
): unknown {
  const result = transform_request(JSON.stringify(input), wasmTarget);
  if (!isTransformResultData(result)) {
    throw new Error("Invalid transform result");
  }
  const json = JSON.stringify(result.data);

  // Validate against Lingua's schema (derived from OpenAPI specs)
  REQUEST_VALIDATORS[targetFormat](json);

  return result.data;
}

// Validate and load response from file
export function loadAndValidateResponse(
  path: string,
  format: SourceFormat
): unknown {
  const json = readFileSync(path, "utf-8");

  // Validate against Lingua's schema
  RESPONSE_VALIDATORS[format](json);

  return JSON.parse(json);
}

// Map WasmFormat to SourceFormat for validation
const WASM_TO_SOURCE: Record<WasmFormat, SourceFormat> = {
  OpenAI: "chat-completions",
  Responses: "responses",
  Anthropic: "anthropic",
};

// Transform and validate response
export function transformResponseData(
  response: unknown,
  wasmSource: WasmFormat
): unknown {
  const result = transform_response(JSON.stringify(response), wasmSource);
  if (!isTransformResultData(result)) {
    throw new Error("Invalid transform result");
  }
  const json = JSON.stringify(result.data);

  // Validate transformed response against source format's schema
  const sourceFormat = WASM_TO_SOURCE[wasmSource];
  RESPONSE_VALIDATORS[sourceFormat](json);

  return result.data;
}

// Export validators for capture script
export { RESPONSE_VALIDATORS };

// Shared utilities for capture and test scripts
export const TRANSFORMS_DIR = join(__dirname, "../../transforms");

export const isParamCase = (name: string) => name.endsWith("Param");

export function getResponsePath(
  source: string,
  target: string,
  caseName: string
): string {
  return join(TRANSFORMS_DIR, `${source}_to_${target}`, `${caseName}.json`);
}

export const TARGET_MODELS: Record<SourceFormat, string> = {
  anthropic: ANTHROPIC_MODEL,
  "chat-completions": OPENAI_CHAT_COMPLETIONS_MODEL,
  responses: OPENAI_RESPONSES_MODEL,
};

export function getTransformableCases(
  pair: TransformPair,
  filter?: string
): string[] {
  return getCaseNames(allTestCases).filter((caseName) => {
    // Only test param cases for chat-completions → anthropic for now
    if (
      caseName.endsWith("Param") &&
      (pair.source !== "chat-completions" || pair.target !== "anthropic")
    )
      return false;
    if (filter && !caseName.includes(filter)) return false;
    const sourceCase = getCaseForProvider(allTestCases, caseName, pair.source);
    const testCase = allTestCases[caseName];
    return sourceCase != null && !testCase?.expect;
  });
}
