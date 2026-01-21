import OpenAI from "openai";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";

// Define specific types for OpenAI
type OpenAIRequest = OpenAI.Chat.Completions.ChatCompletionCreateParams;
type OpenAIResponse = OpenAI.Chat.Completions.ChatCompletion;
type OpenAIStreamChunk = OpenAI.Chat.Completions.ChatCompletionChunk;

// OpenAI Chat Completions cases - extracted from unified cases
export const openaiCases: Record<string, OpenAIRequest> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(
    allTestCases,
    caseName,
    "chat-completions"
  );
  if (caseData) {
    openaiCases[caseName] = caseData;
  }
});

type ParallelOpenAIResult =
  | {
      type: "response";
      data: OpenAIResponse;
    }
  | {
      type: "streamingResponse";
      data: Array<OpenAIStreamChunk>;
    };

export async function executeOpenAI(
  caseName: string,
  payload: OpenAIRequest,
  options?: ExecuteOptions
): Promise<CaptureResult<OpenAIRequest, OpenAIResponse, OpenAIStreamChunk>> {
  const { stream, baseURL, apiKey } = options ?? {};
  const client = new OpenAI({
    apiKey: apiKey ?? process.env.OPENAI_API_KEY,
    baseURL: baseURL ? `${baseURL}/v1` : undefined,
  });
  const result: CaptureResult<
    OpenAIRequest,
    OpenAIResponse,
    OpenAIStreamChunk
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelOpenAIResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.chat.completions
          .create({
            ...payload,
            stream: false,
          })
          .then((response) => ({ type: "response", data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: Array<OpenAIStreamChunk> = [];
          const streamResponse = await client.chat.completions.create({
            ...payload,
            stream: true,
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
    if (
      result.response &&
      "choices" in result.response &&
      result.response.choices?.[0]?.message
    ) {
      const assistantMessage = result.response.choices[0].message;

      // Build follow-up messages, handling tool calls
      const followUpMessages: OpenAI.Chat.Completions.ChatCompletionMessageParam[] =
        [...payload.messages, assistantMessage];

      // If the assistant message contains tool calls, add dummy tool responses
      if (
        assistantMessage.tool_calls &&
        assistantMessage.tool_calls.length > 0
      ) {
        for (const toolCall of assistantMessage.tool_calls) {
          followUpMessages.push({
            role: "tool",
            tool_call_id: toolCall.id,
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

      const followUpPayload: OpenAI.Chat.Completions.ChatCompletionCreateParams =
        {
          ...payload,
          messages: followUpMessages,
        };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      type FollowupOpenAIResult =
        | {
            type: "followupResponse";
            data: OpenAIResponse;
          }
        | {
            type: "followupStreamingResponse";
            data: Array<OpenAIStreamChunk>;
          };

      const followupPromises: Promise<FollowupOpenAIResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.chat.completions
            .create({
              ...followUpPayload,
              stream: false,
            })
            .then((response) => ({ type: "followupResponse", data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: Array<OpenAIStreamChunk> = [];
            const followupStreamResponse = await client.chat.completions.create(
              {
                ...followUpPayload,
                stream: true,
              }
            );

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
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const openaiExecutor: ProviderExecutor<
  OpenAIRequest,
  OpenAIResponse,
  OpenAIStreamChunk
> = {
  name: "chat-completions",
  cases: openaiCases,
  execute: executeOpenAI,
  ignoredFields: [
    "id",
    "created",
    "model",
    "service_tier",
    "system_fingerprint",
    "choices.*.message.content",
    "choices.*.message.tool_calls.*.id",
    "choices.*.delta.content",
    "choices.*.delta.tool_calls.*.id",
    "choices.*.finish_reason",
    "usage",
    "obfuscation",
  ],
};
