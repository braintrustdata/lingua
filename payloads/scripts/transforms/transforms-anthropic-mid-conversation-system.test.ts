import { describe, expect, test } from "vitest";
import { transform_request } from "@braintrust/lingua-wasm";
import {
  allTestCases,
  getCaseForProvider,
  ANTHROPIC_MODEL,
  OPENAI_CHAT_COMPLETIONS_MODEL,
} from "../../cases";
import { transformAndValidateRequest } from "./helpers";

describe("anthropic mid-conversation system messages", () => {
  test.each([
    "anthropicConsecutiveSystemMessages",
    "anthropicSystemPromptPatchWithEffort",
  ])("preserves native %s", (caseName) => {
    const request = getCaseForProvider(allTestCases, caseName, "anthropic");
    expect(request).toBeDefined();
    expect(
      transformAndValidateRequest(request, "Anthropic", "anthropic")
    ).toEqual(request);
  });

  test.each(["OpenAI", "Responses", "Google"])(
    "rejects per-message effort conversion to %s",
    (target) => {
      const request = getCaseForProvider(
        allTestCases,
        "anthropicSystemPromptPatchWithEffort",
        "anthropic"
      );
      expect(() => transform_request(JSON.stringify(request), target)).toThrow(
        "per-message output_config"
      );
    }
  );

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
