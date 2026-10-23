import { describe, expect, test } from "vitest";
import {
  importAndDeduplicateMessages,
  importMessagesFromSpans,
} from "../src/index";

describe.each([
  { name: "import", importMessages: importMessagesFromSpans },
  {
    name: "import and deduplicate",
    importMessages: importAndDeduplicateMessages,
  },
])("$name native input", ({ importMessages }) => {
  test("rejects unsupported payloads without reading their contents", () => {
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
