import {
  BedrockRuntimeClient,
  ConverseCommand,
  ConverseStreamCommand,
  type ConverseResponse,
  type ConverseStreamOutput,
  type Message,
} from "@aws-sdk/client-bedrock-runtime";
import { CaptureResult, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  BedrockConverseRequest,
} from "../../cases";

// Bedrock cases - extracted from unified cases
export const bedrockCases: Record<string, BedrockConverseRequest> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  const caseData = getCaseForProvider(allTestCases, caseName, "bedrock");
  if (caseData) {
    bedrockCases[caseName] = caseData;
  }
});

// Create client with bearer token middleware
function createBedrockClient(): BedrockRuntimeClient {
  const client = new BedrockRuntimeClient({ region: "us-east-1" });

  // Add middleware to inject bearer token if present
  const token = process.env.AWS_BEARER_TOKEN_BEDROCK;
  if (token) {
    client.middlewareStack.add(
      (next) => async (args) => {
        const request =
          // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- AWS SDK middleware requires type assertion for request object
          (args as { request?: { headers?: Record<string, string> } }).request;
        if (request) {
          if (!request.headers) {
            request.headers = {};
          }
          request.headers["Authorization"] = `Bearer ${token}`;
        }
        return next(args);
      },
      { step: "build", name: "addBearerToken" }
    );
  }

  return client;
}

type ParallelBedrockResult =
  | {
      type: "response";
      data: ConverseResponse;
    }
  | {
      type: "streamingResponse";
      data: Array<ConverseStreamOutput>;
    };

export async function executeBedrock(
  caseName: string,
  payload: BedrockConverseRequest,
  stream?: boolean
): Promise<
  CaptureResult<BedrockConverseRequest, ConverseResponse, ConverseStreamOutput>
> {
  const client = createBedrockClient();
  const result: CaptureResult<
    BedrockConverseRequest,
    ConverseResponse,
    ConverseStreamOutput
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelBedrockResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client
          .send(new ConverseCommand(payload))
          .then((response) => ({ type: "response", data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: Array<ConverseStreamOutput> = [];
          const streamResponse = await client.send(
            new ConverseStreamCommand(payload)
          );

          if (streamResponse.stream) {
            for await (const event of streamResponse.stream) {
              streamChunks.push(event);
            }
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
    // Skip follow-up for multimodal requests as image buffers don't serialize correctly
    const hasImages = payload.messages?.some((msg) =>
      msg.content?.some(
        (block) => "image" in block || "document" in block || "video" in block
      )
    );

    if (result.response && !hasImages) {
      const outputMessage = result.response.output?.message;

      if (outputMessage) {
        // Build follow-up messages
        const followUpMessages: Message[] = [
          ...(payload.messages || []),
          outputMessage,
        ];

        // Check if the assistant message contains tool use
        const assistantContent = outputMessage.content || [];
        let hasToolCalls = false;

        for (const block of assistantContent) {
          if ("toolUse" in block && block.toolUse) {
            hasToolCalls = true;
            // Add tool result for each tool use
            followUpMessages.push({
              role: "user",
              content: [
                {
                  toolResult: {
                    toolUseId: block.toolUse.toolUseId,
                    content: [{ text: "71 degrees" }],
                  },
                },
              ],
            });
          }
        }

        // If no tool calls were found, add the generic follow-up message
        if (!hasToolCalls) {
          followUpMessages.push({
            role: "user",
            content: [{ text: "What should I do next?" }],
          });
        }

        const followUpPayload: BedrockConverseRequest = {
          ...payload,
          messages: followUpMessages,
        };

        result.followupRequest = followUpPayload;

        // Create follow-up promises for parallel execution
        type FollowupBedrockResult =
          | {
              type: "followupResponse";
              data: ConverseResponse;
            }
          | {
              type: "followupStreamingResponse";
              data: Array<ConverseStreamOutput>;
            };

        const followupPromises: Promise<FollowupBedrockResult>[] = [];

        if (stream !== true) {
          followupPromises.push(
            client
              .send(new ConverseCommand(followUpPayload))
              .then((response) => ({
                type: "followupResponse",
                data: response,
              }))
          );
        }

        if (stream !== false) {
          followupPromises.push(
            (async () => {
              const followupStreamChunks: Array<ConverseStreamOutput> = [];
              const followupStreamResponse = await client.send(
                new ConverseStreamCommand(followUpPayload)
              );

              if (followupStreamResponse.stream) {
                for await (const event of followupStreamResponse.stream) {
                  followupStreamChunks.push(event);
                }
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

export const bedrockExecutor: ProviderExecutor<
  BedrockConverseRequest,
  ConverseResponse,
  ConverseStreamOutput
> = {
  name: "bedrock",
  cases: bedrockCases,
  execute: executeBedrock,
};
