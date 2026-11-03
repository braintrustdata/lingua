import { TestCase, TestCaseCollection } from "./types";
import { OPENAI_DECISIONS_MODEL, TYPESAFE_MODEL } from "./models";

// Decision cases only have decision-model payloads.
const NO_CHAT_PROVIDERS = {
  "chat-completions": null,
  responses: null,
  anthropic: null,
  google: null,
  bedrock: null,
} satisfies Partial<TestCase>;

const GRADER_SYSTEM_PROMPT =
  "You grade a support agent's replies. Judge only what the agent wrote.";

const DECISION_IMAGE =
  "iVBORw0KGgoAAAANSUhEUgAAARgAAAEYCAIAAAAI7H7bAAACPElEQVR42u3TMQ0AAAjAsPk3DSI4eJpUwZLVBFxJAEYCI4GRACOBkcBIYCTASGAkMBIYCTASGAmMBEYCjARGAiOBkQAjgZHASGAkwEhgJDASYCQwEhgJjAQYCYwERgIjAUYCI4GRwEiAkcBIYCQwEmAkMBIYCYwEGAmMBEYCIwFGAiOBkQAjgZHASGAkwEhgJDASGAkwEhgJjARGAowERgIjgZEAI4GRwEhgJMBIYCQwEhhJAjASGAmMBBgJjARGAiMBRgIjgZHASICRwEhgJDASYCQwEhgJjAQYCYwERgIjAUYCI4GRACOBkcBIYCTASGAkMBIYCTASGAmMBEYCjARGAiOBkQAjgZHASGAkwEhgJDASGAkwEhgJjAQYCYwERgIjAUYCI4GRwEiAkcBIYCQwEmAkMBIYCYwEGAmMBEYCIwFGAiOBkcBIgJHASGAkwEhgJDASGAkwEhgJjARGAowERgIjgZEAI4GRwEhgJMBIYCQwEhgJMBIYCYwESABGAiOBkQAjgZHASGAkwEhgJDASGAkwEhgJjARGAowERgIjgZEAI4GRwEhgJMBIYCQwEmAkMBIYCYwEGAmMBEYCIwFGAiOBkcBIgJHASGAkMBJgJDASGAmMBBgJjARGAiMBRgIjgZEAI4GRwEhgJMBIYCQwEhgJMBIYCYwERgKMBEYCI4GRACOBkcBIYCTASGAkMBIgARgJjARGAowERgIjgZEAI4GRwEhgJMBIYCQwEhgJMBIYCYwERgKMBB8WJ/YfoHAN5uAAAAAASUVORK5CYII=";

