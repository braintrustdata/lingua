import OpenAI from "openai";

export interface ProxyTestExpectation {
  status?: number;
  fields?: Record<string, unknown>;
  error?: {
    type?: string;
    message?: string;
  };
}

export type ProxyTestCase =
  | {
      format: "chat-completions";
      request: OpenAI.Chat.Completions.ChatCompletionCreateParams;
      expect: ProxyTestExpectation;
    }
  | {
      format: "responses";
      request: OpenAI.Responses.ResponseCreateParams;
      expect: ProxyTestExpectation;
    };

export type ProxyTestCaseCollection = Record<string, ProxyTestCase>;
