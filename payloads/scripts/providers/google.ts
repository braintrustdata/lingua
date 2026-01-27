import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  GoogleGenerateContentRequest,
  GOOGLE_MODEL,
} from "../../cases";

const GOOGLE_API_BASE = "https://generativelanguage.googleapis.com/v1beta";

// Response type matching Google's GenerateContent response
// Using a minimal type since we just need to capture the raw response
type GenerateContentResponse = {
  candidates?: Array<{
    content?: {
      role?: string;
      parts?: Array<{
        text?: string;
        functionCall?: { name: string; args?: Record<string, unknown> };
        [key: string]: unknown;
      }>;
    };
    [key: string]: unknown;
  }>;
  [key: string]: unknown;
};

// Content type for building follow-up messages
type Content = {
  role: string;
  parts: Array<Record<string, unknown>>;
};

// Helper to parse JSON response with proper typing
async function parseJsonResponse(
  response: Response
): Promise<GenerateContentResponse> {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- response.json() returns unknown
  return (await response.json()) as GenerateContentResponse;
}

// Helper to convert content to our local Content type
function toContent(content: { role?: string; parts?: unknown[] }): Content {
  const parts: Array<Record<string, unknown>> = [];
  for (const part of content.parts ?? []) {
    if (part && typeof part === "object") {
      // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- validated above
      parts.push(part as Record<string, unknown>);
    }
  }
  return {
    role: content.role ?? "user",
    parts,
  };
}

// Google cases - extracted from unified cases
// Skips cases with expectations (those are validated, not captured)
export const googleCases: Record<string, GoogleGenerateContentRequest> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  // Skip cases with expectations - they use validate.ts, not capture.ts
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "google");
  if (caseData) {
    googleCases[caseName] = caseData;
  }
});

/**
 * Make a request to the Google Gemini API
 */
async function googleRequest(
  model: string,
  payload: GoogleGenerateContentRequest,
  apiKey: string,
  stream: boolean
): Promise<Response> {
  const endpoint = stream
    ? `${GOOGLE_API_BASE}/models/${model}:streamGenerateContent?alt=sse`
    : `${GOOGLE_API_BASE}/models/${model}:generateContent`;

  // Build the request body matching the raw Google API structure
  const body: Record<string, unknown> = {
    contents: payload.contents,
  };

  if (payload.generationConfig) {
    body.generationConfig = payload.generationConfig;
  }

  if (payload.tools) {
    body.tools = payload.tools;
  }

  if (payload.toolConfig) {
    body.toolConfig = payload.toolConfig;
  }

  if (payload.systemInstruction) {
    body.systemInstruction = payload.systemInstruction;
  }

  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-goog-api-key": apiKey,
    },
    body: JSON.stringify(body),
  });

  return response;
}

/**
 * Parse SSE stream into array of response chunks
 */
async function parseSSEStream(
  response: Response
): Promise<GenerateContentResponse[]> {
  const chunks: GenerateContentResponse[] = [];
  const text = await response.text();

  // SSE format: "data: {...}\n\n"
  const lines = text.split("\n");
  for (const line of lines) {
    if (line.startsWith("data: ")) {
      const jsonStr = line.slice(6); // Remove "data: " prefix
      if (jsonStr.trim()) {
        try {
          chunks.push(JSON.parse(jsonStr));
        } catch {
          // Skip malformed JSON
        }
      }
    }
  }

  return chunks;
}

type ParallelGoogleResult =
  | {
      type: "response";
      data: GenerateContentResponse;
    }
  | {
      type: "streamingResponse";
      data: Array<GenerateContentResponse>;
    };

