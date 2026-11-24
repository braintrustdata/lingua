import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import {
  transform_decision_request,
  validate_decision_request,
} from "@braintrust/lingua-wasm/full";
import { decisionCases } from "../../cases/decisions";
import { getResponsePath } from "./helpers";
import {
  DECISION_TRANSFORM_PAIRS,
  getDecisionTransformableCases,
  transformDecisionRequest,
  transformDecisionResponse,
} from "./decisions";

// Same-format requests preserve unknown choice fields for the provider to handle.
test("passes through unknown fields on choice options", () => {
  const request = JSON.stringify(
    decisionCases.decisionUnknownChoiceFieldRequest.decisions
  );
  expect(
    transform_decision_request(request, "decisions", "decisions")
  ).toMatchObject({ passThrough: true, json: request });
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
