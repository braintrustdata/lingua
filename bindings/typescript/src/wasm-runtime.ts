type WasmModule = typeof import("../dist/wasm/nodejs/lingua.js");

let wasmInstance: WasmModule | null = null;
let initialization: Promise<void> | null = null;

export function getWasm(): WasmModule {
  if (!wasmInstance) {
    throw new Error("Lingua WASM not initialized");
  }
  return wasmInstance;
}

export function setWasm(module: WasmModule): void {
  wasmInstance = module;
}

export function ensureOnce(
  initializer: () => Promise<WasmModule> | WasmModule
): Promise<void> {
  if (!initialization) {
    initialization = Promise.resolve(initializer())
      .then((module) => {
        setWasm(module);
      })
      .catch((error) => {
        initialization = null;
        throw error;
      });
  }
  return initialization;
}

export function resetWasmForTests(): void {
  wasmInstance = null;
  initialization = null;
}
