/**
 * Lingua TypeScript Bindings for Browser (Web)
 *
 * Universal message format for LLMs
 *
 * This entry point uses the web target which fetches WASM from a URL,
 * making it compatible with Next.js SSR and other server-side scenarios.
 */

import initWasm, * as wasmModule from "../wasm/web/lingua.js";
import type { InitInput } from "../wasm/web/lingua.js";

import { ensureOnce, getWasm, setWasm } from "./wasm-runtime";

/**
 * Initialize the Lingua WASM module.
 *
 * Must be called before using any Lingua functions. Safe to call multiple
 * times - initialization only happens once.
 *
 * @param module - Optional WASM module source. Can be:
 *   - **String URL**: `'/wasm/lingua.wasm'`
 *   - **URL object**: `new URL('./lingua_bg.wasm', import.meta.url)`
 *   - **Response**: `await fetch('/wasm/lingua_bg.wasm')`
 *   - **BufferSource**: ArrayBuffer or TypedArray
 *   - **WebAssembly.Module**: Pre-compiled WASM module
 *   - **undefined**: Auto-detect using import.meta.url (may not work in all bundlers)
 *
 * @returns Promise that resolves when initialization is complete
 */
export async function init(module?: InitInput): Promise<void> {
  return ensureOnce(async () => {
    await initWasm(module);
    const exports = wasmModule as unknown as typeof import("../wasm/web/lingua.js");
    setWasm(exports);
    return exports;
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
