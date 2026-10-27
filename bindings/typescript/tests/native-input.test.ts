import { readFileSync } from "node:fs";
import { beforeAll, describe, expect, test } from "vitest";
import * as browser from "../src/index.browser";
import * as nodeWasm from "@braintrust/lingua-wasm";
import * as browserWasm from "@braintrust/lingua-wasm/browser";
import { setWasm } from "../src/wasm-runtime";
import {
  importAndDeduplicateMessages,
  importMessagesFromSpans,
} from "../src/index";

beforeAll(async () => {
  await browser.init(
    readFileSync(
      new URL("../../lingua-wasm/web/lingua_bg.wasm", import.meta.url)
    )
  );
});

describe.each([
  {
    name: "browser import",
    importMessages: browser.importMessagesFromSpans,
    wasm: browserWasm,
  },
  {
    name: "browser import and deduplicate",
    importMessages: browser.importAndDeduplicateMessages,
    wasm: browserWasm,
  },
  { name: "import", importMessages: importMessagesFromSpans, wasm: nodeWasm },
  {
    name: "import and deduplicate",
    importMessages: importAndDeduplicateMessages,
    wasm: nodeWasm,
  },
])("$name native input", ({ importMessages, wasm }) => {
  beforeAll(() => setWasm(wasm));
  test("rejects unsupported objects without reading unrelated fields", () => {
    let reads = 0;
    const unsupported = {
      get unrelated() {
        reads++;
        return { nested: "not message data" };
      },
    };

    expect(
      importMessages([{ input: unsupported, output: [unsupported] }])
    ).toEqual([]);
    expect(reads).toBe(0);
  });

  test("preserves errors thrown by array element accessors", () => {
    const original = new Error("Cannot read array element");
    const input = Object.defineProperty([], "0", {
      enumerable: true,
      get() {
        throw original;
      },
    });
    let caught: unknown;
    try {
      importMessages([{ input }]);
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(Error);
    if (!(caught instanceof Error) || !("cause" in caught)) {
      throw new Error("Expected the array accessor error");
    }
    expect(caught.cause).toBe(original);
  });

  test("reads array accessors once across parser attempts and decoding", () => {
    let reads = 0;
    const input = Object.defineProperty([], "0", {
      enumerable: true,
      get() {
        reads++;
        if (reads > 1) {
          throw new Error("Array item was read twice");
        }
        return { role: "user", content: "Hello" };
      },
    });

    expect(importMessages([{ input }])).toEqual([
      { role: "user", content: "Hello" },
    ]);
    expect(reads).toBe(1);
  });

  test("reads consumed object accessors once across parser attempts", () => {
    let reads = 0;
    const input = {
      get contents() {
        reads++;
        if (reads > 1) {
          throw new Error("Object field was read twice");
        }
        return [{ role: "user", parts: [{ text: "Hello" }] }];
      },
      get unrelated() {
        throw new Error("Unrelated LLM input field was decoded");
      },
    };

    expect(importMessages([{ input }])).toEqual([
      { role: "user", content: "Hello" },
    ]);
    expect(reads).toBe(1);
  });

  test("reads proxied array elements once", () => {
    let reads = 0;
    const input = new Proxy([{ role: "user", content: "Hello" }], {
      get(target, key, receiver) {
        if (key === "0") {
          reads++;
          if (reads > 1) {
            throw new Error("Proxy array element was read twice");
          }
        }
        return Reflect.get(target, key, receiver);
      },
    });

    expect(importMessages([{ input }])).toEqual([
      { role: "user", content: "Hello" },
    ]);
    expect(reads).toBe(1);
  });

  test("rejects invalid enum values without throwing inspection errors", () => {
    expect(
      importMessages([{ input: { role: null, content: "Hello" } }])
    ).toEqual([]);
  });

  test("propagates errors from consumed provider fields", () => {
    const error = new Error("Cannot read provider input");
    const input = {
      get contents() {
        throw error;
      },
    };

    expect(() => importMessages([{ input }])).toThrow(
      expect.objectContaining({ cause: error })
    );
  });

  test("skips unused span fields while importing LLM messages", () => {
    const error = new Error("Unused span field was decoded");
    const span = {
      input: [{ role: "user", content: "Hello" }],
      get unrelated() {
        throw error;
      },
    };

    expect(importMessages([span])).toEqual([
      { role: "user", content: "Hello" },
    ]);
  });

  test("imports supported output beside an unsupported input", () => {
    let reads = 0;
    const messages = importMessages([
      {
        input: {
          get unrelated() {
            reads++;
            return "not message data";
          },
        },
        output: [{ role: "assistant", content: "Hello" }],
      },
    ]);

    expect(messages).toEqual([
      { role: "assistant", content: "Hello", id: null },
    ]);
    expect(reads).toBe(0);
  });

  test("imports supported output beside scalar-only input", () => {
    expect(
      importMessages([{ input: [1, null, [false]], output: "Hello" }])
    ).toEqual([{ role: "assistant", content: "Hello", id: null }]);
  });

  test("preserves metadata instructions when both payloads are unsupported", () => {
    expect(
      importMessages([
        {
          input: { unrelated: "data" },
          output: { unrelated: "data" },
          metadata: { instructions: "Be concise" },
        },
      ])
    ).toEqual([{ role: "system", content: "Be concise" }]);
  });

  test("propagates errors from inspecting the original JS object", () => {
    const error = new Error("Cannot inspect input");
    const input = new Proxy(
      {},
      {
        ownKeys: () => {
          throw error;
        },
      }
    );

    expect(() => importMessages([{ input }])).toThrow(
      expect.objectContaining({ cause: error })
    );
  });

  test("admits JS maps containing messages", () => {
    expect(
      importMessages([
        {
          input: new Map([
            ["role", "user"],
            ["content", "Hello"],
          ]),
        },
      ])
    ).toEqual([{ role: "user", content: "Hello" }]);
  });
});
