import { TestCaseCollection, TestCase, ProviderType } from "./types";

// Helper to get all case names from a collection
export function getCaseNames(collection: TestCaseCollection): string[] {
  return Object.keys(collection);
}

// Helper to get all provider types for a specific case
export function getProviderTypesForCase(
  collection: TestCaseCollection,
  caseName: string
): ProviderType[] {
  const testCase = collection[caseName];
  return testCase ? (Object.keys(testCase) as ProviderType[]) : [];
}

// Helper to get all provider types across all cases in a collection
export function getAllProviderTypes(
  collection: TestCaseCollection
): ProviderType[] {
  const providerTypes = new Set<ProviderType>();
  Object.values(collection).forEach((testCase) => {
    (Object.keys(testCase) as ProviderType[]).forEach((providerType) => {
      providerTypes.add(providerType);
    });
  });
  return Array.from(providerTypes).sort();
}

// Helper to get a specific case for a provider
export function getCaseForProvider<T extends ProviderType>(
  collection: TestCaseCollection,
  caseName: string,
  providerType: T
): TestCase[T] | undefined {
  const testCase = collection[caseName];
  return testCase?.[providerType];
}

// Helper to merge multiple test case collections
export function mergeCollections(
  ...collections: TestCaseCollection[]
): TestCaseCollection {
  return collections.reduce((merged, collection) => {
    return { ...merged, ...collection };
  }, {});
}
