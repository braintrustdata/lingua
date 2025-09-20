import OpenAI from "openai";
import { writeFileSync } from "fs";
import { join } from "path";
import { updateCache } from "./cache-utils";

// Example OpenAI payloads defined with proper TypeScript types
// Based on examples from https://platform.openai.com/docs/guides/reasoning
export const openaiPayloads = {
  // Original examples
  simpleRequest: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content:
          "What is the capital of France? Please explain your reasoning.",
      },
    ],
    max_completion_tokens: 150,
  } satisfies OpenAI.ChatCompletionCreateParams,

  reasoningRequest: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content:
          "Calculate the average speed if someone travels 120 miles in 2 hours.",
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

  // Examples from OpenAI reasoning guide
  matrixTranspose: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content:
          "Write a bash script that takes a matrix represented as a string with format '[1,2],[3,4],[5,6]' and prints the transpose in the same format.",
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,

  reactRefactoring: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: `Instructions:
- Given the React component below, change it so that nonfiction books have red
  text.
- Return only the code in your reply
- Do not include any additional formatting, such as markdown code blocks
- For formatting, use four space tabs, and do not allow any lines of code to
  exceed 80 columns

const books = [
  { title: 'Dune', category: 'fiction', id: 1 },
  { title: 'Frankenstein', category: 'fiction', id: 2 },
  { title: 'Moneyball', category: 'nonfiction', id: 3 },
];

export default function BookList() {
  const listItems = books.map(book =>
    <li>
      {book.title}
    </li>
  );

  return (
    <ul>{listItems}</ul>
  );
}`.trim(),
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,

  pythonAppPlanning: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: `I want to build a Python app that takes user questions and looks them up in a database where they are mapped to answers. If there is close match, it retrieves the matched answer. If there isn't, it asks the user to provide an answer and stores the question/answer pair in the database. Make a plan for the directory structure you'll need, then return each file in full. Only supply your reasoning at the beginning and end, not throughout the code.`,
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,

  stemResearchAntibiotics: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content:
          "What are three compounds we should consider investigating to advance research into new antibiotics? Why should we consider them?",
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,

  capitalOfFrance: {
    model: "gpt-5-nano",
    messages: [
      {
        role: "user" as const,
        content: "What is the capital of France?",
      },
    ],
  } satisfies OpenAI.ChatCompletionCreateParams,
};

