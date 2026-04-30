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
export type * from "./types";

// Version info
export { VERSION } from "./version";
