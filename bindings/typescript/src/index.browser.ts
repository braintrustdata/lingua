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
export type * from "./types";

// Version info
export { VERSION } from "./version";
