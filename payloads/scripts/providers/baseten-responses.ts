import OpenAI from "openai";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
} from "../../cases";
import { BASETEN_BASE_URL, BASETEN_MODEL } from "../../cases/models";
import {
  executeOpenAIResponses,
  openaiResponsesExecutor,
} from "./openai-responses";

type BasetenResponsesRequest = OpenAI.Responses.ResponseCreateParams;
type BasetenResponsesResponse = OpenAI.Responses.Response;

export const basetenResponsesCases: Record<string, BasetenResponsesRequest> =
  {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }

  const explicitCase = getCaseForProvider(
    allTestCases,
    caseName,
    "baseten-responses"
  );
  if (explicitCase) {
    basetenResponsesCases[caseName] = explicitCase;
    return;
  }

  const responsesCase = getCaseForProvider(allTestCases, caseName, "responses");
  if (responsesCase) {
    basetenResponsesCases[caseName] = {
      ...responsesCase,
      model: BASETEN_MODEL,
    };
  }
});

const BASETEN_HOST = BASETEN_BASE_URL.replace(/\/v1\/?$/, "");

export function executeBasetenResponses(
  caseName: string,
  payload: BasetenResponsesRequest,
  options?: ExecuteOptions
): Promise<
  CaptureResult<BasetenResponsesRequest, BasetenResponsesResponse, unknown>
> {
  return executeOpenAIResponses(caseName, payload, {
    ...options,
    baseURL: BASETEN_HOST,
    apiKey: process.env.BASETEN_API_KEY,
  });
}

export const basetenResponsesExecutor: ProviderExecutor<
  BasetenResponsesRequest,
  BasetenResponsesResponse,
  unknown
> = {
  name: "baseten-responses",
  cases: basetenResponsesCases,
  execute: executeBasetenResponses,
  ignoredFields: openaiResponsesExecutor.ignoredFields,
};
