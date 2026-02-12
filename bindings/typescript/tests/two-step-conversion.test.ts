import { describe, test, expect } from "vitest";
import * as fs from "fs";
import * as path from "path";
import { importMessagesFromSpans } from "../src/index";

type ImportAssertionCase = {
  expectedMessageCount?: number;
  expectedRolesInOrder?: string[];
  mustContainText?: string[];
};

function getCaseNameFromFixturePath(filePath: string): string {
  return path.basename(filePath, ".spans.json");
}

function loadJsonFile<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(filePath, "utf-8")) as T;
}

function discoverImportCaseFixtures(): string[] {
  const fixturesDir = path.join(__dirname, "../../../payloads/import-cases");
  if (!fs.existsSync(fixturesDir)) {
    return [];
  }

  return fs
    .readdirSync(fixturesDir)
    .filter((name) => name.endsWith(".spans.json"))
    .map((name) => path.join(fixturesDir, name))
    .sort();
}

describe("Import from spans fixtures", () => {
  const caseFixturePaths = discoverImportCaseFixtures();

  if (caseFixturePaths.length === 0) {
    test("No import fixtures found", () => {
      expect(caseFixturePaths.length).toBeGreaterThan(0);
    });
    return;
  }

  for (const spansFixturePath of caseFixturePaths) {
    const caseName = getCaseNameFromFixturePath(spansFixturePath);
    const assertionsPath = spansFixturePath.replace(
      ".spans.json",
      ".assertions.json",
    );

    test(caseName, () => {
      const spans = loadJsonFile<unknown[]>(spansFixturePath);
      const assertions = loadJsonFile<ImportAssertionCase>(assertionsPath);

      const messages = importMessagesFromSpans(spans);
      const serializedMessages = JSON.stringify(messages);

      if (assertions.expectedMessageCount !== undefined) {
        expect(messages).toHaveLength(assertions.expectedMessageCount);
      }

      if (assertions.expectedRolesInOrder) {
        const roles = messages.map((message) =>
          message && typeof message === "object" && "role" in message
            ? String(message.role)
            : "",
        );
        expect(roles).toEqual(assertions.expectedRolesInOrder);
      }

      for (const requiredText of assertions.mustContainText ?? []) {
        expect(serializedMessages).toContain(requiredText);
      }
    });
  }
});
