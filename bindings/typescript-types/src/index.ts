declare global {
  type Buffer = Uint8Array;
}

export type * from "../../typescript/src/types";

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
