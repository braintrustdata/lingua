import Anthropic from "@anthropic-ai/sdk";
import { writeFileSync } from "fs";
import { join } from "path";
import { updateCache } from "./cache-utils";

// Example Anthropic payloads defined with proper TypeScript types
export const anthropicPayloads = {
  simpleRequest: {
    model: "claude-3-5-sonnet-20241022",
    max_tokens: 150,
    messages: [
      {
        role: "user" as const,
        content:
          "What is the capital of France? Please explain your reasoning.",
      },
    ],
  } satisfies Anthropic.MessageCreateParams,

  withSystemPrompt: {
    model: "claude-3-5-sonnet-20241022",
    max_tokens: 200,
    system:
      "You are a helpful geography expert. Always provide detailed explanations.",
    messages: [
      {
        role: "user" as const,
        content: "What is the capital of France?",
      },
    ],
  } satisfies Anthropic.MessageCreateParams,

  toolCallRequest: {
    model: "claude-3-5-sonnet-20241022",
    max_tokens: 300,
    messages: [
      {
        role: "user" as const,
        content: "What's the weather like in San Francisco?",
      },
    ],
    tools: [
      {
        name: "get_weather",
        description: "Get the current weather for a location",
        input_schema: {
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
    ],
  } satisfies Anthropic.MessageCreateParams,

  thinkingExample: {
    model: "claude-3-5-sonnet-20241022",
    max_tokens: 500,
    messages: [
      {
        role: "user" as const,
        content:
          "Calculate the average speed if someone travels 120 miles in 2 hours. Show your thinking process.",
      },
    ],
  } satisfies Anthropic.MessageCreateParams,
};

export async function captureSingleAnthropicPayload(
  client: Anthropic,
  name: string,
  payload: Anthropic.MessageCreateParams,
  outputDir: string,
  stream?: boolean, // undefined = both, true = streaming only, false = non-streaming only
) {
  console.log(`Starting Anthropic capture for: ${name}`);

  try {
    // Save request payload
    writeFileSync(
      join(outputDir, `anthropic-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // Make API call(s) based on stream parameter
    let response: Anthropic.Message | null = null;
    let streamChunks: unknown[] | null = null;

    if (stream !== true) {
      // Make non-streaming call if stream is false or undefined
      console.log(`  → ${name}: Making non-streaming Anthropic request...`);
      // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
      response = (await client.messages.create(payload)) as Anthropic.Message;
    }

    if (stream !== false) {
      // Make streaming call if stream is true or undefined
      console.log(`  → ${name}: Making streaming Anthropic request...`);
      const chunks: unknown[] = [];
      const messageStream = client.messages.stream(payload);

      for await (const chunk of messageStream) {
        chunks.push(chunk);
      }
      streamChunks = chunks;
    }

    // Save responses
    if (response) {
      writeFileSync(
        join(outputDir, `anthropic-${name}-response.json`),
        JSON.stringify(response, null, 2),
      );
    }
    if (streamChunks) {
      writeFileSync(
        join(outputDir, `anthropic-${name}-response-streaming.json`),
        JSON.stringify(streamChunks, null, 2),
      );
    }

    // Update cache with actually generated files
    const generatedFiles: string[] = [];

    // Add files that were actually created
    generatedFiles.push(`anthropic-${name}-request.json`);

    if (response) {
      generatedFiles.push(`anthropic-${name}-response.json`);
    }
    if (streamChunks) {
      generatedFiles.push(`anthropic-${name}-response-streaming.json`);
    }

    updateCache(outputDir, "anthropic", name, payload, generatedFiles);

    console.log(`✓ Completed ${name} request and response`);
  } catch (error) {
    console.error(`✗ Failed to capture ${name}:`, error);

    // Save the request even if the API call failed
    writeFileSync(
      join(outputDir, `anthropic-${name}-request.json`),
      JSON.stringify(payload, null, 2),
    );

    // Save error details
    writeFileSync(
      join(outputDir, `anthropic-${name}-error.json`),
      JSON.stringify({ error: String(error) }, null, 2),
    );
  }
}

