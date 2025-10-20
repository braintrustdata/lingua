/**
 * Lingua TypeScript Bindings for Browser
 *
 * Universal message format for LLMs
 */

import initWasm, * as wasmModule from '../wasm-web/lingua.js';
import type { InitInput } from '../wasm-web/lingua.js';

import { ensureOnce, getWasm, setWasm } from './wasm-runtime';

/**
 * Initialize the Lingua WASM module for browser use.
 *
 * Must be called before using any conversion or validation functions.
 * Safe to call multiple times - initialization only happens once.
 *
 * @param module - Optional WASM module source. Can be:
 *   - **String URL**: `'https://cdn.example.com/lingua_bg.wasm'`
 *   - **URL object**: `new URL('./lingua_bg.wasm', import.meta.url)`
 *   - **Response**: `await fetch('/wasm/lingua_bg.wasm')`
 *   - **BufferSource**: ArrayBuffer or TypedArray
 *   - **WebAssembly.Module**: Pre-compiled WASM module
 *
 * @returns Promise that resolves when initialization is complete
 *
 * @example
 * // Load from CDN
 * await init('https://unpkg.com/@braintrust/lingua/wasm-web/lingua_bg.wasm');
 *
 * @example
 * // Load from bundled asset with Vite/Webpack
 * await init(new URL('./lingua_bg.wasm', import.meta.url));
 */
export async function init(module: InitInput): Promise<void> {
  await ensureOnce(async () => {
    await initWasm(module);
    const exports = wasmModule as unknown as typeof import('../wasm/lingua.js');
    setWasm(exports);
    return exports;
  });
}

export default init;
export type { InitInput };
export { ensureOnce, getWasm };

export * from './wasm';

// Re-export all generated types
export * from './generated/Message';
export * from './generated/AssistantContent';
export * from './generated/AssistantContentPart';
export * from './generated/GeneratedFileContentPart';
export * from './generated/ProviderMetadata';
export * from './generated/ProviderOptions';
export * from './generated/SourceContentPart';
export * from './generated/SourceType';
export * from './generated/TextContentPart';
export * from './generated/ToolCallArguments';
export * from './generated/ToolCallContentPart';
export * from './generated/ToolContentPart';
export * from './generated/ToolErrorContentPart';
export * from './generated/ToolResultContentPart';
export * from './generated/ToolResultResponsePart';
export * from './generated/UserContent';
export * from './generated/UserContentPart';

// Main type aliases for convenience
export type { Message } from './generated/Message';

// Version info
export { VERSION } from './version';
