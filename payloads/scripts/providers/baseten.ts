import OpenAI from "openai";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
} from "../../cases";
import { BASETEN_BASE_URL, BASETEN_MODEL } from "../../cases/models";
import { executeOpenAI, openaiExecutor } from "./openai";

// Baseten serves OSS models behind an OpenAI-compatible chat-completions API, so the
// snapshots reuse the OpenAI request/response/stream types.
type BasetenRequest = OpenAI.Chat.Completions.ChatCompletionCreateParams;
type BasetenResponse = OpenAI.Chat.Completions.ChatCompletion;
type BasetenStreamChunk = OpenAI.Chat.Completions.ChatCompletionChunk;

export const basetenCases: Record<string, BasetenRequest> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "baseten");
  if (caseData) {
    basetenCases[caseName] = caseData;
    return;
  }

  const chatCompletionsCase = getCaseForProvider(
    allTestCases,
    caseName,
    "chat-completions"
  );
  if (chatCompletionsCase) {
    basetenCases[caseName] = {
      ...chatCompletionsCase,
      model: BASETEN_MODEL,
    };
  }
});

// executeOpenAI appends `/v1` to the baseURL it is given; BASETEN_BASE_URL already
// includes it, so strip it back off to the host.
const BASETEN_HOST = BASETEN_BASE_URL.replace(/\/v1\/?$/, "");

// Reuse the OpenAI executor pointed at Baseten's OpenAI-compatible endpoint.
export function executeBaseten(
  caseName: string,
  payload: BasetenRequest,
  options?: ExecuteOptions
): Promise<CaptureResult<BasetenRequest, BasetenResponse, BasetenStreamChunk>> {
  return executeOpenAI(caseName, payload, {
    ...options,
    baseURL: BASETEN_HOST,
    apiKey: process.env.BASETEN_API_KEY,
  });
}

export const basetenExecutor: ProviderExecutor<
  BasetenRequest,
  BasetenResponse,
  BasetenStreamChunk
> = {
  name: "baseten",
  cases: basetenCases,
  execute: executeBaseten,
  ignoredFields: openaiExecutor.ignoredFields,
};
