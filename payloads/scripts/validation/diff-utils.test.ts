import { describe, it, expect } from "vitest";
import {
  compareResponses,
  formatDiff,
  DiffEntry,
  stripGoogleSdkFields,
  normalizeGoogleRequestFields,
} from "./diff-utils";

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
      expect(result.diffs[0]).toEqual({
        path: "",
        expected: 42,
        actual: 43,
        severity: "major",
      });
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
    const diff: DiffEntry = {
      path: "value",
      expected: 42,
      actual: 43,
      severity: "major",
    };
    expect(formatDiff(diff)).toBe("value: 42 → 43");
  });

  it("formats string diff", () => {
    const diff: DiffEntry = {
      path: "name",
      expected: "hello",
      actual: "world",
      severity: "major",
    };
    expect(formatDiff(diff)).toBe("name: hello → world");
  });

  it("formats object diff", () => {
    const diff: DiffEntry = {
      path: "data",
      expected: { a: 1 },
      actual: { a: 2 },
      severity: "major",
    };
    expect(formatDiff(diff)).toBe('data: {"a":1} → {"a":2}');
  });

  it("formats array diff", () => {
    const diff: DiffEntry = {
      path: "items",
      expected: [1, 2],
      actual: [1, 2, 3],
      severity: "major",
    };
    expect(formatDiff(diff)).toBe("items: [1,2] → [1,2,3]");
  });

  it("formats nested path diff", () => {
    const diff: DiffEntry = {
      path: "choices.0.message.content",
      expected: "hello",
      actual: "world",
      severity: "major",
    };
    expect(formatDiff(diff)).toBe("choices.0.message.content: hello → world");
  });

  it("formats missing field diff", () => {
    const diff: DiffEntry = {
      path: "field (missing)",
      expected: "(exists)",
      actual: "(missing)",
      severity: "major",
    };
    expect(formatDiff(diff)).toBe("field (missing): (exists) → (missing)");
  });
});

