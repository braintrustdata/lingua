import { TestCaseCollection } from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  ANTHROPIC_MODEL,
} from "./models";

// Advanced test cases - complex functionality testing
export const advancedCases: TestCaseCollection = {
  multimodalRequest: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "What do you see in this image?",
            },
            {
              type: "image_url",
              image_url: {
                url: "https://example.com/image.jpg",
              },
            },
          ],
        },
      ],
      max_completion_tokens: 300,
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 300,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "What do you see in this image?",
            },
            {
              type: "image",
              source: {
                type: "base64",
                media_type: "image/jpeg",
                data: "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAABAAEDASIAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAv/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAX/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwCdABmX/9k=",
              },
            },
          ],
        },
      ],
    },
  },

  complexReasoningRequest: {
    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      reasoning: { effort: "high", summary: "detailed" },
      input: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
      max_output_tokens: 20_000,
    },

    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
      max_completion_tokens: 20_000,
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 20_000,
      messages: [
        {
          role: "user",
          content:
            "There is a digital clock, with minutes and hours in the form of 00:00. The clock shows all times from 00:00 to 23:59 and repeating. Imagine you had a list of all these times. Which digit(s) is the most common and which is the rarest? Can you find their percentage?",
        },
      ],
    },
  },
};
