import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type SystemOneRequest,
} from "../../cases";
import { TYPESAFE_BASE_URL } from "../../cases/models";

export const typesafeCases: Record<string, SystemOneRequest> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "typesafe");
  if (caseData) {
    typesafeCases[caseName] = caseData;
  }
});

// TypeSafe publishes no TypeScript SDK, so call System One directly. `body` is
// sent verbatim because System One reads the state, including its key order, as text.
export async function callTypeSafe(
  body: string,
  options?: ExecuteOptions
): Promise<unknown> {
  const apiKey = options?.apiKey ?? process.env.TYPESAFE_API_KEY;
  if (!apiKey) {
    throw new Error("TYPESAFE_API_KEY environment variable is required");
  }
  const response = await fetch(
    `${options?.baseURL ?? TYPESAFE_BASE_URL}/systemone`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
        "TypeSafe-Danger-Zone-Images": "true",
      },
      body,
    }
  );
  if (!response.ok) {
    throw new Error(
      `TypeSafe System One error (${response.status}): ${await response.text()}`
    );
  }
  return response.json();
}

export async function executeTypeSafe(
  _caseName: string,
  payload: SystemOneRequest,
  options?: ExecuteOptions
): Promise<CaptureResult<SystemOneRequest, unknown>> {
  const result: CaptureResult<SystemOneRequest, unknown> = {
    request: payload,
  };
  try {
    result.response = await callTypeSafe(JSON.stringify(payload), options);
  } catch (error) {
    result.error = String(error);
  }
  return result;
}

export const typesafeExecutor: ProviderExecutor<SystemOneRequest, unknown> = {
  name: "typesafe",
  cases: typesafeCases,
  execute: executeTypeSafe,
  ignoredFields: ["model", "usage", "answers"],
};
