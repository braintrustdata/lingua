// Export types and utilities
export * from "./types";
export * from "./utils";
export * from "./models";

// Export all case collections (snapshot-based cases only)
export { simpleCases } from "./simple";
export { advancedCases } from "./advanced";
export { paramsCases } from "./params";

// Import and merge all collections for convenience
import { simpleCases } from "./simple";
import { advancedCases } from "./advanced";
import { paramsCases } from "./params";
import { mergeCollections, getCaseNames } from "./utils";

// Combined collection of all snapshot-based test cases
export const allTestCases = mergeCollections(
  simpleCases,
  advancedCases,
  paramsCases
);

// Map of collection names to their case names (for --cases flag)
// Note: proxy cases are handled separately in the validation library
export const caseCollections: Record<string, string[]> = {
  simple: getCaseNames(simpleCases),
  advanced: getCaseNames(advancedCases),
  params: getCaseNames(paramsCases),
};

// Legacy export for backward compatibility (can be removed later)
export const unifiedTestCases = allTestCases;
