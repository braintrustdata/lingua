import { createWasmRuntime } from "./create-wasm-runtime";

export const { getWasm, setWasm, ensureOnce, resetWasmForTests } =
  createWasmRuntime<typeof import("@braintrust/lingua-wasm")>();
