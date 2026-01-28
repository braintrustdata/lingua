import { GoogleGenAI } from "@google/genai";
import type { GenerateContentResponse, Content } from "@google/genai";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  GoogleGenerateContentRequest,
  GOOGLE_MODEL,
} from "../../cases";

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
  const { stream, apiKey } = options ?? {};
  // Note: Google SDK doesn't support baseURL override, so we ignore it here
  const client = new GoogleGenAI({
    apiKey: apiKey ?? process.env.GOOGLE_API_KEY,
  });
  const result: CaptureResult<
    GoogleGenerateContentRequest,
    GenerateContentResponse,
    GenerateContentResponse
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelGoogleResult>[] = [];

    // Build config with tools and other settings
    const config = {
      ...payload.config,
      tools: payload.tools,
      systemInstruction: payload.systemInstruction,
    };

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.models
          .generateContent({
            model: GOOGLE_MODEL,
            contents: payload.contents,
            config,
          })
          .then((response) => ({ type: "response", data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: Array<GenerateContentResponse> = [];
          const streamResponse = await client.models.generateContentStream({
            model: GOOGLE_MODEL,
            contents: payload.contents,
            config,
          });

          for await (const chunk of streamResponse) {
            streamChunks.push(chunk);
          }
          return { type: "streamingResponse", data: streamChunks };
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
          ...payload.contents,
          assistantContent,
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

        // Build followup config with tools and other settings
        const followupConfig = {
          ...followUpPayload.config,
          tools: followUpPayload.tools,
          systemInstruction: followUpPayload.systemInstruction,
        };

        if (stream !== true) {
          followupPromises.push(
            client.models
              .generateContent({
                model: GOOGLE_MODEL,
                contents: followUpPayload.contents,
                config: followupConfig,
              })
              .then((response) => ({
                type: "followupResponse",
                data: response,
              }))
          );
        }

        if (stream !== false) {
          followupPromises.push(
            (async () => {
              const followupStreamChunks: Array<GenerateContentResponse> = [];
              const followupStreamResponse =
                await client.models.generateContentStream({
                  model: GOOGLE_MODEL,
                  contents: followUpPayload.contents,
                  config: followupConfig,
                });

              for await (const chunk of followupStreamResponse) {
                followupStreamChunks.push(chunk);
              }
              return {
                type: "followupStreamingResponse",
                data: followupStreamChunks,
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
