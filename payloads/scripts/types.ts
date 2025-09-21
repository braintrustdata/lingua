// Standardized types for payload capture across all providers

export interface CaptureResult {
  request: unknown;
  response?: unknown;
  streamingResponse?: unknown[];
  followupRequest?: unknown;
  followupResponse?: unknown;
  followupStreamingResponse?: unknown[];
  error?: string;
}

export interface ProviderCase {
  name: string;
  payload: unknown;
}

export interface ProviderExecutor {
  name: string;
  cases: Record<string, unknown>;
  execute: (caseName: string, payload: unknown, stream?: boolean) => Promise<CaptureResult>;
}