describe("stripGoogleSdkFields", () => {
  describe("primitive values", () => {
    it("returns null unchanged", () => {
      expect(stripGoogleSdkFields(null)).toBe(null);
    });

    it("returns undefined unchanged", () => {
      expect(stripGoogleSdkFields(undefined)).toBe(undefined);
    });

    it("returns strings unchanged", () => {
      expect(stripGoogleSdkFields("hello")).toBe("hello");
    });

    it("returns numbers unchanged", () => {
      expect(stripGoogleSdkFields(42)).toBe(42);
    });

    it("returns booleans unchanged", () => {
      expect(stripGoogleSdkFields(true)).toBe(true);
    });
  });

  describe("object handling", () => {
    it("removes sdkHttpResponse from top-level object", () => {
      const input = {
        candidates: [{ content: "test" }],
        sdkHttpResponse: { statusCode: 200, headers: {} },
      };
      const result = stripGoogleSdkFields(input);
      expect(result).toEqual({
        candidates: [{ content: "test" }],
      });
      expect("sdkHttpResponse" in result).toBe(false);
    });

    it("preserves objects without sdkHttpResponse", () => {
      const input = {
        candidates: [{ content: "test" }],
        modelVersion: "gemini-2.5-flash",
      };
      const result = stripGoogleSdkFields(input);
      expect(result).toEqual(input);
    });

    it("only strips sdkHttpResponse at top level of objects (not deeply nested)", () => {
      // Note: In real Google SDK responses, sdkHttpResponse only appears at the
      // top level of response objects. The function is intentionally NOT recursive
      // for nested object properties - this matches the actual SDK behavior.
      const input = {
        outer: {
          inner: {
            sdkHttpResponse: { statusCode: 200 },
            data: "preserved",
          },
        },
      };
      const result = stripGoogleSdkFields(input);
      // sdkHttpResponse is preserved in nested objects (doesn't happen in practice)
      expect(result).toEqual({
        outer: {
          inner: {
            sdkHttpResponse: { statusCode: 200 },
            data: "preserved",
          },
        },
      });
    });

    it("does not mutate the original object", () => {
      const input = {
        candidates: [{ content: "test" }],
        sdkHttpResponse: { statusCode: 200 },
      };
      const original = JSON.parse(JSON.stringify(input));
      stripGoogleSdkFields(input);
      expect(input).toEqual(original);
    });
  });

  describe("array handling", () => {
    it("processes arrays recursively", () => {
      const input = [
        { sdkHttpResponse: { statusCode: 200 }, data: "chunk1" },
        { sdkHttpResponse: { statusCode: 200 }, data: "chunk2" },
      ];
      const result = stripGoogleSdkFields(input);
      expect(result).toEqual([{ data: "chunk1" }, { data: "chunk2" }]);
    });

    it("preserves arrays without sdkHttpResponse", () => {
      const input = [{ data: "chunk1" }, { data: "chunk2" }];
      const result = stripGoogleSdkFields(input);
      expect(result).toEqual(input);
    });

    it("strips sdkHttpResponse from each item in top-level arrays", () => {
      // This is the actual use case: streaming responses are arrays where
      // each chunk has sdkHttpResponse at its top level
      const input = [
        { sdkHttpResponse: { statusCode: 200 }, data: "a" },
        { sdkHttpResponse: { statusCode: 200 }, data: "b" },
      ];
      const result = stripGoogleSdkFields(input);
      expect(result).toEqual([{ data: "a" }, { data: "b" }]);
    });
  });

  describe("realistic Google response", () => {
    it("strips sdkHttpResponse from real Google GenerateContentResponse structure", () => {
      const googleResponse = {
        sdkHttpResponse: {
          statusCode: 200,
          headers: {
            "content-type": "application/json",
          },
        },
        candidates: [
          {
            content: {
              parts: [{ text: "The capital of France is Paris." }],
              role: "model",
            },
            finishReason: "STOP",
            index: 0,
          },
        ],
        modelVersion: "gemini-2.5-flash",
        responseId: "abc123",
        usageMetadata: {
          promptTokenCount: 8,
          candidatesTokenCount: 8,
          totalTokenCount: 16,
        },
      };

      const result = stripGoogleSdkFields(googleResponse);

      expect(result).toEqual({
        candidates: [
          {
            content: {
              parts: [{ text: "The capital of France is Paris." }],
              role: "model",
            },
            finishReason: "STOP",
            index: 0,
          },
        ],
        modelVersion: "gemini-2.5-flash",
        responseId: "abc123",
        usageMetadata: {
          promptTokenCount: 8,
          candidatesTokenCount: 8,
          totalTokenCount: 16,
        },
      });
      expect("sdkHttpResponse" in result).toBe(false);
    });

    it("handles streaming response array", () => {
      const streamingResponse = [
        {
          sdkHttpResponse: { statusCode: 200 },
          candidates: [{ content: { parts: [{ text: "Hello" }] } }],
        },
        {
          sdkHttpResponse: { statusCode: 200 },
          candidates: [{ content: { parts: [{ text: " world" }] } }],
        },
      ];

      const result = stripGoogleSdkFields(streamingResponse);

      expect(result).toEqual([
        { candidates: [{ content: { parts: [{ text: "Hello" }] } }] },
        { candidates: [{ content: { parts: [{ text: " world" }] } }] },
      ]);
    });
  });
});

describe("normalizeGoogleRequestFields", () => {
  it("renames config to generationConfig", () => {
    const input = { contents: [], config: { maxOutputTokens: 100 } };
    const result = normalizeGoogleRequestFields(input);
    expect(result).toEqual({
      contents: [],
      generationConfig: { maxOutputTokens: 100 },
    });
    expect("config" in result).toBe(false);
  });

  it("preserves existing generationConfig", () => {
    const input = { contents: [], generationConfig: { maxOutputTokens: 100 } };
    const result = normalizeGoogleRequestFields(input);
    expect(result).toEqual(input);
  });

  it("does not overwrite existing generationConfig with config", () => {
    const input = {
      contents: [],
      config: { maxOutputTokens: 50 },
      generationConfig: { maxOutputTokens: 100 },
    };
    const result = normalizeGoogleRequestFields(input);
    expect(result.generationConfig).toEqual({ maxOutputTokens: 100 });
    expect("config" in result).toBe(true); // config preserved when generationConfig exists
  });

  it("handles objects without config", () => {
    const input = { contents: [{ text: "hello" }] };
    const result = normalizeGoogleRequestFields(input);
    expect(result).toEqual(input);
  });

  it("returns primitives unchanged", () => {
    expect(normalizeGoogleRequestFields(null)).toBe(null);
    expect(normalizeGoogleRequestFields("string")).toBe("string");
    expect(normalizeGoogleRequestFields(42)).toBe(42);
  });

  it("does not mutate original object", () => {
    const input = { contents: [], config: { maxOutputTokens: 100 } };
    const original = JSON.parse(JSON.stringify(input));
    normalizeGoogleRequestFields(input);
    expect(input).toEqual(original);
  });
});
