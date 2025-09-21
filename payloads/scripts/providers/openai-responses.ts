import OpenAI from "openai";
import { CaptureResult, ProviderExecutor } from "../types";
import { unifiedTestCases, getAllCaseNames } from "../unified-cases";

// OpenAI Responses API cases - extracted from unified cases
export const openaiResponsesCases: Record<string, OpenAI.Responses.ResponseCreateParams> = {};

// Populate cases from unified structure
getAllCaseNames().forEach(caseName => {
  const caseData = unifiedTestCases[caseName as keyof typeof unifiedTestCases];
  if (caseData["openai-responses"]) {
    openaiResponsesCases[caseName] = caseData["openai-responses"];
  }
});

export async function executeOpenAIResponses(
  caseName: string,
  payload: OpenAI.Responses.ResponseCreateParams,
  stream?: boolean,
): Promise<CaptureResult> {
  const client = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const result: CaptureResult = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<any>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.responses
          .create({
            ...payload,
            stream: false,
          })
          .then((response) => ({ type: "response", data: response })),
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: unknown[] = [];
          const streamResponse = await client.responses.create({
            ...payload,
            stream: true,
          });

          for await (const chunk of streamResponse) {
            streamChunks.push(chunk);
          }
          return { type: "streamingResponse", data: streamChunks };
        })(),
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

    // Create follow-up conversation if we have a non-streaming response with valid output
    if (result.response && "output" in result.response) {
      const assistantOutput = result.response.output;

      // Check if we have valid content for followup (not just reasoning without text)
      const hasValidContent = Array.isArray(assistantOutput)
        ? assistantOutput.some(
            (item) => item.type !== "reasoning" || result.response.output_text,
          )
        : assistantOutput.type !== "reasoning" || result.response.output_text;

      if (!hasValidContent) {
        console.log(
          `⚠️ Skipping followup for ${caseName} - response contains only reasoning without text output`,
        );
        return result;
      }

      console.log(`📝 Creating followup request for ${caseName}...`);

      const followUpPayload: OpenAI.Responses.ResponseCreateParams = {
        ...payload,
        input: [
          ...payload.input,
          ...(Array.isArray(assistantOutput)
            ? assistantOutput
            : [assistantOutput]),
          { role: "user", content: "What should I do next?" },
        ],
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      const followupPromises: Promise<any>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.responses
            .create({
              ...followUpPayload,
              stream: false,
            })
            .then((response) => ({ type: "followupResponse", data: response }))
            .catch((error) => {
              console.error(
                `❌ Followup non-streaming request failed for ${caseName}:`,
                error,
              );
              return {
                type: "followupResponse",
                data: null,
                error: String(error),
              };
            }),
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            try {
              const followupStreamChunks: unknown[] = [];
              const followupStreamResponse = await client.responses.create({
                ...followUpPayload,
                stream: true,
              });

              for await (const chunk of followupStreamResponse) {
                followupStreamChunks.push(chunk);
              }
              return {
                type: "followupStreamingResponse",
                data: followupStreamChunks,
              };
            } catch (error) {
              console.error(
                `❌ Followup streaming request failed for ${caseName}:`,
                error,
              );
              return {
                type: "followupStreamingResponse",
                data: null,
                error: String(error),
              };
            }
          })(),
        );
      }

      // Execute follow-up calls in parallel
      if (followupPromises.length > 0) {
        console.log(
          `🚀 Executing ${followupPromises.length} followup requests for ${caseName}...`,
        );
        const followupResults = await Promise.all(followupPromises);

        for (const result_ of followupResults) {
          if (result_.type === "followupResponse") {
            console.log(`✅ Got followup response for ${caseName}`);
            result.followupResponse = result_.data;
          } else if (result_.type === "followupStreamingResponse") {
            console.log(`✅ Got followup streaming response for ${caseName}`);
            result.followupStreamingResponse = result_.data;
          }
        }
        console.log(`📦 Followup execution completed for ${caseName}`);
      } else {
        console.log(`⚠️ No followup promises to execute for ${caseName}`);
      }
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const openaiResponsesExecutor: ProviderExecutor = {
  name: "openai-responses",
  cases: openaiResponsesCases,
  execute: executeOpenAIResponses,
};

