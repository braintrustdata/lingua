/**
 * Lingua TypeScript Example - Tool Calling with Multiple Providers
 *
 * This example demonstrates the core value of Lingua:
 * Define your conversation once (including tool calls and results),
 * then execute it with any provider using their native APIs.
 *
 * The conversation flow:
 * 1. User asks about weather
 * 2. Assistant calls weather tool
 * 3. Tool returns result
 * 4. Assistant uses result to answer user
 */

import {
  type Message,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
  chatCompletionsMessagesToLingua,
  anthropicMessagesToLingua,
} from "@braintrust/lingua";

// Import SDKs
import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// ============================================================================
// Define the conversation once in Lingua's universal format
// ============================================================================

// Complete conversation with tool call and result already included
// The model just needs to look at this and provide the final answer
const linguaConversation: Message[] = [
  {
    role: "user",
    content: "What's the weather like in San Francisco?",
  },
  {
    role: "assistant",
    content: [
      {
        type: "tool_call",
        tool_call_id: "call_weather_123",
        tool_name: "get_weather",
        arguments: {
          type: "valid",
          location: "San Francisco, CA",
        },
      },
    ],
    id: null,
  },
  {
    role: "tool",
    content: [
      {
        type: "tool_result",
        tool_call_id: "call_weather_123",
        tool_name: "get_weather",
        output: "72 degrees Fahrenheit, sunny with light clouds",
      },
    ],
  },
];

// Tool definition (same schema for both providers)
const weatherTool = {
  name: "get_weather",
  description: "Get the current weather for a location",
  parameters: {
    type: "object" as const,
    properties: {
      location: {
        type: "string" as const,
        description: "The city and state, e.g. San Francisco, CA",
      },
    },
    required: ["location"],
  },
};

// ============================================================================
// Execute with OpenAI
// ============================================================================

async function runWithOpenAI() {
  if (!process.env.OPENAI_API_KEY) {
    console.log("⏭️  Skipping OpenAI (no API key)");
    console.log("   Set OPENAI_API_KEY environment variable to enable");
    console.log();
    return;
  }

  console.log("🤖 Running with OpenAI (gpt-5-nano)");
  console.log("-".repeat(80));

  try {
    const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });

    // Convert to OpenAI format
    const openaiMessages = linguaToChatCompletionsMessages(linguaConversation);

    console.log("📤 Sending conversation to OpenAI...");
    console.log(JSON.stringify(openaiMessages, null, 2));
    console.log();

    const response = await openai.chat.completions.create({
      model: "gpt-5-nano",
      messages: openaiMessages,
      tools: [
        {
          type: "function",
          function: weatherTool,
        },
      ],
    });

    console.log("✅ OpenAI Response:");
    console.log(JSON.stringify(response.choices[0].message, null, 2));
    console.log();

    // Convert response back to Lingua
    const linguaResponse = chatCompletionsMessagesToLingua([response.choices[0].message]);
    console.log("🔄 Converted to Lingua format:");
    console.log(JSON.stringify(linguaResponse, null, 2));
    console.log();
  } catch (error: any) {
    console.error("❌ OpenAI error:", error.message);
    console.log();
  }
}

// ============================================================================
// Execute with Anthropic
// ============================================================================

async function runWithAnthropic() {
  if (!process.env.ANTHROPIC_API_KEY) {
    console.log("⏭️  Skipping Anthropic (no API key)");
    console.log("   Set ANTHROPIC_API_KEY environment variable to enable");
    console.log();
    return;
  }

  console.log("🤖 Running with Anthropic (claude-sonnet-4-20250514)");
  console.log("-".repeat(80));

  try {
    const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });

    // Convert to Anthropic format
    const anthropicMessages = linguaToAnthropicMessages(linguaConversation);

    console.log("📤 Sending conversation to Anthropic...");
    console.log(JSON.stringify(anthropicMessages, null, 2));
    console.log();

    const response = await anthropic.messages.create({
      model: "claude-sonnet-4-20250514",
      max_tokens: 1024,
      messages: anthropicMessages,
      tools: [
        {
          name: weatherTool.name,
          description: weatherTool.description,
          input_schema: weatherTool.parameters,
        },
      ],
    });

    console.log("✅ Anthropic Response:");
    console.log(JSON.stringify(response, null, 2));
    console.log();

    // Convert response back to Lingua
    const linguaResponse = anthropicMessagesToLingua([
      {
        role: "assistant",
        content: response.content,
      },
    ]);
    console.log("🔄 Converted to Lingua format:");
    console.log(JSON.stringify(linguaResponse, null, 2));
    console.log();
  } catch (error: any) {
    console.error("❌ Anthropic error:", error.message);
    console.log();
  }
}

// ============================================================================
// Main
// ============================================================================

async function main() {
  console.log("=".repeat(80));
  console.log("Lingua - Universal Message Format for LLMs");
  console.log("=".repeat(80));
  console.log();
  console.log("This example shows the same conversation executed with multiple providers.");
  console.log("The conversation includes tool calling - a complex multi-turn interaction.");
  console.log();

  console.log("📝 Lingua Conversation (universal format):");
  console.log("-".repeat(80));
  console.log(JSON.stringify(linguaConversation, null, 2));
  console.log();
  console.log("=".repeat(80));
  console.log();

  await runWithOpenAI();
  console.log("=".repeat(80));
  console.log();

  await runWithAnthropic();
  console.log("=".repeat(80));
  console.log();

  console.log("✨ Key Takeaway:");
  console.log("   Same conversation → Multiple providers → Zero runtime overhead");
  console.log("=".repeat(80));
}

main().catch(console.error);
