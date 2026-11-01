import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type DecisionsRequest,
} from "../../cases";

const OPENAI_API_BASE = "https://api.openai.com";

export const openaiDecisionsCases: Record<string, DecisionsRequest> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "decisions");
  if (caseData) {
    openaiDecisionsCases[caseName] = caseData;
  }
});

// The OpenAI SDK has no Decisions API client, so call the endpoint directly.
export async function callOpenAIDecisions(
  body: string,
  options?: ExecuteOptions
): Promise<unknown> {
  const apiKey = options?.apiKey ?? process.env.OPENAI_API_KEY;
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY environment variable is required");
  }
  const response = await fetch(
    `${options?.baseURL ?? OPENAI_API_BASE}/v1/decisions`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body,
    }
  );
  if (!response.ok) {
    throw new Error(
      `OpenAI Decisions API error (${response.status}): ${await response.text()}`
    );
  }
  return response.json();
}

export async function executeOpenAIDecisions(
  _caseName: string,
  payload: DecisionsRequest,
  options?: ExecuteOptions
): Promise<CaptureResult<DecisionsRequest, unknown>> {
  const result: CaptureResult<DecisionsRequest, unknown> = {
    request: payload,
  };
  try {
    result.response = await callOpenAIDecisions(
      JSON.stringify(payload),
      options
    );
  } catch (error) {
    result.error = String(error);
  }
  return result;
}

export const openaiDecisionsExecutor: ProviderExecutor<
  DecisionsRequest,
  unknown
> = {
  name: "decisions",
  cases: openaiDecisionsCases,
  execute: executeOpenAIDecisions,
  ignoredFields: [
    "model",
    "usage",
    "answers.*.probability",
    "answers.*.choice",
    "answers.*.score",
    "answers.*.confidence",
    "answers.*.probabilities",
  ],
};
