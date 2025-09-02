import { Anthropic } from "@anthropic-ai/sdk";

async function main() {
  const client = new Anthropic();
  await client.messages.create({
    model: "claude-sonnet-4-20250514",
    messages: [
      {
        role: "user",
        content: "What is the square root of 82? Think hard about it",
      },
    ],
    thinking: {
      budget_tokens: 1024,
      type: "enabled",
    },
    max_tokens: 2048,
  });
}

main().catch(console.error);