// Decision test cases - the shapes scorers send to decision models
export const decisionCases: TestCaseCollection = {
  decisionImageInputRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: [
        {
          role: "user",
          content: [
            { type: "input_text", text: "Inspect this image." },
            {
              type: "input_image",
              image_url: `data:image/png;base64,${DECISION_IMAGE}`,
            },
          ],
        },
      ],
      questions: [
        {
          type: "predicate",
          name: "green",
          instructions: "Is the image green?",
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state: [
        { type: "text", text: "Inspect this image." },
        { type: "image", image: "__lingua_image_0__" },
      ],
      assets: {
        __lingua_image_0__: {
          type: "image",
          format: "png",
          data_base64: DECISION_IMAGE,
        },
      },
      questions: {
        green: { type: "noul", instructions: "Is the image green?" },
      },
    },
  },
  decisionCacheOptionsRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: "I was charged twice.",
      questions: [
        { type: "predicate", instructions: "Is this about billing?" },
      ],
      prompt_cache_options: { mode: "explicit", ttl: "30m" },
      safety_identifier: "support-grader",
    },
    typesafe: null,
  },
  decisionTextInputRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input:
        "I was charged twice for order 4417 and the second charge is still pending.",
      questions: [
        {
          type: "choice",
          name: "department",
          instructions: "Which team should handle this ticket?",
          choices: [
            { value: "billing" },
            { value: "shipping" },
            { value: "technical" },
          ],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state:
        "I was charged twice for order 4417 and the second charge is still pending.",
      questions: {
        department: {
          type: "choice",
          instructions: "Which team should handle this ticket?",
          criteria: { billing: null, shipping: null, technical: null },
        },
      },
    },
  },

  decisionConversationRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: [
        { role: "system", content: GRADER_SYSTEM_PROMPT },
        {
          role: "user",
          content:
            "Customer: My blender stopped working after two weeks.\nAgent: Sorry to hear that! I've started a free replacement; you'll get a shipping label by email within the hour.",
        },
      ],
      questions: [
        {
          type: "predicate",
          name: "resolved",
          instructions: "Did the agent resolve the customer's problem?",
        },
        {
          type: "score",
          name: "tone",
          instructions: "Rate the agent's tone.",
          levels: [{ label: "rude" }, { label: "neutral" }, { label: "warm" }],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state: {
        messages: [
          { role: "system", content: GRADER_SYSTEM_PROMPT },
          {
            role: "user",
            content:
              "Customer: My blender stopped working after two weeks.\nAgent: Sorry to hear that! I've started a free replacement; you'll get a shipping label by email within the hour.",
          },
        ],
      },
      questions: {
        resolved: {
          type: "noul",
          instructions: "Did the agent resolve the customer's problem?",
        },
        tone: {
          type: "score",
          instructions: "Rate the agent's tone.",
          criteria: ["rude", "neutral", "warm"],
        },
      },
    },
  },

  decisionToolConversationRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: [
        {
          role: "system",
          content: "You are a weather assistant. Use tools for live data.",
        },
        { role: "user", content: "Do I need an umbrella in Lisbon today?" },
        { role: "assistant", content: "Let me check the forecast." },
        {
          type: "function_call",
          id: "fc_forecast_1",
          status: "completed",
          call_id: "call_forecast_1",
          name: "get_forecast",
          arguments: '{"city":"Lisbon","day":"today"}',
        },
        {
          type: "function_call_output",
          call_id: "call_forecast_1",
          output: '{"condition":"rain","chance_of_rain":0.85,"high_c":17}',
        },
        {
          role: "assistant",
          content:
            "Yes, bring an umbrella: there's an 85% chance of rain in Lisbon today, with a high of 17°C.",
        },
      ],
      questions: [
        {
          type: "predicate",
          name: "grounded",
          instructions:
            "Is the assistant's final answer supported by the tool output?",
        },
        {
          type: "choice",
          name: "grade",
          instructions: "Grade the assistant's final answer.",
          choices: [{ value: "A" }, { value: "B" }, { value: "C" }],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state: {
        messages: [
          {
            role: "system",
            content: "You are a weather assistant. Use tools for live data.",
          },
          { role: "user", content: "Do I need an umbrella in Lisbon today?" },
          { role: "assistant", content: "Let me check the forecast." },
          {
            role: "assistant",
            content: [
              {
                type: "tool_call",
                tool_call_id: "call_forecast_1",
                tool_name: "get_forecast",
                arguments: { city: "Lisbon", day: "today" },
              },
            ],
          },
          {
            role: "tool",
            content: [
              {
                type: "tool_result",
                tool_call_id: "call_forecast_1",
                tool_name: "get_forecast",
                output:
                  '{"condition":"rain","chance_of_rain":0.85,"high_c":17}',
              },
            ],
          },
          {
            role: "assistant",
            content:
              "Yes, bring an umbrella: there's an 85% chance of rain in Lisbon today, with a high of 17°C.",
          },
        ],
      },
      questions: {
        grounded: {
          type: "noul",
          instructions:
            "Is the assistant's final answer supported by the tool output?",
        },
        grade: {
          type: "choice",
          instructions: "Grade the assistant's final answer.",
          criteria: { A: null, B: null, C: null },
        },
      },
    },
  },

  decisionMixedQuestionsRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "Question: What is 15% of 80?\nAnswer: 15% of 80 is 12.",
            },
          ],
        },
      ],
      questions: [
        { type: "predicate", instructions: "Is the answer correct?" },
        {
          type: "choice",
          name: "shows_work",
          instructions: "Does the answer show how it was computed?",
          choices: [{ value: true }, { value: false }],
        },
        {
          type: "score",
          instructions: [
            { type: "input_text", text: "Rate how concise the answer is." },
          ],
          levels: [{ label: "verbose" }, { label: "ok" }, { label: "concise" }],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state: "Question: What is 15% of 80?\nAnswer: 15% of 80 is 12.",
      questions: {
        correct: { type: "noul", instructions: "Is the answer correct?" },
        shows_work: {
          type: "choice",
          instructions: "Does the answer show how it was computed?",
          criteria: { true: null, false: null },
        },
        concise: {
          type: "score",
          instructions: "Rate how concise the answer is.",
          criteria: ["verbose", "ok", "concise"],
        },
      },
    },
  },

  decisionChoiceDescriptionsRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input:
        "Question: Why is the sky blue?\nAnswer: Air molecules scatter shorter (blue) wavelengths of sunlight more than longer ones, so blue light reaches your eyes from every direction.",
      questions: [
        {
          type: "choice",
          name: "factuality",
          instructions: "How factually accurate is the answer?",
          choices: [
            {
              value: "A",
              description: "Correct and complete, with no errors.",
            },
            {
              value: "B",
              description: "Mostly correct, with minor omissions.",
            },
            { value: "C", description: "Contains a factual error." },
          ],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state:
        "Question: Why is the sky blue?\nAnswer: Air molecules scatter shorter (blue) wavelengths of sunlight more than longer ones, so blue light reaches your eyes from every direction.",
      questions: {
        factuality: {
          type: "choice",
          instructions: "How factually accurate is the answer?",
          criteria: {
            A: "Correct and complete, with no errors.",
            B: "Mostly correct, with minor omissions.",
            C: "Contains a factual error.",
          },
        },
      },
    },
  },

  decisionScoreLevelsRequest: {
    ...NO_CHAT_PROVIDERS,
    decisions: {
      model: OPENAI_DECISIONS_MODEL,
      input: [
        {
          role: "user",
          content:
            "Summarize: The city council approved a new bike lane on Main Street after a three-month public comment period. Construction starts in May and will close one car lane for six weeks.",
        },
        {
          role: "assistant",
          content:
            "The council approved a Main Street bike lane; construction begins in May and closes a car lane for six weeks.",
        },
      ],
      questions: [
        {
          type: "score",
          name: "faithfulness",
          instructions: "Rate how faithful the summary is to the source.",
          levels: [
            {
              label: "unfaithful",
              description: "States things the source does not.",
            },
            { label: "partial", description: "Omits or blurs key facts." },
            {
              label: "faithful",
              description: "Every claim is supported by the source.",
            },
          ],
        },
      ],
    },
    typesafe: {
      model: TYPESAFE_MODEL,
      state: {
        messages: [
          {
            role: "user",
            content:
              "Summarize: The city council approved a new bike lane on Main Street after a three-month public comment period. Construction starts in May and will close one car lane for six weeks.",
          },
          {
            role: "assistant",
            content:
              "The council approved a Main Street bike lane; construction begins in May and closes a car lane for six weeks.",
          },
        ],
      },
      questions: {
        faithfulness: {
          type: "score",
          instructions: "Rate how faithful the summary is to the source.",
          criteria: [
            {
              label: "unfaithful",
              description: "States things the source does not.",
            },
            { label: "partial", description: "Omits or blurs key facts." },
            {
              label: "faithful",
              description: "Every claim is supported by the source.",
            },
          ],
        },
      },
    },
  },
};
