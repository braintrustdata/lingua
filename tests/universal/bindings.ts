import { Message } from "../../bindings/typescript/Message";

const message: Message = {
  role: "user",
  content: [
    {
      type: "text",
      text: "Analyze this document with citations",
      provider_config: {
        anthropic: {
          cache_control: {
            type: "ephemeral",
          },
        },
      },
    },
  ],
};