export async function captureSinglePayload(
  client: OpenAI,
  name: string,
  payload: OpenAI.ChatCompletionCreateParams,
  outputDir: string,
  stream?: boolean, // undefined = both, true = streaming only, false = non-streaming only
) {
  console.log(`Starting capture for: ${name}`);

  try {
    // 1. Save original request payload
    writeFileSync(
      join(outputDir, `openai-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // 2. Conditionally make non-streaming and/or streaming API calls
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const promises: Promise<any>[] = [];
    let responsePromise: Promise<OpenAI.Chat.Completions.ChatCompletion> | null =
      null;
    let streamPromise: Promise<
      Array<OpenAI.Chat.Completions.ChatCompletionChunk>
    > | null = null;

    // Follow-up variables
    let followUpResponse: OpenAI.Chat.Completions.ChatCompletion | null = null;
    let followUpStreamMessages: Array<OpenAI.Chat.Completions.ChatCompletionChunk> | null = null;

    if (stream !== true) {
      // Make non-streaming call if stream is false or undefined
      console.log(`  → ${name}: Making non-streaming request...`);
      responsePromise = client.chat.completions.create({
        ...payload,
        stream: false,
      });
      promises.push(responsePromise);
    }

    if (stream !== false) {
      // Make streaming call if stream is true or undefined
      console.log(`  → ${name}: Making streaming request...`);
      streamPromise = (async () => {
        const streamMessages: Array<OpenAI.Chat.Completions.ChatCompletionChunk> =
          [];
        const stream = await client.chat.completions.create({
          ...payload,
          stream: true,
        });

        for await (const chunk of stream) {
          streamMessages.push(chunk);
        }
        return streamMessages;
      })();
      promises.push(streamPromise);
    }

    // Wait for requested calls to complete
    const [response, streamMessages] =
      stream === true
        ? [null, await streamPromise!]
        : stream === false
          ? [await responsePromise!, null]
          : [await responsePromise!, await streamPromise!];

    // Save responses
    if (response) {
      writeFileSync(
        join(outputDir, `openai-${name}-response-non-streaming.json`),
        JSON.stringify(response, null, 2),
      );
    }
    if (streamMessages) {
      writeFileSync(
        join(outputDir, `openai-${name}-response-streaming.json`),
        JSON.stringify(streamMessages, null, 2),
      );
    }

    // 4. Create follow-up conversation with "what next?"
    // Only create follow-up if we have a non-streaming response
    const assistantMessage = response?.choices[0]?.message;
    if (assistantMessage) {
      console.log(`  → ${name}: Creating follow-up conversation...`);

      const followUpMessages: OpenAI.Chat.ChatCompletionMessageParam[] = [
        ...payload.messages,
        assistantMessage,
      ];

      // If the assistant made tool calls, we need to provide tool responses
      if (
        assistantMessage.tool_calls &&
        assistantMessage.tool_calls.length > 0
      ) {
        console.log(
          `  → ${name}: Adding dummy tool responses for ${assistantMessage.tool_calls.length} tool calls`,
        );

        for (const toolCall of assistantMessage.tool_calls) {
          followUpMessages.push({
            role: "tool",
            content: "<tool response>",
            tool_call_id: toolCall.id,
          });
        }
      }

      // Add the user's follow-up message
      followUpMessages.push({
        role: "user",
        content: "what next?",
      });

      const followUpPayload = {
        ...payload,
        messages: followUpMessages,
      };

      // Save follow-up request
      writeFileSync(
        join(outputDir, `openai-${name}-followup-request.json`),
        JSON.stringify(followUpPayload, null, 2),
      );

      // Make follow-up calls based on stream parameter
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const followUpPromises: Promise<any>[] = [];
      let followUpResponsePromise: Promise<OpenAI.Chat.Completions.ChatCompletion> | null =
        null;
      let followUpStreamPromise: Promise<
        Array<OpenAI.Chat.Completions.ChatCompletionChunk>
      > | null = null;

      if (stream !== true) {
        // Make non-streaming follow-up if stream is false or undefined
        followUpResponsePromise = client.chat.completions.create({
          ...followUpPayload,
          stream: false,
        });
        followUpPromises.push(followUpResponsePromise);
      }

      if (stream !== false) {
        // Make streaming follow-up if stream is true or undefined
        followUpStreamPromise = (async () => {
          const followUpStreamMessages: Array<OpenAI.Chat.Completions.ChatCompletionChunk> =
            [];
          const followUpStream = await client.chat.completions.create({
            ...followUpPayload,
            stream: true,
          });

          for await (const chunk of followUpStream) {
            followUpStreamMessages.push(chunk);
          }
          return followUpStreamMessages;
        })();
        followUpPromises.push(followUpStreamPromise);
      }

      [followUpResponse, followUpStreamMessages] =
        stream === true
          ? [null, await followUpStreamPromise!]
          : stream === false
            ? [await followUpResponsePromise!, null]
            : [await followUpResponsePromise!, await followUpStreamPromise!];

      // Save follow-up responses
      if (followUpResponse) {
        writeFileSync(
          join(
            outputDir,
            `openai-${name}-followup-response-non-streaming.json`,
          ),
          JSON.stringify(followUpResponse, null, 2),
        );
      }
      if (followUpStreamMessages) {
        writeFileSync(
          join(outputDir, `openai-${name}-followup-response-streaming.json`),
          JSON.stringify(followUpStreamMessages, null, 2),
        );
      }
    }

    // Update cache with actually generated files
    const generatedFiles: string[] = [];

    // Add files that were actually created
    generatedFiles.push(`openai-${name}-request.json`);

    if (response) {
      generatedFiles.push(`openai-${name}-response-non-streaming.json`);
    }
    if (streamMessages) {
      generatedFiles.push(`openai-${name}-response-streaming.json`);
    }

    // Add follow-up files if they were created (only when assistantMessage exists)
    if (assistantMessage) {
      generatedFiles.push(`openai-${name}-followup-request.json`);

      if (followUpResponse) {
        generatedFiles.push(`openai-${name}-followup-response-non-streaming.json`);
      }
      if (followUpStreamMessages) {
        generatedFiles.push(`openai-${name}-followup-response-streaming.json`);
      }
    }

    updateCache(outputDir, "openai-chat", name, payload, generatedFiles);

    console.log(`✓ Completed ${name} with streaming and follow-up`);
  } catch (error) {
    console.error(`✗ Failed to capture ${name}:`, error);

    // Save the request even if the API call failed
    writeFileSync(
      join(outputDir, `openai-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // Save error details
    writeFileSync(
      join(outputDir, `openai-${name}-error.json`),
      JSON.stringify({ error: String(error) }, null, 2),
    );
  }
}

