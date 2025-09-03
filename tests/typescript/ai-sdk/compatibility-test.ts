/**
 * TypeScript compatibility test for Elmir's LanguageModelV2 types
 *
 * This test validates that our Rust-generated types are 100% compatible
 * with the Vercel AI SDK's LanguageModelV2 structure.
 *
 * If this file compiles without errors, our types are fully compatible!
 */

import { generateText, ModelMessage } from "ai";
import { openai } from "@ai-sdk/openai";

// Import our generated types (these would come from ts-rs generation)
import type { LanguageModelV2Message } from "../../../bindings/typescript/LanguageModelV2Message";
import type { LanguageModelV2UserContent } from "../../../bindings/typescript/LanguageModelV2UserContent";
import type { LanguageModelV2AssistantContent } from "../../../bindings/typescript/LanguageModelV2AssistantContent";
import type { LanguageModelV2ToolContent } from "../../../bindings/typescript/LanguageModelV2ToolContent";
import type { SharedV2ProviderOptions } from "../../../bindings/typescript/SharedV2ProviderOptions";

/**
 * Test 1: Basic conversation structure
 * Should compile without errors if types match AI SDK exactly
 */
function testBasicCompatibility() {
  const messages: LanguageModelV2Message[] = [
    {
      role: "system",
      content: "You are a helpful assistant.",
    },
    {
      role: "user",
      content: [
        {
          type: "text",
          text: "What's 2+2?",
        },
      ],
    },
    {
      role: "assistant",
      content: [
        {
          type: "text",
          text: "2+2 equals 4.",
        },
      ],
    },
  ];

  const aiMessages: ModelMessage[] = messages; // ← This line validates compatibility!
  const messagesRT: LanguageModelV2Message[] = aiMessages; // ← And back again!
}
