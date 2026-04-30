declare global {
  type Buffer = Uint8Array;
}

export type { Message } from "../../typescript/src/generated/Message";
export type { AssistantContent } from "../../typescript/src/generated/AssistantContent";
export type { AssistantContentPart } from "../../typescript/src/generated/AssistantContentPart";
export type { GeneratedFileContentPart } from "../../typescript/src/generated/GeneratedFileContentPart";
export type { ProviderMetadata } from "../../typescript/src/generated/ProviderMetadata";
export type { ProviderOptions } from "../../typescript/src/generated/ProviderOptions";
export type { SourceContentPart } from "../../typescript/src/generated/SourceContentPart";
export type { SourceType } from "../../typescript/src/generated/SourceType";
export type { TextContentPart } from "../../typescript/src/generated/TextContentPart";
export type { ToolCallArguments } from "../../typescript/src/generated/ToolCallArguments";
export type { ToolCallContentPart } from "../../typescript/src/generated/ToolCallContentPart";
export type { ToolContentPart } from "../../typescript/src/generated/ToolContentPart";
export type { ToolErrorContentPart } from "../../typescript/src/generated/ToolErrorContentPart";
export type { ToolResultContentPart } from "../../typescript/src/generated/ToolResultContentPart";
export type { ToolResultResponsePart } from "../../typescript/src/generated/ToolResultResponsePart";
export type { UserContent } from "../../typescript/src/generated/UserContent";
export type { UserContentPart } from "../../typescript/src/generated/UserContentPart";

export type { UniversalRequest } from "../../typescript/src/generated/UniversalRequest";
export type { UniversalParams } from "../../typescript/src/generated/UniversalParams";
export type { ProviderFormat } from "../../typescript/src/generated/ProviderFormat";

export type { ReasoningConfig } from "../../typescript/src/generated/ReasoningConfig";
export type { ReasoningEffort } from "../../typescript/src/generated/ReasoningEffort";
export type { ReasoningCanonical } from "../../typescript/src/generated/ReasoningCanonical";
export type { SummaryMode } from "../../typescript/src/generated/SummaryMode";
export type { ToolChoiceConfig } from "../../typescript/src/generated/ToolChoiceConfig";
export type { ToolChoiceMode } from "../../typescript/src/generated/ToolChoiceMode";
export type { ResponseFormatConfig } from "../../typescript/src/generated/ResponseFormatConfig";
export type { ResponseFormatType } from "../../typescript/src/generated/ResponseFormatType";
export type { JsonSchemaConfig } from "../../typescript/src/generated/JsonSchemaConfig";

export type { UniversalTool } from "../../typescript/src/generated/UniversalTool";
export type { UniversalToolType } from "../../typescript/src/generated/UniversalToolType";
export type { BuiltinToolProvider } from "../../typescript/src/generated/BuiltinToolProvider";
export type { TokenBudget } from "../../typescript/src/generated/TokenBudget";

export type { ChatCompletionRequestMessage } from "../../typescript/src/generated/openai/ChatCompletionRequestMessage";
export type { InputItem } from "../../typescript/src/generated/openai/InputItem";
export type { InputMessage } from "../../typescript/src/generated/anthropic/InputMessage";

export type ValidationResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: { message: string } };

export type TransformStreamChunkResult =
  | { passThrough: true; data: unknown }
  | { transformed: true; data: unknown; sourceFormat: string };

export interface StreamSessionChunk {
  data: unknown;
  eventType?: string;
}

export interface TransformStreamSessionHandle {
  push(input: string): StreamSessionChunk[];
  finish(): StreamSessionChunk[];
  pushSse(input: string): string[];
  finishSse(): string[];
}
