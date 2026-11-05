import { describe, expect, test } from "vitest";
import { transform_request } from "@braintrust/lingua-wasm";
import { paramsCases } from "../../cases/params";

const cases = [
  ["thinkingLevelLowercaseLowParam", "low"],
  ["thinkingLevelLowercaseMediumParam", "medium"],
  ["thinkingLevelLowercaseHighParam", "high"],
  ["thinkingLevelLowercaseMinimalParam", "minimal"],
] as const;

function nativeRequest(caseName: string) {
  const input = paramsCases[caseName].google;
  if (!input) {
    throw new Error(`Missing Google thinking level case: ${caseName}`);
  }
  // Native Gemini selects its model in the URL, outside the request body.
  const { model: _model, ...body } = input;
  return body;
}

describe("Google thinking level casing", () => {
  test.each(cases)("%s preserves native lowercase input", (caseName, level) => {
    const input = nativeRequest(caseName);
    expect(input?.generationConfig?.thinkingConfig?.thinkingLevel).toBe(level);
    const result = transform_request(JSON.stringify(input), "google");
    expect(result.passThrough).toBe(true);
    expect(result.data).toEqual(input);
  });

  test.each(cases)("%s keeps uppercase input supported", (caseName, level) => {
    const input = {
      ...nativeRequest(caseName),
      generationConfig: {
        thinkingConfig: { thinkingLevel: level.toUpperCase() },
      },
    };
    const result = transform_request(JSON.stringify(input), "google");
    expect(result.passThrough).toBe(true);
    expect(result.data).toEqual(input);
  });

  test.each(cases)(
    "%s maps to the same reasoning effort",
    (caseName, level) => {
      const lowercase = nativeRequest(caseName);
      const uppercase = {
        ...lowercase,
        generationConfig: {
          thinkingConfig: { thinkingLevel: level.toUpperCase() },
        },
      };
      const lowerResult = transform_request(
        JSON.stringify(lowercase),
        "chat-completions",
        "gpt-5-nano"
      );
      const upperResult = transform_request(
        JSON.stringify(uppercase),
        "chat-completions",
        "gpt-5-nano"
      );
      expect(lowerResult).toEqual(upperResult);
      expect(lowerResult.data.reasoning_effort).toBe(level);
    }
  );

  test("rejects unrecognized thinking levels", () => {
    const input = {
      ...nativeRequest("thinkingLevelLowercaseLowParam"),
      generationConfig: { thinkingConfig: { thinkingLevel: "unknown" } },
    };
    expect(() => transform_request(JSON.stringify(input), "google")).toThrow(
      /Unable to detect .*source format/
    );
  });
});
