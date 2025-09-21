import OpenAI from "openai";
import { CaptureResult, ProviderExecutor } from "../types";

// OpenAI Chat Completion cases
export const openaiCases = {
  simpleRequest: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: "What is the capital of France? Please explain your reasoning.",
      },
    ],
    max_completion_tokens: 150,
  } satisfies OpenAI.ChatCompletionCreateParams,

  reasoningRequest: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: "Calculate the average speed if someone travels 120 miles in 2 hours.",
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,

  toolCallRequest: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: "What's the weather like in San Francisco?",
      },
    ],
    tools: [
      {
        type: "function" as const,
        function: {
          name: "get_weather",
          description: "Get the current weather for a location",
          parameters: {
            type: "object",
            properties: {
              location: {
                type: "string",
                description: "The city and state, e.g. San Francisco, CA",
              },
            },
            required: ["location"],
          },
        },
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,
};

export async function executeOpenAI(
  caseName: string,
  payload: OpenAI.ChatCompletionCreateParams,
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
        client.chat.completions.create({
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
          const streamResponse = await client.chat.completions.create({
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
    if (result.response && "choices" in result.response && result.response.choices?.[0]?.message) {
      const assistantMessage = result.response.choices[0].message;

      const followUpPayload: OpenAI.ChatCompletionCreateParams = {
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
          client.chat.completions.create({
            ...followUpPayload,
            stream: false,
          }).then(response => ({ type: 'followupResponse', data: response }))
        );
      }

      if (stream !== false) {
        followupPromises.push(
          (async () => {
            const followupStreamChunks: unknown[] = [];
            const followupStreamResponse = await client.chat.completions.create({
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

export const openaiExecutor: ProviderExecutor = {
  name: "openai",
  cases: openaiCases,
  execute: executeOpenAI,
};