// Export types and utilities
export * from "./types";
export * from "./utils";
export * from "./models";

// Export all case collections
export { simpleCases } from "./simple";
export { advancedCases } from "./advanced";
export { paramsCases } from "./params";
export { proxyCases } from "./proxy";

// Import and merge all collections for convenience
import { simpleCases } from "./simple";
import { advancedCases } from "./advanced";
import { paramsCases } from "./params";
import { proxyCases } from "./proxy";
import { mergeCollections, getCaseNames } from "./utils";

// Combined collection of all test cases
export const allTestCases = mergeCollections(
  simpleCases,
  advancedCases,
  paramsCases,
  proxyCases
);

// Map of collection names to their case names (for --cases flag)
export const caseCollections: Record<string, string[]> = {
  simple: getCaseNames(simpleCases),
  advanced: getCaseNames(advancedCases),
  params: getCaseNames(paramsCases),
  proxy: getCaseNames(proxyCases),
};

// Legacy export for backward compatibility (can be removed later)
export const unifiedTestCases = allTestCases;
