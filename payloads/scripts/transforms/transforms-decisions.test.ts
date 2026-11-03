import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { validate_decision_request } from "@braintrust/lingua-wasm/full";
import { decisionCases } from "../../cases/decisions";
import { getResponsePath } from "./helpers";
import {
  DECISION_TRANSFORM_PAIRS,
  getDecisionTransformableCases,
  transformDecisionRequest,
  transformDecisionResponse,
} from "./decisions";

test("rejects unknown fields on choice options", () => {
  const caseName = "decisionUnknownChoiceFieldRequest";
  expect(() =>
    validate_decision_request(
      JSON.stringify(decisionCases[caseName].decisions),
      "decisions"
    )
  ).toThrow("descriptino");
  expect(() =>
    transformDecisionRequest(
      { source: "decisions", target: "typesafe" },
      caseName
    )
  ).toThrow("descriptino");
});

test("accepts arbitrary fields inside input_json data", () => {
  expect(() =>
    validate_decision_request(
      JSON.stringify({
        model: "gpt-6-luna",
        input: [
          {
            role: "user",
            content: [
              {
                type: "input_json",
                data: { department: "billing", details: { paid: true } },
              },
            ],
          },
        ],
        questions: [
          { type: "predicate", instructions: "Is this about billing?" },
        ],
      }),
      "decisions"
    )
  ).not.toThrow();
});

for (const pair of DECISION_TRANSFORM_PAIRS) {
  describe(`${pair.source} → ${pair.target}`, () => {
    for (const caseName of getDecisionTransformableCases(pair)) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      test(caseName, () => {
        // The exact bytes sent upstream: decision models read the state's key order.
        expect(transformDecisionRequest(pair, caseName)).toMatchSnapshot(
          "request"
        );
        if (existsSync(responsePath)) {
          expect(
            transformDecisionResponse(
              pair,
              caseName,
              readFileSync(responsePath, "utf-8")
            )
          ).toMatchSnapshot("response");
        }
      });
    }
  });
}
