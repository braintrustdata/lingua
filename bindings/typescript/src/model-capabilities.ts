import { getWasm } from "./wasm-runtime";

export function supportsForcedToolChoice(model: string): boolean {
  return getWasm().supports_forced_tool_choice(model);
}
