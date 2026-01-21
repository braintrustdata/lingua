import Anthropic from "@anthropic-ai/sdk";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";

// Anthropic cases - extracted from unified cases
export const anthropicCases: Record<
  string,
  Anthropic.Messages.MessageCreateParams
> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(allTestCases, caseName, "anthropic");
  if (caseData) {
    anthropicCases[caseName] = caseData;
  }
});

type ParallelAnthropicResult =
  | {
      type: "response";
      data: Anthropic.Messages.Message;
    }
  | {
      type: "streamingResponse";
      data: Array<Anthropic.Messages.MessageStreamEvent>;
    };

export async function executeAnthropic(
  caseName: string,
  payload: Anthropic.Messages.MessageCreateParams,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    Anthropic.Messages.MessageCreateParams,
    Anthropic.Messages.Message,
    unknown
  >
> {
  const { stream, baseURL, apiKey } = options ?? {};
  const client = new Anthropic({
    apiKey: apiKey ?? process.env.ANTHROPIC_API_KEY,
    // Anthropic SDK adds /messages, gateway expects /v1/messages
    baseURL: baseURL ? `${baseURL}/v1` : undefined,
  });
  const result: CaptureResult<
    Anthropic.Messages.MessageCreateParams,
    Anthropic.Messages.Message,
    unknown
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelAnthropicResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.messages
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
          const streamChunks: Array<Anthropic.Messages.MessageStreamEvent> = [];
          const streamResponse = await client.messages.create({
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
      typeof result.response === "object" &&
      result.response !== null &&
      "content" in result.response &&
      Array.isArray(result.response.content)
    ) {
      const assistantMessage: Anthropic.MessageParam = {
        role: "assistant",
        content: result.response.content,
      };

      // Build follow-up messages, handling tool calls
      const followUpMessages: Anthropic.MessageParam[] = [
        ...payload.messages,
        assistantMessage,
      ];

      // Check if the assistant message contains tool_use blocks
      const assistantContent = Array.isArray(assistantMessage.content)
        ? assistantMessage.content
        : [assistantMessage.content];

      let hasToolCalls = false;
      for (const contentBlock of assistantContent) {
        if (
          typeof contentBlock === "object" &&
          contentBlock !== null &&
          "type" in contentBlock &&
          contentBlock.type === "tool_use" &&
          "id" in contentBlock &&
          typeof contentBlock.id === "string"
        ) {
          hasToolCalls = true;
          // Add tool result for each tool call
          followUpMessages.push({
            role: "user",
            content: [
              {
                type: "tool_result",
                tool_use_id: contentBlock.id,
                content: "71 degrees",
              },
            ],
          });
        }
      }

      // If no tool calls were found, add the generic follow-up message
      if (!hasToolCalls) {
        followUpMessages.push({
          role: "user",
          content: "What should I do next?",
        });
      }

      const followUpPayload: Anthropic.MessageCreateParams = {
        ...payload,
        messages: followUpMessages,
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      type FollowupAnthropicResult =
        | {
            type: "followupResponse";
            data: Anthropic.Messages.Message;
          }
        | {
            type: "followupStreamingResponse";
            data: Array<Anthropic.Messages.MessageStreamEvent>;
          };

      const followupPromises: Promise<FollowupAnthropicResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.messages
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
            const followupStreamChunks: Array<Anthropic.Messages.MessageStreamEvent> =
              [];
            const followupStreamResponse = await client.messages.create({
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

export const anthropicExecutor: ProviderExecutor<
  Anthropic.Messages.MessageCreateParams,
  Anthropic.Messages.Message,
  unknown
> = {
  name: "anthropic",
  cases: anthropicCases,
  execute: executeAnthropic,
  ignoredFields: ["id", "content.*.text", "usage"],
};
