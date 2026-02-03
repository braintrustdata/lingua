export * from "./types";
export { proxyCases } from "./cases";

import { proxyCases } from "./cases";

export function getProxyCaseNames(): string[] {
  return Object.keys(proxyCases);
}
