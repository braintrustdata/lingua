import { describe, it, expect } from "vitest";
import { compareResponses, formatDiff, DiffEntry } from "./diff-utils";

describe("compareResponses", () => {
  describe("matching objects", () => {
    it("returns match for identical primitive values", () => {
      const result = compareResponses(42, 42);
      expect(result.match).toBe(true);
      expect(result.diffs).toEqual([]);
    });

    it("returns match for identical strings", () => {
      const result = compareResponses("hello", "hello");
      expect(result.match).toBe(true);
      expect(result.diffs).toEqual([]);
    });

    it("returns match for identical objects", () => {
      const obj = { id: "123", name: "test", nested: { value: 42 } };
      const result = compareResponses(obj, obj);
      expect(result.match).toBe(true);
      expect(result.diffs).toEqual([]);
    });

    it("returns match for identical arrays", () => {
      const arr = [1, 2, { a: "b" }];
      const result = compareResponses(arr, arr);
      expect(result.match).toBe(true);
      expect(result.diffs).toEqual([]);
    });
  });

  describe("different values", () => {
    it("detects different primitive values", () => {
      const result = compareResponses(42, 43);
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0]).toEqual({ path: "", expected: 42, actual: 43 });
    });

    it("detects different string values", () => {
      const result = compareResponses("hello", "world");
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
    });

    it("detects different nested values", () => {
      const expected = { nested: { value: 42 } };
      const actual = { nested: { value: 43 } };
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].path).toBe("nested.value");
    });
  });

  describe("missing fields", () => {
    it("detects missing field in actual", () => {
      const expected = { id: "123", name: "test" };
      const actual = { id: "123" };
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].path).toBe("name (missing)");
    });

    it("detects extra field in actual", () => {
      const expected = { id: "123" };
      const actual = { id: "123", extra: "value" };
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
      expect(result.diffs[0].path).toBe("extra");
      expect(result.diffs[0].expected).toBe(undefined);
      expect(result.diffs[0].actual).toBe("value");
    });
  });

  describe("type mismatches", () => {
    it("detects type mismatch between number and string", () => {
      const result = compareResponses(42, "42");
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
    });

    it("detects difference between object and array (treated as objects)", () => {
      // Arrays are objects in JS, so deepCompare compares their keys
      // { a: 1 } has key "a", [1] has key "0" - both report as missing from the other
      const result = compareResponses({ a: 1 }, [1]);
      expect(result.match).toBe(false);
      expect(result.diffs.length).toBeGreaterThanOrEqual(1);
    });

    it("detects type mismatch between null and value", () => {
      const result = compareResponses(null, "value");
      expect(result.match).toBe(false);
      expect(result.diffs).toHaveLength(1);
    });
  });

  describe("array handling", () => {
    it("detects different array lengths within objects", () => {
      // Note: Top-level arrays auto-ignore "length" for streaming support
      // So we test array length detection within an object
      const expected = { items: [1, 2, 3] };
      const actual = { items: [1, 2] };
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(false);
      expect(result.diffs.some((d) => d.path === "items.length")).toBe(true);
    });

    it("ignores length for top-level arrays (streaming support)", () => {
      // Top-level arrays auto-ignore length since streaming chunk counts vary
      const expected = [1, 2, 3];
      const actual = [1, 2];
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(true);
    });

    it("detects different array elements", () => {
      const result = compareResponses([1, 2, 3], [1, 5, 3]);
      expect(result.match).toBe(false);
      expect(result.diffs.some((d) => d.path === "1")).toBe(true);
    });

    it("handles nested arrays", () => {
      const expected = { items: [{ id: 1 }, { id: 2 }] };
      const actual = { items: [{ id: 1 }, { id: 3 }] };
      const result = compareResponses(expected, actual);
      expect(result.match).toBe(false);
      expect(result.diffs[0].path).toBe("items.1.id");
    });
  });

  describe("ignored fields", () => {
    it("ignores exact field match", () => {
      const expected = { id: "123", name: "test" };
      const actual = { id: "456", name: "test" };
      const result = compareResponses(expected, actual, ["id"]);
      expect(result.match).toBe(true);
    });

    it("ignores nested field with dot notation", () => {
      const expected = { data: { timestamp: 100, value: "a" } };
      const actual = { data: { timestamp: 200, value: "a" } };
      const result = compareResponses(expected, actual, ["data.timestamp"]);
      expect(result.match).toBe(true);
    });

    it("ignores field with wildcard in array", () => {
      const expected = {
        choices: [
          { index: 0, message: { content: "hello" } },
          { index: 1, message: { content: "world" } },
        ],
      };
      const actual = {
        choices: [
          { index: 0, message: { content: "different" } },
          { index: 1, message: { content: "values" } },
        ],
      };
      const result = compareResponses(expected, actual, [
        "choices.*.message.content",
      ]);
      expect(result.match).toBe(true);
    });

    it("still detects non-ignored differences with wildcards", () => {
      const expected = {
        choices: [
          { index: 0, message: { content: "hello" } },
          { index: 1, message: { content: "world" } },
        ],
      };
      const actual = {
        choices: [
          { index: 5, message: { content: "different" } },
          { index: 1, message: { content: "values" } },
        ],
      };
      const result = compareResponses(expected, actual, [
        "choices.*.message.content",
      ]);
      expect(result.match).toBe(false);
      expect(result.diffs[0].path).toBe("choices.0.index");
    });

    it("ignores multiple fields", () => {
      const expected = { id: "123", timestamp: 100, value: "a" };
      const actual = { id: "456", timestamp: 200, value: "a" };
      const result = compareResponses(expected, actual, ["id", "timestamp"]);
      expect(result.match).toBe(true);
    });
  });

  describe("array (streaming) response handling", () => {
    it("auto-prefixes ignore patterns with *. for array responses", () => {
      const expected = [
        { id: "1", content: "chunk1" },
        { id: "2", content: "chunk2" },
      ];
      const actual = [
        { id: "different1", content: "chunk1" },
        { id: "different2", content: "chunk2" },
      ];
      const result = compareResponses(expected, actual, ["id"]);
      expect(result.match).toBe(true);
    });

    it("ignores array length for streaming responses", () => {
      const expected = [{ id: "1" }, { id: "2" }];
      const actual = [{ id: "different1" }, { id: "different2" }, { id: "3" }];
      const result = compareResponses(expected, actual, ["id"]);
      expect(result.match).toBe(true);
    });
  });
});

