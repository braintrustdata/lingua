import { afterAll, beforeAll, beforeEach, test } from "vitest";
import {
  createTransformTestServer,
  registerSkippedFixtureTest as buildSkippedFixtureLabel,
  type FixtureSkipReason,
  type TransformTestServer,
} from "./helpers";

export function useTransformTestServer(): () => TransformTestServer {
  let server: TransformTestServer | undefined;

  beforeAll(async () => {
    server = await createTransformTestServer();
  });

  beforeEach(() => {
    server?.reset();
  });

  afterAll(async () => {
    await server?.close();
  });

  return () => {
    if (!server) {
      throw new Error("Transform test server was not started");
    }
    return server;
  };
}

export function registerSkippedFixtureTest(
  pairLabel: string,
  caseName: string,
  reason: FixtureSkipReason
): void {
  test.skip(buildSkippedFixtureLabel(pairLabel, caseName, reason), () => {});
}
