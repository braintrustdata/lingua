import { Mistral } from "@mistralai/mistralai";
import type {
  ChatCompletionRequest,
  ChatCompletionResponse,
  CompletionChunk,
} from "@mistralai/mistralai/models/components";
import { CaptureResult, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  MISTRAL_MODEL,
} from "../../cases";

// Mistral cases - extracted from unified cases
export const mistralCases: Record<string, ChatCompletionRequest> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(allTestCases, caseName, "mistral");
  if (caseData) {
    mistralCases[caseName] = caseData;
  }
});

type ParallelMistralResult =
  | {
      type: "response";
      data: ChatCompletionResponse;
    }
  | {
      type: "streamingResponse";
      data: Array<CompletionChunk>;
    };

export async function executeMistral(
  caseName: string,
  payload: ChatCompletionRequest,
  stream?: boolean
): Promise<
  CaptureResult<ChatCompletionRequest, ChatCompletionResponse, CompletionChunk>
> {
  const client = new Mistral({ apiKey: process.env.MISTRAL_API_KEY });
  const result: CaptureResult<
    ChatCompletionRequest,
    ChatCompletionResponse,
    CompletionChunk
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelMistralResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.chat
          .complete({
            ...payload,
            model: MISTRAL_MODEL,
          })
          .then((response) => ({ type: "response", data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: Array<CompletionChunk> = [];
          const streamResponse = await client.chat.stream({
            ...payload,
            model: MISTRAL_MODEL,
          });

          for await (const event of streamResponse) {
            streamChunks.push(event.data);
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
    if (
      result.response &&
      "choices" in result.response &&
      result.response.choices?.[0]?.message
    ) {
      const assistantMessage = result.response.choices[0].message;

      // Build follow-up messages, handling tool calls
      const followUpMessages: ChatCompletionRequest["messages"] = [
        ...payload.messages,
        {
          role: "assistant",
          content: assistantMessage.content || "",
          toolCalls: assistantMessage.toolCalls,
        },
      ];

      // If the assistant message contains tool calls, add dummy tool responses
      if (assistantMessage.toolCalls && assistantMessage.toolCalls.length > 0) {
        for (const toolCall of assistantMessage.toolCalls) {
          followUpMessages.push({
            role: "tool",
            toolCallId: toolCall.id,
            content: "71 degrees",
          });
        }
      } else {
        // Always add the user follow-up message
        followUpMessages.push({
          role: "user",
          content: "What should I do next?",
        });
      }

      const followUpPayload: ChatCompletionRequest = {
        ...payload,
        messages: followUpMessages,
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      type FollowupMistralResult =
        | {
            type: "followupResponse";
            data: ChatCompletionResponse;
          }
        | {
            type: "followupStreamingResponse";
            data: Array<CompletionChunk>;
          };

      const followupPromises: Promise<FollowupMistralResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.chat
            .complete({
              ...followUpPayload,
              model: MISTRAL_MODEL,
            })
            .then((response) => ({ type: "followupResponse", data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: Array<CompletionChunk> = [];
            const followupStreamResponse = await client.chat.stream({
              ...followUpPayload,
              model: MISTRAL_MODEL,
            });

            for await (const event of followupStreamResponse) {
              followupStreamChunks.push(event.data);
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

export const mistralExecutor: ProviderExecutor<
  ChatCompletionRequest,
  ChatCompletionResponse,
  CompletionChunk
> = {
  name: "mistral",
  cases: mistralCases,
  execute: executeMistral,
};
