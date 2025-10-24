import {
  type Message,
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

  console.log("\n📝 Step 1: Write in Lingua universal format");
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

  console.log("\n✅ Step 4: Response converted back to universal format");

  // ✨ Proceed in Lingua format ✨
  return response;
}

async function main() {
  const hasOpenAiApiKey = !!process.env.OPENAI_API_KEY;
  const hasAnthropicApiKey = !!process.env.ANTHROPIC_API_KEY;

  if (hasOpenAiApiKey && hasAnthropicApiKey) {
    console.log("\n" + "═".repeat(80));
    console.log("  🌍 Lingua: Universal Message Format for LLMs");
    console.log("═".repeat(80));

    const [message] = await basicUsage();

    console.log("\n💬 Response:");
    console.log("─".repeat(80));
    console.log(message.content);
    console.log("─".repeat(80));
    console.log("\n✨ One format. Any model. No proxy. ✨");
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
 * Test ideas:
 * - Agent loop
 * - Fallback to different provider within agent loop
 * - Fan out to multiple providers using same lingua messages, then do something cool with the results (choose best candidate perhaps or have LLM choose best?)
 */

main().catch(console.error);
