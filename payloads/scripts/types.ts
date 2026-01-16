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

export interface ExecuteOptions {
  stream?: boolean;
  baseURL?: string; // Proxy URL (e.g., "http://localhost:8080")
  apiKey?: string; // API key override (e.g., BRAINTRUST_API_KEY)
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
    options?: ExecuteOptions
  ) => Promise<CaptureResult<TRequest, TResponse, TStreamChunk>>;
  // Fields to ignore when comparing responses (e.g., 'id', 'created', 'content')
  ignoredFields?: string[];
}
