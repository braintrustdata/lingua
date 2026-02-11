import {
  BedrockRuntimeClient,
  InvokeModelCommand,
  InvokeModelWithResponseStreamCommand,
} from "@aws-sdk/client-bedrock-runtime";
import Anthropic from "@anthropic-ai/sdk";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type AnthropicMessageCreateParams,
} from "../../cases";

// Bedrock Anthropic cases - uses Anthropic Messages API via Bedrock InvokeModel endpoint
export const bedrockAnthropicCases: Record<
  string,
  AnthropicMessageCreateParams
> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(
    allTestCases,
    caseName,
    "bedrock-anthropic"
  );
  if (caseData) {
    bedrockAnthropicCases[caseName] = caseData;
  }
});

function createBedrockClient(): BedrockRuntimeClient {
  const client = new BedrockRuntimeClient({ region: "us-east-1" });

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

function buildInvokeBody(payload: AnthropicMessageCreateParams): {
  modelId: string;
  body: string;
  bodyObj: BedrockAnthropicBody;
} {
  const { model, stream: _stream, ...rest } = payload;
  const bodyObj = {
    anthropic_version: "bedrock-2023-05-31",
    ...rest,
  };
  return { modelId: model, body: JSON.stringify(bodyObj), bodyObj };
}

type ParallelResult =
  | {
      type: "response";
      data: Anthropic.Messages.Message;
    }
  | {
      type: "streamingResponse";
      data: Array<unknown>;
    };

type BedrockAnthropicBody = Omit<
  AnthropicMessageCreateParams,
  "model" | "stream"
> & { anthropic_version: string };

export async function executeBedrockAnthropic(
  _caseName: string,
  payload: AnthropicMessageCreateParams | BedrockAnthropicBody,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    AnthropicMessageCreateParams | BedrockAnthropicBody,
    Anthropic.Messages.Message,
    unknown
  >
> {
  if (!("model" in payload)) {
    throw new Error(
      "bedrock-anthropic executor expects input format, not wire format"
    );
  }
  const anthropicPayload = payload;
  const { stream } = options ?? {};
  const client = createBedrockClient();
  const { modelId, body, bodyObj } = buildInvokeBody(anthropicPayload);
  const result: CaptureResult<
    AnthropicMessageCreateParams | BedrockAnthropicBody,
    Anthropic.Messages.Message,
    unknown
  > = { request: bodyObj };

  try {
    const promises: Promise<ParallelResult>[] = [];

    if (stream !== true) {
      promises.push(
        client
          .send(
            new InvokeModelCommand({
              modelId,
              contentType: "application/json",
              accept: "application/json",
              body,
            })
          )
          .then((response) => {
            const decoded = JSON.parse(new TextDecoder().decode(response.body));
            return { type: "response", data: decoded };
          })
      );
    }

    if (stream !== false) {
      promises.push(
        (async () => {
          const chunks: Array<unknown> = [];
          const response = await client.send(
            new InvokeModelWithResponseStreamCommand({
              modelId,
              contentType: "application/json",
              accept: "application/json",
              body,
            })
          );

          if (response.body) {
            for await (const event of response.body) {
              if (event.chunk?.bytes) {
                const decoded = JSON.parse(
                  new TextDecoder().decode(event.chunk.bytes)
                );
                chunks.push(decoded);
              }
            }
          }
          return { type: "streamingResponse", data: chunks };
        })()
      );
    }

    const initialResults = await Promise.all(promises);

    for (const result_ of initialResults) {
      if (result_.type === "response") {
        result.response = result_.data;
      } else if (result_.type === "streamingResponse") {
        result.streamingResponse = result_.data;
      }
    }

    // Follow-up conversation
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

      const followUpMessages: Anthropic.MessageParam[] = [
        ...payload.messages,
        assistantMessage,
      ];

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

      if (!hasToolCalls) {
        followUpMessages.push({
          role: "user",
          content: "What should I do next?",
        });
      }

      const followUpPayload: AnthropicMessageCreateParams = {
        ...payload,
        messages: followUpMessages,
      };

      const {
        modelId: followupModelId,
        body: followupBody,
        bodyObj: followupBodyObj,
      } = buildInvokeBody(followUpPayload);
      result.followupRequest = followupBodyObj;

      type FollowupResult =
        | {
            type: "followupResponse";
            data: Anthropic.Messages.Message;
          }
        | {
            type: "followupStreamingResponse";
            data: Array<unknown>;
          };

      const followupPromises: Promise<FollowupResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client
            .send(
              new InvokeModelCommand({
                modelId: followupModelId,
                contentType: "application/json",
                accept: "application/json",
                body: followupBody,
              })
            )
            .then((response) => {
              const decoded = JSON.parse(
                new TextDecoder().decode(response.body)
              );
              return { type: "followupResponse", data: decoded };
            })
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupChunks: Array<unknown> = [];
            const response = await client.send(
              new InvokeModelWithResponseStreamCommand({
                modelId: followupModelId,
                contentType: "application/json",
                accept: "application/json",
                body: followupBody,
              })
            );

            if (response.body) {
              for await (const event of response.body) {
                if (event.chunk?.bytes) {
                  const decoded = JSON.parse(
                    new TextDecoder().decode(event.chunk.bytes)
                  );
                  followupChunks.push(decoded);
                }
              }
            }
            return { type: "followupStreamingResponse", data: followupChunks };
          })()
        );
      }

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

export const bedrockAnthropicExecutor: ProviderExecutor<
  AnthropicMessageCreateParams | BedrockAnthropicBody,
  Anthropic.Messages.Message,
  unknown
> = {
  name: "bedrock-anthropic",
  cases: bedrockAnthropicCases,
  execute: executeBedrockAnthropic,
  ignoredFields: ["id", "content.*.text", "usage"],
};