export async function executeGoogle(
  caseName: string,
  payload: GoogleGenerateContentRequest,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    GoogleGenerateContentRequest,
    GenerateContentResponse,
    GenerateContentResponse
  >
> {
  const { stream, apiKey: optApiKey } = options ?? {};
  const apiKey = optApiKey ?? process.env.GOOGLE_API_KEY;

  if (!apiKey) {
    throw new Error("GOOGLE_API_KEY is required");
  }

  const model = payload.model ?? GOOGLE_MODEL;
  const result: CaptureResult<
    GoogleGenerateContentRequest,
    GenerateContentResponse,
    GenerateContentResponse
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelGoogleResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        (async () => {
          const response = await googleRequest(model, payload, apiKey, false);
          if (!response.ok) {
            const text = await response.text();
            throw new Error(`ApiError: ${text}`);
          }
          const data = await parseJsonResponse(response);
          return { type: "response" as const, data };
        })()
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const response = await googleRequest(model, payload, apiKey, true);
          if (!response.ok) {
            const text = await response.text();
            throw new Error(`ApiError: ${text}`);
          }
          const chunks = await parseSSEStream(response);
          return { type: "streamingResponse" as const, data: chunks };
        })()
      );
    }

    // Execute initial calls in parallel
    const initialResults = await Promise.all(promises);

    // Process results
    for (const result_ of initialResults) {
      if (result_.type === "response") {
        result.response = result_.data;
      } else if (result_.type === "streamingResponse") {
        result.streamingResponse = result_.data;
      }
    }

    // Create follow-up conversation if we have a non-streaming response
    if (result.response) {
      const assistantContent = result.response.candidates?.[0]?.content;

      if (assistantContent) {
        // Build follow-up messages
        const followUpContents: Content[] = [
          ...payload.contents.map(toContent),
          toContent(assistantContent),
        ];

        // Check if the assistant message contains function calls
        const assistantParts = assistantContent.parts || [];
        let hasToolCalls = false;

        for (const part of assistantParts) {
          if ("functionCall" in part && part.functionCall) {
            hasToolCalls = true;
            // Add function response for each function call
            followUpContents.push({
              role: "user",
              parts: [
                {
                  functionResponse: {
                    name: part.functionCall.name,
                    response: { temperature: "71 degrees" },
                  },
                },
              ],
            });
          }
        }

        // If no tool calls were found, add the generic follow-up message
        if (!hasToolCalls) {
          followUpContents.push({
            role: "user",
            parts: [{ text: "What should I do next?" }],
          });
        }

        const followUpPayload: GoogleGenerateContentRequest = {
          ...payload,
          contents: followUpContents,
        };

        result.followupRequest = followUpPayload;

        // Create follow-up promises for parallel execution
        type FollowupGoogleResult =
          | {
              type: "followupResponse";
              data: GenerateContentResponse;
            }
          | {
              type: "followupStreamingResponse";
              data: Array<GenerateContentResponse>;
            };

        const followupPromises: Promise<FollowupGoogleResult>[] = [];

        if (stream !== true) {
          followupPromises.push(
            (async () => {
              const response = await googleRequest(
                model,
                followUpPayload,
                apiKey,
                false
              );
              if (!response.ok) {
                const text = await response.text();
                throw new Error(`ApiError: ${text}`);
              }
              const data = await parseJsonResponse(response);
              return { type: "followupResponse" as const, data };
            })()
          );
        }

        if (stream !== false) {
          followupPromises.push(
            (async () => {
              const response = await googleRequest(
                model,
                followUpPayload,
                apiKey,
                true
              );
              if (!response.ok) {
                const text = await response.text();
                throw new Error(`ApiError: ${text}`);
              }
              const chunks = await parseSSEStream(response);
              return {
                type: "followupStreamingResponse" as const,
                data: chunks,
              };
            })()
          );
        }

        // Execute follow-up calls in parallel
        if (followupPromises.length > 0) {
          const followupResults = await Promise.all(followupPromises);

          for (const result_ of followupResults) {
            if (result_.type === "followupResponse") {
              result.followupResponse = result_.data;
            } else if (result_.type === "followupStreamingResponse") {
              result.followupStreamingResponse = result_.data;
            }
          }
        }
      }
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const googleExecutor: ProviderExecutor<
  GoogleGenerateContentRequest,
  GenerateContentResponse,
  GenerateContentResponse
> = {
  name: "google",
  cases: googleCases,
  execute: executeGoogle,
};
