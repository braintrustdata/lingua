/**
 * Lingua TypeScript Bindings
 *
 * Universal message format for LLMs
 */

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

// Main type aliases for convenience
export type { Message } from "./generated/Message";

// WASM conversion functions
export * from "./wasm";

// Version info
export const VERSION = "0.1.0";
