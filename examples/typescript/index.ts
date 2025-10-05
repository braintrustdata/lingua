/**
 * Lingua TypeScript Example
 *
 * This example demonstrates the core value proposition of Lingua:
 * Define your messages once in the universal Lingua format, then convert
 * them to any provider's format with zero runtime overhead.
 */

import {
  type Message,
  linguaToChatCompletionsMessages,
  linguaToAnthropicMessages,
} from "@braintrust/lingua";

// ============================================================================
// Define your messages once in Lingua's universal format
// ============================================================================

const linguaMessages: Message[] = [
  {
    role: "user",
    content: "What is the capital of France?",
  },
];

// Multi-turn conversation example
const linguaConversation: Message[] = [
  {
    role: "user",
    content: "What is 15 * 24?",
  },
  {
    role: "assistant",
    content: "15 * 24 = 360",
    id: null,
  },
  {
    role: "user",
    content: "Can you show me how you calculated that?",
  },
];

// ============================================================================
// Convert to any provider format
// ============================================================================

function main() {
  console.log("=".repeat(80));
  console.log("Lingua Universal Message Format - TypeScript Example");
  console.log("=".repeat(80));
  console.log();

  // Example 1: Simple messages
  console.log("📝 Example 1: Simple Messages");
  console.log("-".repeat(80));
  console.log("Lingua messages:");
  console.log(JSON.stringify(linguaMessages, null, 2));
  console.log();

  try {
    // Convert to OpenAI Chat Completions format
    const openaiMessages = linguaToChatCompletionsMessages(linguaMessages);
    console.log("✅ Converted to OpenAI Chat Completions format:");
    console.log(JSON.stringify(openaiMessages, null, 2));
    console.log();

    // Convert to Anthropic format
    const anthropicMessages = linguaToAnthropicMessages(linguaMessages);
    console.log("✅ Converted to Anthropic format:");
    console.log(JSON.stringify(anthropicMessages, null, 2));
    console.log();
  } catch (error) {
    console.error("❌ Conversion error:", error);
  }

  // Example 2: Multi-turn conversation
  console.log("=".repeat(80));
  console.log("📝 Example 2: Multi-Turn Conversation");
  console.log("-".repeat(80));
  console.log("Lingua messages:");
  console.log(JSON.stringify(linguaConversation, null, 2));
  console.log();

  try {
    // Convert to OpenAI format
    const openaiConversation = linguaToChatCompletionsMessages(linguaConversation);
    console.log("✅ Converted to OpenAI Chat Completions format:");
    console.log(JSON.stringify(openaiConversation, null, 2));
    console.log();

    // Convert to Anthropic format
    const anthropicConversation = linguaToAnthropicMessages(linguaConversation);
    console.log("✅ Converted to Anthropic format:");
    console.log(JSON.stringify(anthropicConversation, null, 2));
    console.log();
  } catch (error) {
    console.error("❌ Conversion error:", error);
  }

  // Summary
  console.log("=".repeat(80));
  console.log("✨ Key Benefits:");
  console.log("  • Define messages once in universal format");
  console.log("  • Convert to any provider with zero runtime overhead");
  console.log("  • Type-safe conversions with TypeScript");
  console.log("  • Supports all provider-specific features");
  console.log("=".repeat(80));
}

// Run the example
main();
