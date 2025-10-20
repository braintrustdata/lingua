import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";
import {
  CallSettings,
  LanguageModel,
  Prompt,
  StopCondition,
  ToolChoice,
  ToolSet,
} from "ai";

import { ProviderOptions } from "@ai-sdk/provider-utils";

type TOOLS = ToolSet;

type AISDKStreamTextParams = CallSettings &
  Prompt & {
    model: LanguageModel;
    tools?: TOOLS;
    toolChoice?: ToolChoice<TOOLS>;
    stopWhen?:
      | StopCondition<NoInfer<TOOLS>>
      | Array<StopCondition<NoInfer<TOOLS>>>;
    // experimental_telemetry?: TelemetrySettings;
    providerOptions?: ProviderOptions;
    // experimental_activeTools?: Array<keyof NoInfer<TOOLS>>;
    activeTools?: Array<keyof NoInfer<TOOLS>>;
    // experimental_output?: Output<OUTPUT, PARTIAL_OUTPUT>;
    // prepareStep?: PrepareStepFunction<NoInfer<TOOLS>>;
    // experimental_repairToolCall?: ToolCallRepairFunction<TOOLS>;
    // experimental_transform?: StreamTextTransform<TOOLS> | Array<StreamTextTransform<TOOLS>>;
    // experimental_download?: DownloadFunction | undefined;
    // includeRawChunks?: boolean;
    // onChunk?: StreamTextOnChunkCallback<TOOLS>;
    // onError?: StreamTextOnErrorCallback;
    // onFinish?: StreamTextOnFinishCallback<TOOLS>;
    // onAbort?: StreamTextOnAbortCallback<TOOLS>;
    // onStepFinish?: StreamTextOnStepFinishCallback<TOOLS>;
    // experimental_context?: unknown;
    // _internal?: { now?: () => number; generateId?: IdGenerator; currentDate?: () => Date;};
  };

type AISDKGenerateTextParams = CallSettings &
  Prompt & {
    model: LanguageModel;
    tools?: TOOLS;
    toolChoice?: ToolChoice<NoInfer<TOOLS>>;
    stopWhen?:
      | StopCondition<NoInfer<TOOLS>>
      | Array<StopCondition<NoInfer<TOOLS>>>;
    // experimental_telemetry?: TelemetrySettings;
    providerOptions?: ProviderOptions;
    // experimental_activeTools?: Array<keyof NoInfer<TOOLS>>;
    activeTools?: Array<keyof NoInfer<TOOLS>>;
    // experimental_output?: Output<OUTPUT, OUTPUT_PARTIAL>;
    // experimental_download?: DownloadFunction | undefined;
    // experimental_prepareStep?: PrepareStepFunction<NoInfer<TOOLS>>;
    // prepareStep?: PrepareStepFunction<NoInfer<TOOLS>>;
    // experimental_repairToolCall?: ToolCallRepairFunction<NoInfer<TOOLS>>;
    // onStepFinish?: GenerateTextOnStepFinishCallback<NoInfer<TOOLS>>;
    // onFinish?: GenerateTextOnFinishCallback<NoInfer<TOOLS>>;
    // experimental_context?: unknown;
    // _internal?: { generateId?: IdGenerator; currentDate?: () => Date; }
  };

// TODO: wrapLanguageModel
type AISDKCreateParams = AISDKStreamTextParams | AISDKGenerateTextParams;

// Well-defined types for test cases
export interface TestCase {
  "chat-completions": OpenAI.Chat.Completions.ChatCompletionCreateParams | null;
  responses: OpenAI.Responses.ResponseCreateParams | null;
  anthropic: Anthropic.Messages.MessageCreateParams | null;
  // TODO: what about different versions? i.e. LanguageModel changed from v4 to v5
  "ai-sdk": AISDKCreateParams | null;
}

// Collection of test cases organized by name
export interface TestCaseCollection {
  [caseName: string]: TestCase;
}

// Provider type definitions
export type ProviderType = keyof TestCase;

export const PROVIDER_TYPES = [
  "chat-completions",
  "responses",
  "anthropic",
  "ai-sdk",
] as const;
