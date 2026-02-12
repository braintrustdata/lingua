import { describe, test, expect } from "vitest";
import { existsSync } from "fs";
import { join } from "path";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  type ProviderType,
} from "../cases";
import {
  TRANSFORM_PAIRS,
  getTransformableCases,
  getResponsePath,
} from "./transforms/helpers";

const SNAPSHOTS_DIR = join(__dirname, "..", "snapshots");

const SNAPSHOT_PROVIDERS: ProviderType[] = [
  "chat-completions",
  "responses",
  "anthropic",
  "google",
  "bedrock",
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
      test(`transform capture exists: ${pair.source} → ${pair.target} / ${caseName}`, () => {
        const responsePath = getResponsePath(
          pair.source,
          pair.target,
          caseName
        );
        expect(
          existsSync(responsePath),
          `Missing transform capture: ${responsePath}. Run 'pnpm capture --filter ${caseName}'`
        ).toBe(true);
      });
    }
  }
});
