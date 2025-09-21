import { TestCaseCollection } from "./types";
import { OPENAI_CHAT_COMPLETIONS_MODEL, OPENAI_RESPONSES_MODEL, ANTHROPIC_MODEL } from "./models";

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
      max_tokens: 300,
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
      reasoning: { effort: "high" },
      input: [
        {
          role: "user",
          content: "A company has 100 employees. 60% work in engineering, 25% in sales, and the rest in administration. If engineering gets a 10% budget increase, sales gets 15%, and admin gets 5%, and the total budget was originally $1M, what's the new total budget if each department's budget is proportional to their headcount?",
        },
      ],
      max_output_tokens: 1000,
    },

    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "A company has 100 employees. 60% work in engineering, 25% in sales, and the rest in administration. If engineering gets a 10% budget increase, sales gets 15%, and admin gets 5%, and the total budget was originally $1M, what's the new total budget if each department's budget is proportional to their headcount?",
        },
      ],
      max_tokens: 1000,
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1000,
      messages: [
        {
          role: "user",
          content: "A company has 100 employees. 60% work in engineering, 25% in sales, and the rest in administration. If engineering gets a 10% budget increase, sales gets 15%, and admin gets 5%, and the total budget was originally $1M, what's the new total budget if each department's budget is proportional to their headcount?",
        },
      ],
    },
  },
};