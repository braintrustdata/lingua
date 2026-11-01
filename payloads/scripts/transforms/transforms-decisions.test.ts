import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { getResponsePath } from "./helpers";
import {
  DECISION_TRANSFORM_PAIRS,
  getDecisionTransformableCases,
  transformDecisionRequest,
  transformDecisionResponse,
} from "./decisions";

for (const pair of DECISION_TRANSFORM_PAIRS) {
  describe(`${pair.source} → ${pair.target}`, () => {
    for (const caseName of getDecisionTransformableCases(pair)) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      test.skipIf(!existsSync(responsePath))(caseName, () => {
        // The exact bytes sent upstream: decision models read the state's key order.
        expect(transformDecisionRequest(pair, caseName)).toMatchSnapshot(
          "request"
        );
        expect(
          transformDecisionResponse(
            pair,
            caseName,
            readFileSync(responsePath, "utf-8")
          )
        ).toMatchSnapshot("response");
      });
    }
  });
}
