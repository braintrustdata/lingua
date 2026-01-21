// JSON comparison utilities with field ignore support

export interface DiffEntry {
  path: string;
  expected: unknown;
  actual: unknown;
}

export interface DiffResult {
  match: boolean;
  diffs: DiffEntry[];
}

/**
 * Check if a path matches an ignore pattern.
 * Supports:
 * - Exact matches: "id", "usage"
 * - Dot notation: "choices.0.message.content"
 * - Wildcards: "choices.*.message.content" (matches any array index)
 */
function matchesIgnorePattern(path: string, pattern: string): boolean {
  // Convert pattern to regex
  // - Escape dots
  // - Replace * with regex pattern for array indices or object keys
  const regexPattern = pattern
    .split(".")
    .map((part) => {
      if (part === "*") {
        return "[^.]+"; // Match any segment (array index or key)
      }
      return part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); // Escape special chars
    })
    .join("\\.");

  const regex = new RegExp(`^${regexPattern}$`);
  return regex.test(path);
}

/**
 * Check if a path should be ignored based on the ignore patterns.
 */
function shouldIgnore(path: string, ignoredFields: string[]): boolean {
  return ignoredFields.some((pattern) => matchesIgnorePattern(path, pattern));
}

/**
 * Deep compare two values, tracking the path and collecting differences.
 * When a field is ignored, we still verify the key exists - just ignore the value.
 */
function deepCompare(
  expected: unknown,
  actual: unknown,
  path: string,
  ignoredFields: string[],
  diffs: DiffEntry[]
): void {
  const isIgnored = shouldIgnore(path, ignoredFields);

  // Handle null/undefined - even for ignored fields, check presence
  if (expected === null || expected === undefined) {
    if (actual !== null && actual !== undefined) {
      // Only report if NOT ignored (ignored means we don't care about value differences)
      if (!isIgnored) {
        diffs.push({ path, expected, actual });
      }
    }
    return;
  }
  if (actual === null || actual === undefined) {
    // Key exists in expected but missing in actual
    // Skip if this field is ignored (we don't care about its presence)
    if (!isIgnored) {
      diffs.push({
        path: `${path} (missing)`,
        expected: "(exists)",
        actual: "(missing)",
      });
    }
    return;
  }

  // If this path is ignored, we've verified both have the key - stop here
  if (isIgnored) {
    return;
  }

  // Handle different types
  const expectedType = typeof expected;
  const actualType = typeof actual;

  if (expectedType !== actualType) {
    diffs.push({ path, expected, actual });
    return;
  }

  // Handle arrays
  if (Array.isArray(expected)) {
    if (!Array.isArray(actual)) {
      diffs.push({ path, expected, actual });
      return;
    }

    // Compare array lengths
    const lengthPath = path ? `${path}.length` : "length";
    const ignoringLength = shouldIgnore(lengthPath, ignoredFields);
    if (expected.length !== actual.length && !ignoringLength) {
      diffs.push({
        path: lengthPath,
        expected: expected.length,
        actual: actual.length,
      });
    }

    // Compare each element
    // When ignoring length (e.g., streaming), only compare elements that exist in both
    const compareLen = ignoringLength
      ? Math.min(expected.length, actual.length)
      : Math.max(expected.length, actual.length);
    for (let i = 0; i < compareLen; i++) {
      const elemPath = path ? `${path}.${i}` : String(i);
      deepCompare(expected[i], actual[i], elemPath, ignoredFields, diffs);
    }
    return;
  }

  // Handle objects
  if (expectedType === "object") {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Type narrowing already confirmed this is an object
    const expectedObj = expected as Record<string, unknown>;
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- Type narrowing already confirmed this is an object
    const actualObj = actual as Record<string, unknown>;

    // Get all keys from both objects
    const allKeys = new Set([
      ...Object.keys(expectedObj),
      ...Object.keys(actualObj),
    ]);

    for (const key of allKeys) {
      const keyPath = path ? `${path}.${key}` : key;
      deepCompare(
        expectedObj[key],
        actualObj[key],
        keyPath,
        ignoredFields,
        diffs
      );
    }
    return;
  }

  // Handle primitives
  if (expected !== actual) {
    diffs.push({ path, expected, actual });
  }
}

/**
 * Compare two JSON objects and return differences, ignoring specified fields.
 *
 * @param expected - The expected/baseline response
 * @param actual - The actual response from the proxy
 * @param ignoredFields - Fields to ignore (supports wildcards like "choices.*.message.content")
 * @returns DiffResult with match status and list of differences
 */
export function compareResponses(
  expected: unknown,
  actual: unknown,
  ignoredFields: string[] = []
): DiffResult {
  // Transform patterns for array (streaming) responses
  // "id" → "*.id" to match "0.id", "1.id", etc.
  // "choices.*.delta.content" → "*.choices.*.delta.content"
  // Also ignore "length" since streaming chunk count is variable
  const effectiveIgnoredFields = Array.isArray(expected)
    ? [...ignoredFields.map((pattern) => `*.${pattern}`), "length"]
    : ignoredFields;

  const diffs: DiffEntry[] = [];
  deepCompare(expected, actual, "", effectiveIgnoredFields, diffs);

  return {
    match: diffs.length === 0,
    diffs,
  };
}

/**
 * Format a diff entry for display.
 */
export function formatDiff(diff: DiffEntry): string {
  const expectedStr =
    typeof diff.expected === "object"
      ? JSON.stringify(diff.expected)
      : String(diff.expected);
  const actualStr =
    typeof diff.actual === "object"
      ? JSON.stringify(diff.actual)
      : String(diff.actual);

  return `${diff.path}: ${expectedStr} → ${actualStr}`;
}
