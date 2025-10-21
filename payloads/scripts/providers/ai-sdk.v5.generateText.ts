import {
  generateText,
  streamText,
  CoreMessage,
  ToolCallPart,
  ToolResultPart,
} from "ai";
import { CaptureResult, ProviderExecutor } from "../types";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";

// Use any for the request type to avoid complex type issues
// The actual types are validated at runtime by the AI SDK
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AISDKRequest = any;

type AISDKResponse = Awaited<ReturnType<typeof generateText>>;
type AISDKStreamChunk = {
  type: string;
  value?: unknown;
  textDelta?: string;
  toolCallDelta?: unknown;
  toolResult?: unknown;
  finishReason?: string;
  usage?: unknown;
  rawResponse?: unknown;
  warnings?: unknown;
  error?: unknown;
};

// AI SDK cases - extracted from unified cases
export const aiSDKCases: Record<string, AISDKRequest> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(
    allTestCases,
    caseName,
    "ai-sdk.v5.generateText"
  );
  if (caseData) {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    aiSDKCases[caseName] = caseData as AISDKRequest;
  }
});

type ParallelAISDKResult =
  | {
      type: "response";
      data: AISDKResponse;
    }
  | {
      type: "streamingResponse";
      data: Array<AISDKStreamChunk>;
    };

export async function executeAISDK(
  caseName: string,
  payload: AISDKRequest,
  // TODO: stream should always be false
  stream?: boolean
): Promise<CaptureResult<AISDKRequest, AISDKResponse, AISDKStreamChunk>> {
  const result: CaptureResult<AISDKRequest, AISDKResponse, AISDKStreamChunk> = {
    request: payload,
  };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelAISDKResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        generateText(payload).then((response) => ({
          type: "response",
          data: response,
        }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: Array<AISDKStreamChunk> = [];
          const streamResponse = await streamText(payload);

          for await (const chunk of streamResponse.textStream) {
            streamChunks.push({
              type: "text",
              textDelta: chunk,
            });
          }

          // Also capture the full stream result if available
          const fullStreamResult = await streamResponse;
          if (fullStreamResult) {
            streamChunks.push({
              type: "final",
              value: {
                text: fullStreamResult.text,
                toolCalls: fullStreamResult.toolCalls,
                toolResults: fullStreamResult.toolResults,
                usage: fullStreamResult.usage,
                finishReason: fullStreamResult.finishReason,
                warnings: fullStreamResult.warnings,
                rawResponse: fullStreamResult.response,
              },
            });
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
    // We allow follow-ups even with empty responses for testing purposes
    const hasText =
      result.response &&
      "text" in result.response &&
      result.response.text &&
      result.response.text.trim().length > 0;
    const hasToolCalls =
      result.response &&
      result.response.toolCalls &&
      result.response.toolCalls.length > 0;

    if (result.response && "text" in result.response) {
      // Build follow-up messages
      const originalMessages = payload.messages || [];
      const followUpMessages: CoreMessage[] = [...originalMessages];

      // Add assistant's response message
      if (hasToolCalls) {
        // Message with tool calls
        followUpMessages.push({
          role: "assistant",
          content: [
            { type: "text", text: result.response.text || "" },
            ...result.response.toolCalls.map(
              (toolCall): ToolCallPart => ({
                type: "tool-call",
                toolCallId: toolCall.toolCallId,
                toolName: toolCall.toolName,
                input: toolCall.input,
              })
            ),
          ],
        });
      } else if (hasText) {
        // Simple text message (only if text is non-empty)
        followUpMessages.push({
          role: "assistant",
          content: result.response.text,
        });
      } else if (!hasText && !hasToolCalls) {
        // For testing: add empty assistant message if response was empty/truncated
        followUpMessages.push({
          role: "assistant",
          content: "",
        });
      }

      // If the assistant message contains tool calls, add dummy tool responses
      if (hasToolCalls) {
        for (const toolCall of result.response.toolCalls) {
          const toolResult: ToolResultPart = {
            type: "tool-result",
            toolCallId: toolCall.toolCallId,
            toolName: toolCall.toolName,
            output: { type: "text", value: "71 degrees" },
          };
          followUpMessages.push({
            role: "tool",
            content: [toolResult],
          });
        }
      } else {
        // Add user follow-up message
        followUpMessages.push({
          role: "user",
          content: "What should I do next?",
        });
      }

      const followUpPayload: AISDKRequest = {
        ...payload,
        messages: followUpMessages,
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      type FollowupAISDKResult =
        | {
            type: "followupResponse";
            data: AISDKResponse;
          }
        | {
            type: "followupStreamingResponse";
            data: Array<AISDKStreamChunk>;
          };

      const followupPromises: Promise<FollowupAISDKResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          generateText(followUpPayload).then((response) => ({
            type: "followupResponse",
            data: response,
          }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: Array<AISDKStreamChunk> = [];
            const followupStreamResponse = await streamText(followUpPayload);

            for await (const chunk of followupStreamResponse.textStream) {
              followupStreamChunks.push({
                type: "text",
                textDelta: chunk,
              });
            }

            // Capture final result
            const fullResult = await followupStreamResponse;
            if (fullResult) {
              followupStreamChunks.push({
                type: "final",
                value: {
                  text: fullResult.text,
                  toolCalls: fullResult.toolCalls,
                  toolResults: fullResult.toolResults,
                  usage: fullResult.usage,
                  finishReason: fullResult.finishReason,
                  warnings: fullResult.warnings,
                  rawResponse: fullResult.response,
                },
              });
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
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const aiSDKv5GenerateTextExecutor: ProviderExecutor<
  AISDKRequest,
  AISDKResponse,
  AISDKStreamChunk
> = {
  name: "ai-sdk.v5.generateText",
  cases: aiSDKCases,
  execute: executeAISDK,
};
