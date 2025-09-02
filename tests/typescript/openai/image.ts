import OpenAI from "openai";
const client = new OpenAI();

async function main() {
  const response = await client.responses.create({
    model: "gpt-5-nano",
    input: [
      {
        role: "user",
        content: [
          {
            type: "input_text",
            text: "What is in this image?",
          },
          {
            type: "input_file",
            file_url:
              "https://shorthand.com/the-craft/raster-images/assets/5kVrMqC0wp/sh-unsplash_5qt09yibrok-4096x2731.jpeg",
          },
        ],
      },
    ],
  });

  console.log(response.output_text);
}

main().catch(console.error);
