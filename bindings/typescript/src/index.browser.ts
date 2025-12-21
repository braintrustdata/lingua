/**
 * Lingua TypeScript Bindings (Browser)
 *
 * Universal message format for LLMs
 *
 * Call init(url) with the WASM URL before using.
 */

import initWasm, * as wasmModule from "@braintrust/lingua-wasm/browser";
import type { InitInput } from "@braintrust/lingua-wasm/browser";

import { ensureOnce, getWasm, setWasm } from "./wasm-runtime";

export async function init(module: InitInput): Promise<void> {
  return ensureOnce(async () => {
    await initWasm(module);
    setWasm(wasmModule as unknown as typeof wasmModule);
    return wasmModule;
  });
}

export default init;
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

// Main type aliases for convenience
export type { Message } from "./generated/Message";

// Version info
export { VERSION } from "./version";
