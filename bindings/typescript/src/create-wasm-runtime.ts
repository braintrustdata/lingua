export function createWasmRuntime<T>() {
  let wasmInstance: T | null = null;
  let initialization: Promise<void> | null = null;

  function getWasm(): T {
    if (!wasmInstance) {
      throw new Error("Lingua WASM not initialized");
    }
    return wasmInstance;
  }

  function setWasm(module: T): void {
    wasmInstance = module;
  }

  function ensureOnce(initializer: () => Promise<T> | T): Promise<void> {
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

  function resetWasmForTests(): void {
    wasmInstance = null;
    initialization = null;
  }

  return { getWasm, setWasm, ensureOnce, resetWasmForTests };
}
