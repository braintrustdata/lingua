import OpenAI from "openai";
import { CaptureResult, ProviderExecutor } from "../types";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";

// OpenAI Responses API cases - extracted from unified cases
export const openaiResponsesCases: Record<string, OpenAI.Responses.ResponseCreateParams> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(allTestCases, caseName, "openai-responses");
  if (caseData) {
    openaiResponsesCases[caseName] = caseData;
  }
});

export async function executeOpenAIResponses(
  caseName: string,
  payload: OpenAI.Responses.ResponseCreateParams,
  stream?: boolean,
): Promise<CaptureResult<OpenAI.Responses.ResponseCreateParams, OpenAI.Responses.Response, unknown>> {
  const client = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const result: CaptureResult<OpenAI.Responses.ResponseCreateParams, OpenAI.Responses.Response, unknown> = { request: payload };

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
    if (result.response && typeof result.response === 'object' && result.response !== null && "output" in result.response) {
      const assistantOutput = (result.response as OpenAI.Responses.Response).output;

      // Check if we have valid content for followup (not just reasoning without text)
      const response = result.response as OpenAI.Responses.Response;
      const hasValidContent = Array.isArray(assistantOutput)
        ? assistantOutput.some(
            (item) => (item as any).type !== "reasoning" || response.output_text,
          )
        : (assistantOutput as any).type !== "reasoning" || response.output_text;

      if (!hasValidContent) {
        console.log(
          `⚠️ Skipping followup for ${caseName} - response contains only reasoning without text output`,
        );
        return result;
      }

      console.log(`📝 Creating followup request for ${caseName}...`);

      // Build follow-up input, handling tool calls
      const followUpInput: any[] = [
        ...(Array.isArray(payload.input) ? payload.input : [payload.input]),
        ...(Array.isArray(assistantOutput)
          ? assistantOutput
          : [assistantOutput]),
      ];

      // Check if the assistant output contains tool calls and add tool responses
      const assistantMessages = Array.isArray(assistantOutput) ? assistantOutput : [assistantOutput];
      let hasToolCalls = false;

      for (const message of assistantMessages) {
        if ((message as any).type === "message" && (message as any).content) {
          const contentItems = Array.isArray((message as any).content) ? (message as any).content : [(message as any).content];
          for (const contentItem of contentItems) {
            if ((contentItem as any).type === "tool_call" && (contentItem as any).id) {
              hasToolCalls = true;
              // Add tool call output for OpenAI Responses API format
              followUpInput.push({
                role: "user",
                content: [
                  {
                    type: "custom_tool_call_output",
                    tool_call_id: (contentItem as any).id,
                    output: "71 degrees",
                  },
                ],
              });
            }
          }
        }
      }

      // If no tool calls were found, add the generic follow-up message
      if (!hasToolCalls) {
        followUpInput.push({ role: "user", content: "What should I do next?" });
      }

      const followUpPayload: OpenAI.Responses.ResponseCreateParams = {
        ...payload,
        input: followUpInput,
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

export const openaiResponsesExecutor: ProviderExecutor<OpenAI.Responses.ResponseCreateParams, OpenAI.Responses.Response, unknown> = {
  name: "openai-responses",
  cases: openaiResponsesCases,
  execute: executeOpenAIResponses,
};

