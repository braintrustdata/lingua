import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  type ProviderType,
} from "../cases";
import {
  TRANSFORM_PAIRS,
  TRANSFORMS_DIR,
  getTransformableCases,
  getResponsePath,
} from "./transforms/helpers";

const SNAPSHOTS_DIR = join(__dirname, "..", "snapshots");
const ERRORS_PATH = join(TRANSFORMS_DIR, "transform_errors.json");
const transformErrors: Record<string, Record<string, string>> = existsSync(
  ERRORS_PATH
)
  ? JSON.parse(readFileSync(ERRORS_PATH, "utf-8"))
  : {};

const SNAPSHOT_PROVIDERS: ProviderType[] = [
  "chat-completions",
  "responses",
  "anthropic",
  "google",
  "bedrock",
  "baseten",
];

describe("test data sync", () => {
  const caseNames = getCaseNames(allTestCases);

  for (const caseName of caseNames) {
    const testCase = allTestCases[caseName];
    if (testCase?.expect) continue;

    for (const provider of SNAPSHOT_PROVIDERS) {
      const caseData = getCaseForProvider(allTestCases, caseName, provider);
      if (caseData == null) continue;

      test(`snapshot exists: ${provider}/${caseName}`, () => {
        const snapshotDir = join(SNAPSHOTS_DIR, caseName, provider);
        expect(
          existsSync(snapshotDir),
          `Missing snapshot directory: ${snapshotDir}. Run 'pnpm capture --filter ${caseName}'`
        ).toBe(true);
      });
    }
  }

  for (const pair of TRANSFORM_PAIRS) {
    const cases = getTransformableCases(pair);

    for (const caseName of cases) {
      const pairKey = `${pair.source}_to_${pair.target}`;
      const transformError = transformErrors[pairKey]?.[caseName];

      test.skipIf(transformError)(
        `transform capture exists: ${pair.source} → ${pair.target} / ${caseName}`,
        () => {
          const responsePath = getResponsePath(
            pair.source,
            pair.target,
            caseName
          );
          expect(
            existsSync(responsePath),
            `Missing transform capture: ${responsePath}. Run 'pnpm capture --filter ${caseName}'`
          ).toBe(true);
        }
      );
    }
  }
});
