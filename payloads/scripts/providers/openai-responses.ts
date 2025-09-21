import OpenAI from "openai";
import { CaptureResult, ProviderExecutor } from "../types";

// OpenAI Responses API cases
export const openaiResponsesCases: Record<string, OpenAI.Responses.ResponseCreateParams> = {
  simpleRequest: {
    model: "gpt-5-nano",
    reasoning: { effort: "medium", summary: "auto" },
    input: [
      {
        role: "user" as const,
        content: "What is the capital of France? Please explain your reasoning.",
      },
    ],
    max_output_tokens: 150,
  },

  reasoningRequest: {
    model: "gpt-5-nano",
    reasoning: { effort: "high" as const },
    input: [
      {
        role: "user" as const,
        content: "Solve this step by step: If a train travels 60 mph for 2 hours, then 80 mph for 1 hour, what's the average speed?",
      },
    ],
    max_output_tokens: 300,
  },
};

export async function executeOpenAIResponses(
  caseName: string,
  payload: OpenAI.Responses.ResponseCreateParams,
  stream?: boolean
): Promise<CaptureResult> {
  const client = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const result: CaptureResult = { request: payload };

  try {
    // Create promises for parallel execution
    const promises: Promise<any>[] = [];

    // Add non-streaming call if requested
    if (stream !== true) {
      promises.push(
        client.responses.create({
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
          const streamResponse = await client.responses.create({
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
    if (result.response && "output" in result.response) {
      const assistantOutput = result.response.output;

      const followUpPayload: OpenAI.Responses.ResponseCreateParams = {
        ...payload,
        input: [
          ...payload.input,
          assistantOutput,
          { role: "user", content: "What should I do next?" },
        ],
      };

      result.followupRequest = followUpPayload;

      // Create follow-up promises for parallel execution
      const followupPromises: Promise<any>[] = [];

      if (stream !== true) {
        followupPromises.push(
          client.responses.create({
            ...followUpPayload,
            stream: false,
          }).then(response => ({ type: 'followupResponse', data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: unknown[] = [];
            const followupStreamResponse = await client.responses.create({
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

export const openaiResponsesExecutor: ProviderExecutor = {
  name: "openai-responses",
  cases: openaiResponsesCases,
  execute: executeOpenAIResponses,
};