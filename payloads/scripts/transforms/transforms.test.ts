import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  TRANSFORM_PAIRS,
  TRANSFORMS_DIR,
  transformAndValidateRequest,
  transformResponseData,
  loadAndValidateResponse,
  getTransformableCases,
  getResponsePath,
} from "./helpers";

const API_TEST_TIMEOUT = 30000;

// Load expected transformation errors (provider incompatibilities)
const ERRORS_PATH = join(TRANSFORMS_DIR, "transform_errors.json");
const transformErrors: Record<string, Record<string, string>> = existsSync(
  ERRORS_PATH
)
  ? JSON.parse(readFileSync(ERRORS_PATH, "utf-8"))
  : {};

function getTestKey(source: string, target: string, caseName: string): string {
  return `${source}_to_${target}_${caseName}`;
}

for (const pair of TRANSFORM_PAIRS) {
  describe(`${pair.source} → ${pair.target}`, () => {
    const cases = getTransformableCases(pair);

    for (const caseName of cases) {
      const testKey = getTestKey(pair.source, pair.target, caseName);
      const responsePath = getResponsePath(pair.source, pair.target, caseName);

      test(
        caseName,
        () => {
          const pairKey = `${pair.source}_to_${pair.target}`;

          try {
            // Fail if response file missing (should have been captured)
            if (!existsSync(responsePath)) {
              throw new Error(
                `Missing response file: ${responsePath}\n` +
                  `Run 'pnpm transforms:capture ${caseName}' to capture, ` +
                  `or add "${testKey}" to SKIPPED_TESTS if intentionally skipped.`
              );
            }
            const input = getCaseForProvider(
              allTestCases,
              caseName,
              pair.source
            );

            // 1. Transform request + validate against target schema
            const request = transformAndValidateRequest(
              input,
              pair.wasmTarget,
              pair.target
            );
            expect(request).toMatchSnapshot("request");

            // 2. Load response + validate against target schema
            const response = loadAndValidateResponse(responsePath, pair.target);

            // 3. Transform response back → snapshot
            const output = transformResponseData(response, pair.wasmSource);
            expect(output).toMatchSnapshot("response");
          } catch (e) {
            // Check if this is an expected error (known provider incompatibility)
            const errorReason = transformErrors[pairKey]?.[caseName];
            if (errorReason) {
              // Known error - pass the test
              return;
            }
            throw e; // Unknown error - fail
          }
        },
        API_TEST_TIMEOUT
      );
    }
  });
}
