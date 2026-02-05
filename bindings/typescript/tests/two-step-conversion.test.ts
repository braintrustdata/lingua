/**
 * Two-step conversion test: Responses API format -> Lingua Messages -> Thread
 *
 * This test demonstrates the full conversion pipeline:
 * 1. Import messages from spans (Responses API format -> Lingua Messages)
 * 2. Thread preprocessor extracts and filters messages
 */

import { describe, test, expect } from "vitest";
import { importMessagesFromSpans } from "../src/index";
import * as fs from "fs";

/* eslint-disable @typescript-eslint/no-explicit-any */

describe("Two-step conversion: Responses API to Thread", () => {
  test("should extract messages from Responses API output format", () => {
    // Your actual trace data - focusing on the output field
    const outputFromTrace = [
      {
        id: "rs_0c7105cd8354f2660069824fe9039081938557c4fcb69a4d1a",
        summary: [],
        type: "reasoning",
      },
      {
        content: [
          {
            annotations: [],
            logprobs: [],
            text: "I consulted the magic 8-ball for you (I will not reveal its exact words). Its guidance leans positively — so take this as a hopeful, mystical nudge toward yes.",
            type: "output_text",
          },
        ],
        id: "msg_0c7105cd8354f2660069824fef675481938a1fde9d9e5917b9",
        role: "assistant",
        status: "completed",
        type: "message",
      },
    ];

    // Step 1: Try importing from just the output field
    console.log("\n=== Testing Output Field Conversion ===");
    const messagesFromOutput = importMessagesFromSpans([
      { output: outputFromTrace },
    ]);
    console.log(
      "Messages from output:",
      JSON.stringify(messagesFromOutput, null, 2)
    );

    // Check if assistant message was extracted
    const assistantMessages = messagesFromOutput.filter(
      (m: any) => m.role === "assistant"
    );
    console.log(`Found ${assistantMessages.length} assistant message(s)`);

    if (assistantMessages.length > 0) {
      // Find the message with actual text content (not reasoning)
      const messageWithText = assistantMessages.find((m: any) => {
        const content = JSON.stringify(m.content);
        return content.includes("magic 8-ball");
      });

      if (messageWithText) {
        console.log("✅ Found assistant message with magic 8-ball content");
        expect(messageWithText).toBeDefined();
      } else {
        console.log("❌ No assistant message contains 'magic 8-ball'");
        expect(messageWithText).toBeDefined();
      }
    } else {
      console.log("❌ No assistant messages found from output field!");
      expect(assistantMessages.length).toBeGreaterThan(0);
    }
  });
});
