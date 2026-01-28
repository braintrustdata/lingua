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
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Object.keys() returns string[], but we know these are ProviderType keys
  return testCase ? (Object.keys(testCase) as ProviderType[]) : [];
}

// Helper to get all provider types across all cases in a collection
export function getAllProviderTypes(
  collection: TestCaseCollection
): ProviderType[] {
  const providerTypes = new Set<ProviderType>();
  Object.values(collection).forEach((testCase) => {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Object.keys() returns string[], but we know these are ProviderType keys
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

// Helper to get the full test case (including expect field)
export function getFullTestCase(
  collection: TestCaseCollection,
  caseName: string
): TestCase | undefined {
  return collection[caseName];
}

// Helper to check if a test case has expectations (should skip capture)
export function hasExpectation(
  collection: TestCaseCollection,
  caseName: string
): boolean {
  const testCase = collection[caseName];
  return testCase?.expect !== undefined;
}