describe("formatDiff", () => {
  it("formats primitive diff", () => {
    const diff: DiffEntry = { path: "value", expected: 42, actual: 43 };
    expect(formatDiff(diff)).toBe("value: 42 → 43");
  });

  it("formats string diff", () => {
    const diff: DiffEntry = {
      path: "name",
      expected: "hello",
      actual: "world",
    };
    expect(formatDiff(diff)).toBe("name: hello → world");
  });

  it("formats object diff", () => {
    const diff: DiffEntry = {
      path: "data",
      expected: { a: 1 },
      actual: { a: 2 },
    };
    expect(formatDiff(diff)).toBe('data: {"a":1} → {"a":2}');
  });

  it("formats array diff", () => {
    const diff: DiffEntry = {
      path: "items",
      expected: [1, 2],
      actual: [1, 2, 3],
    };
    expect(formatDiff(diff)).toBe("items: [1,2] → [1,2,3]");
  });

  it("formats nested path diff", () => {
    const diff: DiffEntry = {
      path: "choices.0.message.content",
      expected: "hello",
      actual: "world",
    };
    expect(formatDiff(diff)).toBe("choices.0.message.content: hello → world");
  });

  it("formats missing field diff", () => {
    const diff: DiffEntry = {
      path: "field (missing)",
      expected: "(exists)",
      actual: "(missing)",
    };
    expect(formatDiff(diff)).toBe("field (missing): (exists) → (missing)");
  });
});
