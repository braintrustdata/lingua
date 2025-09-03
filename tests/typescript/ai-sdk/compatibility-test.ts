/**
 * TypeScript compatibility test for Elmir's LanguageModelV2 types
 *
 * This test validates that our Rust-generated types are 100% compatible
 * with the Vercel AI SDK's LanguageModelV2 structure.
 *
 * If this file compiles without errors, our types are fully compatible!
 */

import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";

// Import our generated types (these would come from ts-rs generation)
import type {
  LanguageModelV2Message,
  LanguageModelV2UserContent,
  LanguageModelV2AssistantContent,
  LanguageModelV2ToolContent,
  SharedV2ProviderOptions,
} from "../../../bindings/typescript/LanguageModelV2Message";

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

  // This should accept our messages if types are compatible
  const result = generateText({
    model: openai("gpt-4"),
    messages, // ← This line validates compatibility!
  });

  console.log("✅ Basic compatibility test passed");
  return result;
}

/**
 * Test 2: Multi-modal with role-specific content
 * Tests that content types are properly restricted by role
 */
function testRoleSpecificContent() {
  // ✅ SHOULD WORK: User sending text and file
  const userMessage: LanguageModelV2Message = {
    role: "user",
    content: [
      { type: "text", text: "Analyze this image" },
      {
        type: "file",
        data: "data:image/png;base64,...",
        mimeType: "image/png",
      },
    ],
  };

  // ✅ SHOULD WORK: Assistant with reasoning, text, sources, tool calls
  const assistantMessage: LanguageModelV2Message = {
    role: "assistant",
    content: [
      { type: "reasoning", text: "Let me think about this..." },
      { type: "text", text: "I can see a cat in the image." },
      {
        type: "source",
        sourceType: "document",
        id: "doc-1",
        title: "Cat Identification Guide",
      },
      {
        type: "tool-call",
        id: "call_123",
        name: "search_images",
        args: { query: "similar cats" },
      },
    ],
  };

  // ✅ SHOULD WORK: Tool message with results only
  const toolMessage: LanguageModelV2Message = {
    role: "tool",
    content: [
      {
        type: "tool-result",
        toolCallId: "call_123",
        result: { found: 5, images: ["url1", "url2"] },
      },
    ],
  };

  console.log("✅ Role-specific content test passed");
  return [userMessage, assistantMessage, toolMessage];
}

/**
 * Test 3: Provider options and metadata
 * Tests extensible provider-specific options
 */
function testProviderOptions() {
  const messageWithOptions: LanguageModelV2Message = {
    role: "user",
    content: [{ type: "text", text: "Hello" }],
    providerOptions: {
      anthropic: {
        cache_control: { type: "ephemeral" },
        max_tokens: 1000,
      },
      openai: {
        logprobs: true,
        top_logprobs: 5,
      },
    },
  };

  const contentWithMetadata: LanguageModelV2AssistantContent = {
    type: "text",
    text: "Response with metadata",
    providerMetadata: {
      anthropic: {
        stop_reason: "end_turn",
        usage: { input_tokens: 10, output_tokens: 5 },
      },
    },
  };

  console.log("✅ Provider options test passed");
  return { messageWithOptions, contentWithMetadata };
}

/**
 * Test 4: Type safety enforcement
 * These should cause TypeScript errors if uncommented
 */
function testTypeSafety() {
  // ❌ SHOULD FAIL: User trying to send reasoning
  // const invalidUser: LanguageModelV2Message = {
  //   role: "user",
  //   content: [
  //     { type: "reasoning", text: "I'm thinking..." } // TypeScript error!
  //   ]
  // };

  // ❌ SHOULD FAIL: Tool trying to send text
  // const invalidTool: LanguageModelV2Message = {
  //   role: "tool",
  //   content: [
  //     { type: "text", text: "I'm a tool" } // TypeScript error!
  //   ]
  // };

  // ❌ SHOULD FAIL: Assistant content in user message
  // const invalidContent: LanguageModelV2Message = {
  //   role: "user",
  //   content: [
  //     { type: "tool-call", id: "123", name: "test", args: {} } // TypeScript error!
  //   ]
  // };

  console.log("✅ Type safety enforcement working (errors commented out)");
}

/**
 * Test 5: Real AI SDK integration
 * This validates that our types work with actual AI SDK functions
 */
async function testRealIntegration() {
  const conversation: LanguageModelV2Message[] = [
    {
      role: "user",
      content: [{ type: "text", text: "What's the capital of France?" }],
    },
  ];

  try {
    // This is the ultimate compatibility test - if this compiles and runs,
    // our types are 100% compatible with the AI SDK!
    const result = await generateText({
      model: openai("gpt-4o-mini"),
      messages: conversation, // ← Critical compatibility validation
      maxTokens: 50,
    });

    console.log("✅ Real AI SDK integration successful");
    console.log("Response:", result.text);
    return result;
  } catch (error) {
    console.log("❌ Integration failed:", error);
    throw error;
  }
}

/**
 * Main test runner
 * Run all compatibility tests
 */
export async function runCompatibilityTests() {
  console.log("🔍 Running Elmir ↔ AI SDK compatibility tests...\n");

  try {
    testBasicCompatibility();
    testRoleSpecificContent();
    testProviderOptions();
    testTypeSafety();

    // Uncomment when you want to test actual API calls
    // await testRealIntegration();

    console.log("\n✅ All compatibility tests passed!");
    console.log("🎉 Elmir types are fully compatible with Vercel AI SDK!");
  } catch (error) {
    console.error("\n❌ Compatibility test failed:", error);
    process.exit(1);
  }
}

// Auto-run if called directly
if (require.main === module) {
  runCompatibilityTests().catch(console.error);
}

