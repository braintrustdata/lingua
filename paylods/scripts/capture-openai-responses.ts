import OpenAI from "openai";
import { writeFileSync } from "fs";
import { join } from "path";
import { updateCache } from "./cache-utils";

// Example OpenAI Responses API payloads defined with proper TypeScript types
// Based on examples from https://platform.openai.com/docs/guides/reasoning
export const openaiResponsesPayloads: Record<
  string,
  OpenAI.Responses.ResponseCreateParams
> = {
  // Original examples adapted for Responses API
  simpleRequest: {
    model: "gpt-5-nano",
    reasoning: { effort: "medium", summary: "auto" },
    input: [
      {
        role: "user" as const,
        content:
          "What is the capital of France? Please explain your reasoning.",
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
        content:
          "Calculate the average speed if someone travels 120 miles in 2 hours.",
      },
    ],
  },

  toolCallRequest: {
    model: "gpt-5-nano",
    reasoning: { effort: "medium" as const },
    input: [
      {
        role: "user" as const,
        content: "What's the weather like in San Francisco?",
      },
    ],
    tools: [
      {
        type: "function" as const,
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
        strict: false,
      },
    ],
  },

  // Examples from OpenAI reasoning guide
  matrixTranspose: {
    model: "gpt-5-nano",
    reasoning: { effort: "medium" as const },
    input: [
      {
        role: "user" as const,
        content:
          "Write a bash script that takes a matrix represented as a string with format '[1,2],[3,4],[5,6]' and prints the transpose in the same format.",
      },
    ],
  },

  reactRefactoring: {
    model: "gpt-5-nano",
    reasoning: { effort: "low" as const },
    input: [
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
  },

  pythonAppPlanning: {
    model: "gpt-5-nano",
    reasoning: { effort: "high" as const },
    input: [
      {
        role: "user" as const,
        content: `I want to build a Python app that takes user questions and looks them up in a database where they are mapped to answers. If there is close match, it retrieves the matched answer. If there isn't, it asks the user to provide an answer and stores the question/answer pair in the database. Make a plan for the directory structure you'll need, then return each file in full. Only supply your reasoning at the beginning and end, not throughout the code.`,
      },
    ],
  },

  stemResearchAntibiotics: {
    model: "gpt-5-nano",
    reasoning: { effort: "high" as const },
    input: [
      {
        role: "user" as const,
        content:
          "What are three compounds we should consider investigating to advance research into new antibiotics? Why should we consider them?",
      },
    ],
  },

  capitalOfFrance: {
    model: "gpt-5-nano",
    reasoning: { effort: "low" as const, summary: "auto" as const },
    input: [
      {
        role: "user" as const,
        content: "What is the capital of France?",
      },
    ],
  },
};

// Extract the parameter type from the create method
type ResponseCreateParameters = Parameters<
  typeof OpenAI.prototype.responses.create
>[0];

export async function captureSingleResponsesPayload(
  client: OpenAI,
  name: string,
  payload: ResponseCreateParameters,
  outputDir: string,
  stream?: boolean, // undefined = both, true = streaming only, false = non-streaming only
) {
  console.log(`Starting Responses API capture for: ${name}`);

  try {
    // 1. Save original request payload
    writeFileSync(
      join(outputDir, `openai-responses-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // 2. Make Responses API call(s) based on stream parameter
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let response: any = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let streamResponse: any = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let followUpResponse: any = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let followUpStreamResponse: any = null;
    let followUpRequestCreated = false;

    if (stream !== true) {
      // Make non-streaming call if stream is false or undefined
      console.log(`  → ${name}: Making non-streaming Responses API request...`);
      response = await client.responses.create({
        ...payload,
        stream: false,
      });
    }

    if (stream !== false) {
      // Make streaming call if stream is true or undefined
      console.log(`  → ${name}: Making streaming Responses API request...`);
      const streamChunks: unknown[] = [];
      const responseStream = await client.responses.create({
        ...payload,
        stream: true,
      });

      // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
      for await (const chunk of responseStream as AsyncIterable<unknown>) {
        streamChunks.push(chunk);
      }
      streamResponse = streamChunks;
    }

    // Save responses
    if (response) {
      writeFileSync(
        join(outputDir, `openai-responses-${name}-response.json`),
        JSON.stringify(response, null, 2),
      );
    }
    if (streamResponse) {
      writeFileSync(
        join(outputDir, `openai-responses-${name}-response-streaming.json`),
        JSON.stringify(streamResponse, null, 2),
      );
    }

    // 3. Create follow-up conversation with "what next?"
    // Extract the assistant's response from the output (only for non-streaming responses)
    if (!response || !("output" in response)) {
      console.log(
        `  → ${name}: No non-streaming response available, skipping follow-up`,
      );

      // Update cache with actually generated files (without follow-up)
      const generatedFiles: string[] = [];
      generatedFiles.push(`openai-responses-${name}-request.json`);
      if (response) {
        generatedFiles.push(`openai-responses-${name}-response.json`);
      }
      if (streamResponse) {
        generatedFiles.push(`openai-responses-${name}-response-streaming.json`);
      }
      updateCache(outputDir, "openai-responses", name, payload, generatedFiles);

      console.log(`✓ Completed ${name} with Responses API (no follow-up)`);
      return;
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const assistantOutput = response.output?.find(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (item: any) => item.type === "message",
    );

    if (assistantOutput && assistantOutput.type === "message") {
      console.log(`  → ${name}: Creating follow-up conversation...`);

      // Extract text content from the assistant message safely
      const assistantText =
        assistantOutput.content
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          ?.filter((c: any) => c.type === "output_text")
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          .map((c: any) => (c.type === "output_text" ? c.text : ""))
          .join("") || "";

      // Build follow-up input by adding the assistant message and user follow-up
      // For simplicity, ensure we're working with a message array format
      let inputArray: ResponseCreateParameters["input"];
      if (typeof payload.input === "string") {
        // If input is a string, wrap it in a user message
        inputArray = [{ role: "user" as const, content: payload.input }];
      } else {
        inputArray = payload.input;
      }

      const followUpInput: ResponseCreateParameters["input"] = [
        ...inputArray,
        {
          role: "assistant" as const,
          content: assistantText,
        },
        {
          role: "user" as const,
          content: "what next?",
        },
      ];

      const followUpPayload: ResponseCreateParameters = {
        ...payload,
        input: followUpInput,
      };

      // Save follow-up request
      writeFileSync(
        join(outputDir, `openai-responses-${name}-followup-request.json`),
        JSON.stringify(followUpPayload, null, 2),
      );
      followUpRequestCreated = true;

      // Make follow-up Responses API call(s) based on stream parameter

      if (stream !== true) {
        // Make non-streaming follow-up if stream is false or undefined
        followUpResponse = await client.responses.create({
          ...followUpPayload,
          stream: false,
        });
      }

      if (stream !== false) {
        // Make streaming follow-up if stream is true or undefined
        const followUpStreamChunks: unknown[] = [];
        const followUpStream = await client.responses.create({
          ...followUpPayload,
          stream: true,
        });

        // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
        for await (const chunk of followUpStream as AsyncIterable<unknown>) {
          followUpStreamChunks.push(chunk);
        }
        followUpStreamResponse = followUpStreamChunks;
      }

      // Save follow-up responses
      if (followUpResponse) {
        writeFileSync(
          join(outputDir, `openai-responses-${name}-followup-response.json`),
          JSON.stringify(followUpResponse, null, 2),
        );
      }
      if (followUpStreamResponse) {
        writeFileSync(
          join(
            outputDir,
            `openai-responses-${name}-followup-response-streaming.json`,
          ),
          JSON.stringify(followUpStreamResponse, null, 2),
        );
      }
    }

    // Update cache with actually generated files
    const generatedFiles: string[] = [];

    // Add files that were actually created
    generatedFiles.push(`openai-responses-${name}-request.json`);

    if (response) {
      generatedFiles.push(`openai-responses-${name}-response.json`);
    }
    if (streamResponse) {
      generatedFiles.push(`openai-responses-${name}-response-streaming.json`);
    }

    // Add follow-up files if they were actually created
    if (followUpRequestCreated) {
      generatedFiles.push(`openai-responses-${name}-followup-request.json`);
    }
    if (followUpResponse) {
      generatedFiles.push(`openai-responses-${name}-followup-response.json`);
    }
    if (followUpStreamResponse) {
      generatedFiles.push(`openai-responses-${name}-followup-response-streaming.json`);
    }

    updateCache(outputDir, "openai-responses", name, payload, generatedFiles);

    console.log(`✓ Completed ${name} with Responses API and follow-up`);
  } catch (error) {
    console.error(`✗ Failed to capture ${name}:`, error);

    // Save the request even if the API call failed
    writeFileSync(
      join(outputDir, `openai-responses-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // Save error details
    writeFileSync(
      join(outputDir, `openai-responses-${name}-error.json`),
      JSON.stringify({ error: String(error) }, null, 2),
    );
  }
}

