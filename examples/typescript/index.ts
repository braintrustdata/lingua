import {
  type Message,
  type UniversalParams,
  type UniversalRequest,
  type UniversalTool,
  type ProviderFormat,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  chatCompletionsMessagesToLingua,
  anthropicMessagesToLingua,
} from "@braintrust/lingua";

import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

async function basicUsage() {
  // Write messages and tools in Lingua's universal format
  const messages: Message[] = [
    {
      role: "user",
      content: "Tell me a little-known fact about pizza",
    },
  ];

  console.log("\n📝 Step 1: Write in Lingua's universal format");
  console.log("   Message:", JSON.stringify(messages[0].content));

  // (Imagine we have a feature flag controlling which model we use)
  const useOpenAi = Math.random() > 0.5;
  const provider = useOpenAi ? "OpenAI" : "Anthropic";

  console.log(`\n🎲 Step 2: Dynamically choosing provider: ${provider}`);
  console.log("\n🔄 Step 3: Calling provider API...");

  // Call any provider
  const response = useOpenAi
    ? chatCompletionsMessagesToLingua(await createOpenAiCompletion(messages))
    : anthropicMessagesToLingua(await createAnthropicCompletion(messages));

  console.log("\n✅ Step 4: Response converted back to Lingua");

  // ✨ Proceed in Lingua format ✨
  return response;
}

async function main() {
  // Always run the typed request example (no API keys needed)
  exampleTypedRequest();

  const hasOpenAiApiKey = !!process.env.OPENAI_API_KEY;
  const hasAnthropicApiKey = !!process.env.ANTHROPIC_API_KEY;

  if (hasOpenAiApiKey && hasAnthropicApiKey) {
    console.log("═".repeat(COL_WIDTH));
    console.log(
      centerText("🌍 Lingua: Universal Message Format for LLMs", COL_WIDTH)
    );
    console.log("═".repeat(COL_WIDTH));

    const [message] = await basicUsage();

    console.log("\n💬 Response:");
    // console.log("─".repeat(COL_WIDTH));
    console.log(message.content);
    // console.log("─".repeat(COL_WIDTH));
    console.log("\n" + "═".repeat(COL_WIDTH));
    console.log(
      centerText("✨ One format. Any model. No proxy. ✨", COL_WIDTH)
    );
    console.log("═".repeat(COL_WIDTH));
  } else {
    console.log(
      "⚠️  Skipping example - both OPENAI_API_KEY and ANTHROPIC_API_KEY required"
    );
  }
}

const createOpenAiCompletion = async (messages: Message[]) => {
  const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });
  const openaiMessages =
    linguaToChatCompletionsMessages<OpenAI.Chat.ChatCompletionMessageParam[]>(
      messages
    );
  const openAiResponse = await openai.chat.completions.create({
    model: "gpt-5-nano",
    messages: openaiMessages,
  });

  return [openAiResponse.choices[0].message];
};

const createAnthropicCompletion = async (messages: Message[]) => {
  const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });
  const anthropicMessages =
    linguaToAnthropicMessages<Anthropic.MessageParam[]>(messages);
  const anthropicResponse = await anthropic.messages.create({
    model: "claude-haiku-4-5-20251001",
    messages: anthropicMessages,
    max_tokens: 1000,
  });

  return [anthropicResponse];
};

/**
 * Example: Creating a request with typed parameters
 *
 * This demonstrates the ergonomics of using UniversalParams and UniversalTool types.
 */
function exampleTypedRequest() {
  console.log("\n📋 Example: Creating a typed UniversalRequest");

  // Define tools with full type safety
  const tools: UniversalTool[] = [
    {
      name: "get_weather",
      description: "Get the current weather for a location",
      parameters: {
        type: "object",
        properties: {
          location: { type: "string", description: "City name" },
          units: { type: "string", enum: ["celsius", "fahrenheit"] },
        },
        required: ["location"],
      },
      strict: null,
      kind: "function",
    },
  ];

  // Create params with all the bells and whistles
  const params: UniversalParams = {
    temperature: 0.7,
    max_tokens: BigInt(1000),
    top_p: null,
    top_k: null,
    seed: null,
    presence_penalty: null,
    frequency_penalty: null,
    stop: null,
    logprobs: null,
    top_logprobs: null,
    tools: tools,
    tool_choice: {
      mode: "auto",
      tool_name: null,
      disable_parallel: null,
    },
    parallel_tool_calls: null,
    response_format: {
      format_type: "json_schema",
      json_schema: {
        name: "weather_response",
        schema: {
          type: "object",
          properties: {
            temperature: { type: "number" },
            conditions: { type: "string" },
          },
        },
        strict: true,
        description: null,
      },
    },
    reasoning: {
      enabled: true,
      budget_tokens: BigInt(2048),
      summary: "auto",
    },
    metadata: { user_id: "example-user" },
    store: null,
    service_tier: null,
    stream: null,
  };

  // Create the full request
  const request: UniversalRequest = {
    model: "gpt-5-mini",
    messages: [
      { role: "user", content: "What's the weather in San Francisco?" },
    ],
    params: params,
  };

  console.log("   Model:", request.model);
  console.log("   Tools:", request.params.tools?.length ?? 0, "tool(s)");
  console.log("   Reasoning enabled:", request.params.reasoning?.enabled);
  console.log("   Response format:", request.params.response_format?.format_type);
}

/**
 * Test ideas:
 * - Agent loop
 * - Fallback to different provider within agent loop
 * - Fan out to multiple providers using same lingua messages, then do something cool with the results (choose best candidate perhaps or have LLM choose best?)
 */

const COL_WIDTH = 80;

function centerText(
  text: string,
  width: number,
  padChar: string = " "
): string {
  const textLength = text.length;
  if (textLength >= width) return text;

  const totalPadding = width - textLength;
  const leftPadding = Math.floor(totalPadding / 2);
  const rightPadding = totalPadding - leftPadding;

  return padChar.repeat(leftPadding) + text + padChar.repeat(rightPadding);
}

main().catch(console.error);
