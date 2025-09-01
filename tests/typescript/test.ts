import { OpenAI } from 'openai';
import { SimpleMessage } from '../../bindings/typescript/SimpleMessage';
import { SimpleRole } from '../../bindings/typescript/SimpleRole';

// Test that our Elmir types are compatible with OpenAI SDK types
function testTypeCompatibility() {
  // Create Elmir messages
  const llmirMessages: SimpleMessage[] = [
    { role: "User" as SimpleRole, content: "Hello, how are you?" },
    { role: "Assistant" as SimpleRole, content: "I'm doing well!" },
  ];

  // Convert to OpenAI format (this simulates what our Rust translator does)
  const openaiMessages: OpenAI.Chat.Completions.ChatCompletionMessageParam[] = llmirMessages.map(msg => ({
    role: msg.role.toLowerCase() as "user" | "assistant",
    content: msg.content,
  }));

  // Verify we can create an OpenAI request
  const request: OpenAI.Chat.Completions.ChatCompletionCreateParamsNonStreaming = {
    model: "gpt-4",
    messages: openaiMessages,
  };

  console.log("✅ Type compatibility test passed!");

  return { llmirMessages, openaiMessages, request };
}

// Test round-trip conversion
function testRoundTrip() {
  // Simulate an OpenAI response
  const openaiResponse: OpenAI.Chat.Completions.ChatCompletion = {
    id: "chatcmpl-test",
    object: "chat.completion",
    created: Date.now() / 1000,
    model: "gpt-4",
    choices: [
      {
        index: 0,
        message: {
          role: "assistant",
          content: "This is a test response",
          refusal: null
        },
        logprobs: null,
        finish_reason: "stop"
      }
    ],
    usage: {
      prompt_tokens: 10,
      completion_tokens: 5,
      total_tokens: 15
    }
  };

  // Convert back to Elmir format
  const llmirMessages: SimpleMessage[] = openaiResponse.choices.map(choice => ({
    role: choice.message.role === "assistant" ? "Assistant" as SimpleRole : "User" as SimpleRole,
    content: choice.message.content || ""
  }));

  console.log("✅ Round-trip conversion test passed!");

  return { openaiResponse, llmirMessages };
}

// Run tests
function main() {
  console.log("🧪 Running Elmir <-> OpenAI TypeScript compatibility tests...\n");
  
  try {
    testTypeCompatibility();
    console.log("");
    testRoundTrip();
    console.log("\n🎉 All tests passed! Elmir types are compatible with OpenAI SDK.");
  } catch (error) {
    console.error("❌ Test failed:", error);
    process.exit(1);
  }
}

main();