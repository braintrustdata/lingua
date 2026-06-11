import OpenAI from "openai";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
} from "../../cases";
import {
  ResponseInputItem,
  ResponseOutputItem,
  ResponseStreamEvent,
} from "openai/resources/responses/responses";

// OpenAI Responses API cases - extracted from unified cases
// Skips cases with expectations (those are validated, not captured)
export const openaiResponsesCases: Record<
  string,
  OpenAI.Responses.ResponseCreateParams
> = {};

// Populate cases from unified structure
getCaseNames(allTestCases).forEach((caseName) => {
  // Skip cases with expectations - they use validate.ts, not capture.ts
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "responses");
  if (caseData) {
    openaiResponsesCases[caseName] = caseData;
  }
});

type ParallelResponseResult =
  | {
      stream: true;
      data: Array<ResponseStreamEvent>;
    }
  | {
      stream: false;
      data: OpenAI.Responses.Response;
    };

type ReplayableResponseOutputItem = Extract<
  ResponseOutputItem,
  ResponseInputItem
>;

function isReplayableResponseOutputItem(
  item: ResponseOutputItem
): item is ReplayableResponseOutputItem {
  switch (item.type) {
    case "message":
    case "file_search_call":
    case "computer_call":
    case "web_search_call":
    case "function_call":
    case "reasoning":
    case "code_interpreter_call":
    case "local_shell_call":
    case "local_shell_call_output":
    case "shell_call":
    case "shell_call_output":
    case "apply_patch_call":
    case "apply_patch_call_output":
    case "mcp_list_tools":
    case "mcp_approval_request":
    case "mcp_approval_response":
    case "mcp_call":
    case "custom_tool_call":
      return true;
    default:
      return false;
  }
}

export async function executeOpenAIResponses(
  caseName: string,
  payload: OpenAI.Responses.ResponseCreateParams,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    OpenAI.Responses.ResponseCreateParams,
    OpenAI.Responses.Response,
    unknown
  >
> {
  const { stream, baseURL, apiKey } = options ?? {};
  const client = new OpenAI({
    apiKey: apiKey ?? process.env.OPENAI_API_KEY,
    baseURL: baseURL ? `${baseURL}/v1` : undefined,
  });
  const result: CaptureResult<
    OpenAI.Responses.ResponseCreateParams,
    OpenAI.Responses.Response,
    unknown
  > = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<ParallelResponseResult>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.responses
          .create({
            ...payload,
            stream: false,
          })
          .then((response) => ({ stream: false, data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: ResponseStreamEvent[] = [];
          const streamResponse = await client.responses.create({
            ...payload,
            stream: true,
          });

          for await (const chunk of streamResponse) {
            streamChunks.push(chunk);
          }
          return { stream: true, data: streamChunks };
        })()
      );
    }

    // Execute initial calls in parallel
    const initialResults = await Promise.all(promises);

    // Process results
    for (const result_ of initialResults) {
      if (result_.stream) {
        result.streamingResponse = result_.data;
      } else {
        result.response = result_.data;
      }
    }

    // Skip followup if store is disabled (responses aren't persisted)
    if (payload.store === false) {
      console.log(`⚠️ Skipping followup for ${caseName} - store is disabled`);
      return result;
    }

    // Create follow-up conversation if we have a non-streaming response with valid output
    if (
      result.response &&
      typeof result.response === "object" &&
      result.response !== null &&
      "output" in result.response
    ) {
      const assistantOutput = result.response.output;

      // Check if we have valid content for followup (not just reasoning without text)
      const response = result.response;
      const hasValidContent = assistantOutput.some(
        (item) => item.type !== "reasoning" || response.output_text
      );

      if (!hasValidContent) {
        console.log(
          `⚠️ Skipping followup for ${caseName} - response contains only reasoning without text output`
        );
        return result;
      }

      console.log(`📝 Creating followup request for ${caseName}...`);

      const replayableAssistantOutput = assistantOutput.filter(
        isReplayableResponseOutputItem
      );

      // Build follow-up input, handling tool calls
      const followUpInput: ResponseInputItem[] = [
        ...(Array.isArray(payload.input)
          ? payload.input
          : payload.input
            ? [
                {
                  role: "user" as const,
                  content: payload.input,
                },
              ]
            : []),
        ...replayableAssistantOutput,
      ];

      // Check if the assistant output contains tool calls and add tool responses
      const assistantMessages = Array.isArray(assistantOutput)
        ? assistantOutput
        : [assistantOutput];
      let hasToolCalls = false;

      for (const message of assistantMessages) {
        if (message.type === "function_call") {
          hasToolCalls = true;
          // Add tool call output for OpenAI Responses API format
          followUpInput.push({
            type: "function_call_output",
            call_id: message.call_id,
            output: "71 degrees",
          });
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
      const followupPromises: Promise<ParallelResponseResult>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.responses
            .create({
              ...followUpPayload,
              stream: false,
            })
            .then((response) => ({ stream: false, data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: ResponseStreamEvent[] = [];
            const followupStreamResponse = await client.responses.create({
              ...followUpPayload,
              stream: true,
            });

            for await (const chunk of followupStreamResponse) {
              followupStreamChunks.push(chunk);
            }
            return {
              stream: true,
              data: followupStreamChunks,
            };
          })()
        );
      }

      // Execute follow-up calls in parallel
      if (followupPromises.length > 0) {
        console.log(
          `🚀 Executing ${followupPromises.length} followup requests for ${caseName}...`
        );
        const followupResults = await Promise.all(followupPromises);

        for (const result_ of followupResults) {
          if (result_.stream) {
            console.log(`✅ Got followup streaming response for ${caseName}`);
            result.followupStreamingResponse = result_.data;
          } else {
            console.log(`✅ Got followup response for ${caseName}`);
            result.followupResponse = result_.data;
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

export const openaiResponsesExecutor: ProviderExecutor<
  OpenAI.Responses.ResponseCreateParams,
  OpenAI.Responses.Response,
  unknown
> = {
  name: "responses",
  cases: openaiResponsesCases,
  execute: executeOpenAIResponses,
  ignoredFields: [
    "id",
    "created_at",
    "output.*.content.*.text",
    "output_text",
    "usage",
  ],
};
