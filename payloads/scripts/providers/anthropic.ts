import Anthropic from "@anthropic-ai/sdk";
import { CaptureResult, ProviderExecutor } from "../types";
import { unifiedTestCases, getAllCaseNames } from "../unified-cases";

// Anthropic cases - extracted from unified cases
export const anthropicCases: Record<string, Anthropic.Messages.MessageCreateParams> = {};

// Populate cases from unified structure
getAllCaseNames().forEach(caseName => {
  const caseData = unifiedTestCases[caseName as keyof typeof unifiedTestCases];
  if (caseData["anthropic"]) {
    anthropicCases[caseName] = caseData["anthropic"];
  }
});

export async function executeAnthropic(
  caseName: string,
  payload: Anthropic.Messages.MessageCreateParams,
  stream?: boolean
): Promise<CaptureResult> {
  const client = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
  const result: CaptureResult = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<any>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.messages.create({
          ...payload,
          stream: false,
        }).then(response => ({ type: 'response', data: response }))
      );
    }

    // Add streaming call if requested
    if (stream !== false) {
      promises.push(
        (async () => {
          const streamChunks: unknown[] = [];
          const streamResponse = await client.messages.create({
            ...payload,
            stream: true,
          });

          for await (const chunk of streamResponse) {
            streamChunks.push(chunk);
          }
          return { type: 'streamingResponse', data: streamChunks };
        })()
      );
    }

    // Execute initial calls in parallel
    const initialResults = await Promise.all(promises);

    // Process results
    for (const result_ of initialResults) {
      if (result_.type === 'response') {
        result.response = result_.data;
      } else if (result_.type === 'streamingResponse') {
        result.streamingResponse = result_.data;
      }
    }

    // Create follow-up conversation if we have a non-streaming response
    if (result.response && "content" in result.response && Array.isArray(result.response.content)) {
      const assistantMessage: Anthropic.MessageParam = {
        role: "assistant",
        content: result.response.content,
      };

      const followUpPayload: Anthropic.MessageCreateParams = {
        ...payload,
        messages: [
          ...payload.messages,
          assistantMessage,
          { role: "user", content: "What should I do next?" },
        ],
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      const followupPromises: Promise<any>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.messages.create({
            ...followUpPayload,
            stream: false,
          }).then(response => ({ type: 'followupResponse', data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: unknown[] = [];
            const followupStreamResponse = await client.messages.create({
              ...followUpPayload,
              stream: true,
            });

            for await (const chunk of followupStreamResponse) {
              followupStreamChunks.push(chunk);
            }
            return { type: 'followupStreamingResponse', data: followupStreamChunks };
          })()
        );
      }

      // Execute follow-up calls in parallel
      if (followupPromises.length > 0) {
        const followupResults = await Promise.all(followupPromises);

        for (const result_ of followupResults) {
          if (result_.type === 'followupResponse') {
            result.followupResponse = result_.data;
          } else if (result_.type === 'followupStreamingResponse') {
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

export const anthropicExecutor: ProviderExecutor = {
  name: "anthropic",
  cases: anthropicCases,
  execute: executeAnthropic,
};