import type { ModelMessage } from "ai";
import { LanguageModelV2Message } from "../../../bindings/typescript/LanguageModelV2Message";

const message: LanguageModelV2Message = {
  role: "user",
  content: [
    {
      type: "text",
      text: "Analyze this document with citations",
      providerMetadata: {
        anthropic: {
          cache_control: {
            type: "ephemeral",
          },
        },
      },
    },
  ],
};

const aiMessage: ModelMessage = message;
