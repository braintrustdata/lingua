// Standardized types for payload capture across all providers

export interface CaptureResult<
  TRequest = unknown,
  TResponse = unknown,
  TStreamChunk = unknown,
> {
  request: TRequest;
  response?: TResponse;
  streamingResponse?: TStreamChunk[];
  followupRequest?: TRequest;
  followupResponse?: TResponse;
  followupStreamingResponse?: TStreamChunk[];
  error?: string;
}

export interface ProviderCase<TPayload = unknown> {
  name: string;
  payload: TPayload;
}

export interface ProviderExecutor<
  TRequest = unknown,
  TResponse = unknown,
  TStreamChunk = unknown,
> {
  name: string;
  cases: Record<string, TRequest>;
  execute: (
    caseName: string,
    payload: TRequest,
    stream?: boolean
  ) => Promise<CaptureResult<TRequest, TResponse, TStreamChunk>>;
}
