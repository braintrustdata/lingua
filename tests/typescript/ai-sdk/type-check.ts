import type { ModelMessage, AssistantModelMessage } from "ai";

// Let's create a test to understand what AssistantModelMessage content can be
const assistantMessage1: AssistantModelMessage = {
  role: "assistant",
  content: "Hello world", // string content
};

const assistantMessage2: AssistantModelMessage = {
  role: "assistant", 
  content: [
    {
      type: "text",
      text: "Hello world"
    }
  ], // array content
};

// Export for testing
export { assistantMessage1, assistantMessage2 };