// Export types and utilities
export * from "./types";
export * from "./utils";

// Export all case collections
export { simpleCases } from "./simple";
export { advancedCases } from "./advanced";

// Import and merge all collections for convenience
import { simpleCases } from "./simple";
import { advancedCases } from "./advanced";
import { mergeCollections } from "./utils";

// Combined collection of all test cases
export const allTestCases = mergeCollections(simpleCases, advancedCases);

// Legacy export for backward compatibility (can be removed later)
export const unifiedTestCases = allTestCases;
