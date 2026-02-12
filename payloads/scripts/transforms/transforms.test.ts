import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  TRANSFORMS_DIR,
  TARGET_MODELS,
  transformAndValidateRequest,
  transformResponseData,
  loadAndValidateResponse,
  getTransformableCases,
  getResponsePath,
} from "./helpers";

const API_TEST_TIMEOUT = 30000;

const ERRORS_PATH = join(TRANSFORMS_DIR, "transform_errors.json");
const transformErrors: Record<string, Record<string, string>> = existsSync(
  ERRORS_PATH
)
  ? JSON.parse(readFileSync(ERRORS_PATH, "utf-8"))
  : {};

for (const pair of TRANSFORM_PAIRS) {
  describe(`${pair.source} → ${pair.target}`, () => {
    const cases = getTransformableCases(pair);

    for (const caseName of cases) {
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      if (!existsSync(responsePath)) {
        console.warn(
          `Missing capture: ${pair.source} → ${pair.target} / ${caseName}`
        );
      }

      test.skipIf(!existsSync(responsePath))(
        caseName,
        () => {
          const pairKey = `${pair.source}_to_${pair.target}`;

          try {
            const input = getCaseForProvider(
              allTestCases,
              caseName,
              pair.source
            );

            const request = transformAndValidateRequest(
              input,
              pair.wasmTarget,
              pair.target
            );

            if (
              typeof request === "object" &&
              request !== null &&
              "model" in request
            ) {
              const targetCase = getCaseForProvider(
                allTestCases,
                caseName,
                pair.target
              );
              request.model =
                targetCase &&
                typeof targetCase === "object" &&
                "model" in targetCase
                  ? targetCase.model
                  : TARGET_MODELS[pair.target];
            }

            expect(request).toMatchSnapshot("request");

            const response = loadAndValidateResponse(responsePath, pair.target);

            const output = transformResponseData(response, pair.wasmSource);
            expect(output).toMatchSnapshot("response");
          } catch (e) {
            const errorReason = transformErrors[pairKey]?.[caseName];
            if (errorReason) {
              return;
            }
            throw e;
          }
        },
        API_TEST_TIMEOUT
      );
    }
  });
}
