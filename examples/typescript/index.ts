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
