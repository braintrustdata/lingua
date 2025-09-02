import { OpenAI } from "openai";

async function main() {
  const client = new OpenAI();
  await client.responses.create({
    model: "gpt-5-nano",
    input: [
      {
        role: "user",
        content: "What is the square root of 82? Think hard about it",
      },
    ],
    reasoning: {
      summary: "auto",
      effort: "medium",
    },
  });
}

main().catch(console.error);
