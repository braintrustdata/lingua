import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  GoogleGenerateContentRequest,
} from "../../cases";

type GoogleProxyRequest = GoogleGenerateContentRequest & { model: string };
type GoogleProxyResponse = Record<string, unknown>;

// Google cases for proxy validation - extracted from unified cases.
// Skips cases with expectations (those are validated, not captured).
export const googleProxyCases: Record<string, GoogleProxyRequest> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "google");
  if (caseData) {
    // The proxy requires a model field in the body for routing
    googleProxyCases[caseName] = {
      ...caseData,
      model: "gemini-2.5-flash",
    };
  }
});

export async function executeGoogleProxy(
  _caseName: string,
  payload: GoogleProxyRequest,
  options?: ExecuteOptions
): Promise<
  CaptureResult<GoogleProxyRequest, GoogleProxyResponse, GoogleProxyResponse>
> {
  const { stream, baseURL, apiKey } = options ?? {};
  const result: CaptureResult<
    GoogleProxyRequest,
    GoogleProxyResponse,
    GoogleProxyResponse
  > = { request: payload };

  if (!baseURL) {
    result.error =
      "Google proxy executor requires a baseURL (proxy URL) to send requests to";
    return result;
  }

  // Build the request body: translate google executor format to native Gemini API format.
  // The cases use `config` (matching the @google/genai SDK), but the native API
  // uses `generationConfig` at the top level.
  const body: Record<string, unknown> = {
    model: payload.model,
    contents: payload.contents,
  };
  if (payload.systemInstruction) {
    body.systemInstruction = payload.systemInstruction;
  }
  if (payload.tools) {
    body.tools = payload.tools;
  }
  if (payload.config) {
    body.generationConfig = payload.config;
  }

  try {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (apiKey) {
      headers["Authorization"] = `Bearer ${apiKey}`;
    }

    if (stream !== true) {
      const response = await fetch(`${baseURL}/v1/generateContent`, {
        method: "POST",
        headers,
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        result.error = `HTTP ${response.status}: ${await response.text()}`;
        return result;
      }

      result.response =
        (await response.json()) as unknown as GoogleProxyResponse;
    }

    if (stream !== false) {
      const streamResponse = await fetch(
        `${baseURL}/v1/streamGenerateContent`,
        {
          method: "POST",
          headers,
          body: JSON.stringify(body),
        }
      );

      if (!streamResponse.ok) {
        result.error = `HTTP ${streamResponse.status}: ${await streamResponse.text()}`;
        return result;
      }

      const streamText = await streamResponse.text();
      const chunks: GoogleProxyResponse[] = [];
      for (const line of streamText.split("\n")) {
        if (line.startsWith("data: ") && !line.startsWith("data: [DONE]")) {
          try {
            chunks.push(
              JSON.parse(line.slice(6)) as unknown as GoogleProxyResponse
            );
          } catch {
            // skip non-JSON lines
          }
        }
      }
      result.streamingResponse = chunks;
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const googleProxyExecutor: ProviderExecutor<
  GoogleProxyRequest,
  GoogleProxyResponse,
  GoogleProxyResponse
> = {
  name: "google",
  cases: googleProxyCases,
  execute: executeGoogleProxy,
  ignoredFields: [
    "responseId",
    "modelVersion",
    "candidates.*.content.parts.*.text",
    "usageMetadata",
  ],
};
