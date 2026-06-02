import { describe, expect, test } from "vitest";
import {
  ANTHROPIC_MODEL,
  OPENAI_CHAT_COMPLETIONS_MODEL,
} from "../../cases";
import { transformAndValidateRequest } from "./helpers";

describe("anthropic mid-conversation system messages", () => {
  test("rejects non-leading system messages for Anthropic export", () => {
    expect(() =>
      transformAndValidateRequest(
        {
          model: OPENAI_CHAT_COMPLETIONS_MODEL,
          max_completion_tokens: 300,
          messages: [
            {
              role: "system",
              content: "Use the initial policy.",
            },
            {
              role: "user",
              content: "What is the required answer?",
            },
            {
              role: "system",
              content: "Use the updated policy.",
            },
          ],
        },
        "Anthropic",
        "anthropic",
        ANTHROPIC_MODEL
      )
    ).toThrowErrorMatchingSnapshot();
  });
});
