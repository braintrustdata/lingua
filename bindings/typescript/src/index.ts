/**
 * Lingua TypeScript Bindings (Node.js)
 *
 * Universal message format for LLMs
 */

import * as wasm from "@braintrust/lingua-wasm";

import { setWasm, getWasm, ensureOnce } from "./wasm-runtime";

setWasm(wasm);

export { ensureOnce, getWasm };

export * from "./wasm";

// Re-export all generated types
export * from "./generated/Message";
export * from "./generated/AssistantContent";
export * from "./generated/AssistantContentPart";
export * from "./generated/GeneratedFileContentPart";
export * from "./generated/ProviderMetadata";
export * from "./generated/ProviderOptions";
export * from "./generated/SourceContentPart";
export * from "./generated/SourceType";
export * from "./generated/TextContentPart";
export * from "./generated/ToolCallArguments";
export * from "./generated/ToolCallContentPart";
export * from "./generated/ToolContentPart";
export * from "./generated/ToolErrorContentPart";
export * from "./generated/ToolResultContentPart";
export * from "./generated/ToolResultResponsePart";
export * from "./generated/UserContent";
export * from "./generated/UserContentPart";

// Universal request types
export * from "./generated/UniversalRequest";
export * from "./generated/UniversalParams";
export * from "./generated/ProviderFormat";

// Configuration types
export * from "./generated/ReasoningConfig";
export * from "./generated/ReasoningEffort";
export * from "./generated/ReasoningCanonical";
export * from "./generated/SummaryMode";
export * from "./generated/ToolChoiceConfig";
export * from "./generated/ToolChoiceMode";
export * from "./generated/ResponseFormatConfig";
export * from "./generated/ResponseFormatType";
export * from "./generated/JsonSchemaConfig";

// Tool types
export * from "./generated/UniversalTool";
export * from "./generated/UniversalToolType";

// Main type aliases for convenience
export type { Message } from "./generated/Message";

// Version info
export { VERSION } from "./version";
