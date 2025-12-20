/**
 * Lingua TypeScript Bindings for Browser (Bundler)
 *
 * Universal message format for LLMs
 *
 * This entry point is designed for bundlers like webpack/Next.js that can
 * handle WASM imports natively. The WASM module is automatically initialized.
 */

import * as wasm from "../wasm/bundler/lingua.js";

import { ensureOnce, getWasm, setWasm } from "./wasm-runtime";

// Auto-initialize like the Node.js build
setWasm(wasm);

/**
 * Initialize the Lingua WASM module.
 *
 * Note: When using a bundler (webpack, Next.js, Vite), the WASM module is
 * automatically initialized at import time. This function is provided for
 * API compatibility and resolves immediately.
 *
 * @returns Promise that resolves when initialization is complete
 */
export async function init(): Promise<void> {
  // Already initialized by bundler - this is a no-op for compatibility
  return Promise.resolve();
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
