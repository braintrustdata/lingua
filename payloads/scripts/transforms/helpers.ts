import { existsSync, readFileSync } from "fs";
import {
  createServer,
  type IncomingMessage,
  type Server,
  type ServerResponse,
} from "http";
import { join } from "path";
import {
  TransformStreamSession,
  transform_request,
  transform_response,
  validate_anthropic_request,
  validate_anthropic_response,
  validate_chat_completions_request,
  validate_chat_completions_response,
  validate_google_request,
  validate_google_response,
  validate_responses_request,
  validate_responses_response,
} from "@braintrust/lingua-wasm";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";
import {
  ANTHROPIC_MODEL,
  GOOGLE_MODEL,
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
} from "../../cases/models";

export type SourceFormat =
  | "chat-completions"
  | "responses"
  | "anthropic"
  | "google";
export type WasmFormat = "OpenAI" | "Responses" | "Anthropic" | "Google";

export interface TransformPair {
  source: SourceFormat;
  target: SourceFormat;
  wasmSource: WasmFormat;
  wasmTarget: WasmFormat;
}

export interface FixtureHandlerConfig {
  path: string;
  targetFormat: SourceFormat;
  wasmSource: WasmFormat;
  responsePath: string;
  requireStream?: boolean;
}

export type FixtureSkipReason =
  | "missing capture fixture"
  | "missing streaming capture fixture"
  | "error capture fixture";

type FixtureHandler = (
  req: IncomingMessage,
  res: ServerResponse
) => Promise<void>;

export const TRANSFORM_PAIRS: TransformPair[] = [
  {
    source: "chat-completions",
    target: "anthropic",
    wasmSource: "OpenAI",
    wasmTarget: "Anthropic",
  },
  {
    source: "responses",
    target: "anthropic",
    wasmSource: "Responses",
    wasmTarget: "Anthropic",
  },
  {
    source: "anthropic",
    target: "chat-completions",
    wasmSource: "Anthropic",
    wasmTarget: "OpenAI",
  },
  {
    source: "anthropic",
    target: "responses",
    wasmSource: "Anthropic",
    wasmTarget: "Responses",
  },
  {
    source: "chat-completions",
    target: "google",
    wasmSource: "OpenAI",
    wasmTarget: "Google",
  },
  {
    source: "anthropic",
    target: "google",
    wasmSource: "Anthropic",
    wasmTarget: "Google",
  },
  {
    source: "responses",
    target: "google",
    wasmSource: "Responses",
    wasmTarget: "Google",
  },
  {
    source: "google",
    target: "anthropic",
    wasmSource: "Google",
    wasmTarget: "Anthropic",
  },
  {
    source: "google",
    target: "chat-completions",
    wasmSource: "Google",
    wasmTarget: "OpenAI",
  },
  {
    source: "google",
    target: "responses",
    wasmSource: "Google",
    wasmTarget: "Responses",
  },
];

// Validation functions by format
const REQUEST_VALIDATORS: Record<SourceFormat, (json: string) => unknown> = {
  "chat-completions": validate_chat_completions_request,
  responses: validate_responses_request,
  anthropic: validate_anthropic_request,
  google: validate_google_request,
};

const RESPONSE_VALIDATORS: Record<SourceFormat, (json: string) => unknown> = {
  "chat-completions": validate_chat_completions_response,
  responses: validate_responses_response,
  anthropic: validate_anthropic_response,
  google: validate_google_response,
};

interface TransformResultData {
  passThrough?: boolean;
  transformed?: boolean;
  data: unknown;
  sourceFormat?: string;
}

function isTransformResultData(value: unknown): value is TransformResultData {
  return typeof value === "object" && value !== null && "data" in value;
}

// Transform and validate request
export function transformAndValidateRequest(
  input: unknown,
  wasmTarget: WasmFormat,
  targetFormat: SourceFormat
): unknown {
  const result = transform_request(
    JSON.stringify(input),
    wasmTarget,
    TARGET_MODELS[targetFormat]
  );
  if (!isTransformResultData(result)) {
    throw new Error("Invalid transform result");
  }
  const json = JSON.stringify(result.data);

  // Validate against Lingua's schema (derived from OpenAPI specs)
  REQUEST_VALIDATORS[targetFormat](json);

  return result.data;
}

// Validate and load response from file
export function loadAndValidateResponse(
  path: string,
  format: SourceFormat
): unknown {
  const json = readFileSync(path, "utf-8");

  // Validate against Lingua's schema
  RESPONSE_VALIDATORS[format](json);

  return JSON.parse(json);
}

// Map WasmFormat to SourceFormat for validation
const WASM_TO_SOURCE: Record<WasmFormat, SourceFormat> = {
  OpenAI: "chat-completions",
  Responses: "responses",
  Anthropic: "anthropic",
  Google: "google",
};

// Transform and validate response
export function transformResponseData(
  response: unknown,
  wasmSource: WasmFormat
): unknown {
  const result = transform_response(JSON.stringify(response), wasmSource);
  if (!isTransformResultData(result)) {
    throw new Error("Invalid transform result");
  }
  const json = JSON.stringify(result.data);

  // Validate transformed response against source format's schema
  const sourceFormat = WASM_TO_SOURCE[wasmSource];
  RESPONSE_VALIDATORS[sourceFormat](json);

  return result.data;
}

// Export validators for capture script
export { RESPONSE_VALIDATORS };

// Shared utilities for capture and test scripts
export const TRANSFORMS_DIR = join(__dirname, "../../transforms");

export const isParamCase = (name: string) => name.endsWith("Param");

export function getResponsePath(
  source: string,
  target: string,
  caseName: string
): string {
  return join(TRANSFORMS_DIR, `${source}_to_${target}`, `${caseName}.json`);
}

export function getStreamingResponsePath(
  source: string,
  target: string,
  caseName: string
): string {
  return join(
    TRANSFORMS_DIR,
    `${source}_to_${target}`,
    `${caseName}-streaming.json`
  );
}

export const TARGET_MODELS: Record<SourceFormat, string> = {
  anthropic: ANTHROPIC_MODEL,
  "chat-completions": OPENAI_CHAT_COMPLETIONS_MODEL,
  responses: OPENAI_RESPONSES_MODEL,
  google: GOOGLE_MODEL,
};

export function getGenAiGenerateContentPath(model: string): string {
  return `/v1beta/models/${model}:generateContent`;
}

export function getGenAiStreamGenerateContentPath(model: string): string {
  return `/v1beta/models/${model}:streamGenerateContent?alt=sse`;
}

/**
 * Parse Google `alt=sse` transport into provider event objects.
 *
 * This lives in payload-layer helpers rather than Rust/WASM because the payload
 * capture scripts are making raw HTTP requests and need to peel provider
 * transport framing before they can store fixtures as JSON event objects.
 * Lingua's Rust stream code transforms provider event objects; it does not own
 * raw provider HTTP response parsing in these scripts.
 */
export async function parseGoogleSseStream<T = unknown>(
  response: Response
): Promise<T[]> {
  const chunks: T[] = [];
  const text = await response.text();

  for (const line of text.split("\n")) {
    if (!line.startsWith("data: ")) {
      continue;
    }

    const json = line.slice(6).trim();
    if (!json) {
      continue;
    }

    try {
      const parsed: T = JSON.parse(json);
      chunks.push(parsed);
    } catch {
      // Ignore malformed SSE lines and keep capturing valid chunks.
    }
  }

  return chunks;
}

export function getTransformableCases(
  pair: TransformPair,
  filter?: string
): string[] {
  return getCaseNames(allTestCases).filter((caseName) => {
    if (filter && !caseName.includes(filter)) return false;
    const sourceCase = getCaseForProvider(allTestCases, caseName, pair.source);
    const testCase = allTestCases[caseName];
    return sourceCase != null && !testCase?.expect;
  });
}

export const STREAMING_PAIRS: TransformPair[] = [
  {
    source: "chat-completions",
    target: "anthropic",
    wasmSource: "OpenAI",
    wasmTarget: "Anthropic",
  },
  {
    source: "chat-completions",
    target: "responses",
    wasmSource: "OpenAI",
    wasmTarget: "Responses",
  },
  {
    source: "responses",
    target: "google",
    wasmSource: "Responses",
    wasmTarget: "Google",
  },
  {
    source: "responses",
    target: "anthropic",
    wasmSource: "Responses",
    wasmTarget: "Anthropic",
  },
  {
    source: "anthropic",
    target: "chat-completions",
    wasmSource: "Anthropic",
    wasmTarget: "OpenAI",
  },
  {
    source: "google",
    target: "chat-completions",
    wasmSource: "Google",
    wasmTarget: "OpenAI",
  },
];

export function getStreamingTransformableCases(
  pair: TransformPair,
  filter?: string
): string[] {
  return getTransformableCases(pair, filter).filter(
    (caseName) => !isParamCase(caseName)
  );
}

// ============================================================================
// SDK test helpers
// ============================================================================

export function isErrorCapture(path: string): boolean {
  if (!existsSync(path)) return false;
  const raw = JSON.parse(readFileSync(path, "utf-8"));
  return "error" in raw && !("id" in raw);
}

export function flattenStreamChunks(
  rawChunks: unknown[],
  wasmSource: WasmFormat
): { data: unknown; eventType?: string }[] {
  const session = new TransformStreamSession(wasmSource);
  const events = rawChunks.flatMap((chunk) =>
    session.push(JSON.stringify(chunk))
  );
  return events.concat(session.finish());
}

export function buildSse(rawChunks: unknown[], wasmSource: WasmFormat): string {
  const session = new TransformStreamSession(wasmSource);
  const body = rawChunks.flatMap((chunk) =>
    session.pushSse(JSON.stringify(chunk))
  );
  return body.concat(session.finishSse()).join("");
}

async function readJsonRequest(req: IncomingMessage): Promise<unknown> {
  const chunks: Uint8Array[] = [];

  for await (const chunk of req) {
    chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk);
  }

  const body = Buffer.concat(chunks).toString("utf-8");
  if (!body) {
    throw new Error("Expected JSON request body");
  }

  return JSON.parse(body);
}

async function writeJsonFixtureResponse(
  res: ServerResponse,
  config: FixtureHandlerConfig
): Promise<void> {
  const response = loadAndValidateResponse(
    config.responsePath,
    config.targetFormat
  );
  const output = transformResponseData(response, config.wasmSource);

  res.writeHead(200, { "content-type": "application/json" });
  res.end(JSON.stringify(output));
}

async function writeStreamingFixtureResponse(
  res: ServerResponse,
  config: FixtureHandlerConfig
): Promise<void> {
  const rawChunks = JSON.parse(readFileSync(config.responsePath, "utf-8"));
  const sseBody = buildSse(rawChunks, config.wasmSource);

  res.writeHead(200, {
    "content-type": "text/event-stream",
    "cache-control": "no-cache",
    connection: "keep-alive",
  });
  res.end(sseBody);
}

export class TransformTestServer {
  private readonly server: Server;
  private currentHandler: FixtureHandler | null = null;
  private port: number | null = null;

  constructor() {
    this.server = createServer(async (req, res) => {
      try {
        if (!this.currentHandler) {
          res.writeHead(500, { "content-type": "application/json" });
          res.end(JSON.stringify({ error: "No test handler configured" }));
          return;
        }

        await this.currentHandler(req, res);
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "Unknown server error";
        res.writeHead(500, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: message }));
      }
    });
  }

  get rootBaseUrl(): string {
    if (this.port === null) {
      throw new Error("Transform test server has not been started");
    }
    return `http://127.0.0.1:${this.port}`;
  }

  get openaiBaseUrl(): string {
    return `${this.rootBaseUrl}/v1`;
  }

  get anthropicBaseUrl(): string {
    return this.rootBaseUrl;
  }

  get genaiBaseUrl(): string {
    return this.rootBaseUrl;
  }

  async start(): Promise<void> {
    if (this.port !== null) {
      return;
    }

    await new Promise<void>((resolve, reject) => {
      this.server.once("error", reject);
      this.server.listen(0, "127.0.0.1", () => {
        this.server.off("error", reject);
        const address = this.server.address();
        if (!address || typeof address === "string") {
          reject(new Error("Failed to acquire test server port"));
          return;
        }
        this.port = address.port;
        resolve();
      });
    });
  }

  reset(): void {
    this.currentHandler = null;
  }

  async close(): Promise<void> {
    if (this.port === null) {
      return;
    }

    await new Promise<void>((resolve, reject) => {
      this.server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve();
      });
    });
    this.port = null;
    this.currentHandler = null;
  }

  useJsonFixture(config: FixtureHandlerConfig): void {
    this.currentHandler = async (req, res) => {
      if (req.method !== "POST") {
        res.writeHead(405, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "Method not allowed" }));
        return;
      }

      if (req.url !== config.path) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: `Unexpected path: ${req.url}` }));
        return;
      }

      const body = await readJsonRequest(req);
      if (
        typeof body !== "object" ||
        body === null ||
        Array.isArray(body) ||
        ("stream" in body && body.stream === true)
      ) {
        throw new Error("Expected non-streaming JSON request payload");
      }

      await writeJsonFixtureResponse(res, config);
    };
  }

  useStreamingFixture(config: FixtureHandlerConfig): void {
    this.currentHandler = async (req, res) => {
      if (req.method !== "POST") {
        res.writeHead(405, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "Method not allowed" }));
        return;
      }

      if (req.url !== config.path) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: `Unexpected path: ${req.url}` }));
        return;
      }

      const body = await readJsonRequest(req);
      if (typeof body !== "object" || body === null || Array.isArray(body)) {
        throw new Error("Expected streaming JSON request payload");
      }

      if (
        config.requireStream !== false &&
        (!("stream" in body) || body.stream !== true)
      ) {
        throw new Error(
          "Expected streaming JSON request payload with stream=true"
        );
      }

      await writeStreamingFixtureResponse(res, config);
    };
  }
}

export async function createTransformTestServer(): Promise<TransformTestServer> {
  const server = new TransformTestServer();
  await server.start();
  return server;
}

export function getFixtureSkipReason(
  path: string,
  options: {
    allowErrorCapture?: boolean;
    streaming?: boolean;
  } = {}
): FixtureSkipReason | null {
  if (!existsSync(path)) {
    return options.streaming
      ? "missing streaming capture fixture"
      : "missing capture fixture";
  }

  if (!options.allowErrorCapture && isErrorCapture(path)) {
    return "error capture fixture";
  }

  return null;
}

export function registerSkippedFixtureTest(
  pairLabel: string,
  caseName: string,
  reason: FixtureSkipReason
): string {
  return `${pairLabel} / ${caseName}: ${reason}`;
}

/* eslint-disable @typescript-eslint/consistent-type-assertions -- mock fetch for SDK testing */
export function mockFetch(body: string, contentType: string): typeof fetch {
  return (async () =>
    new Response(body, {
      status: 200,
      headers: { "content-type": contentType },
    })) as unknown as typeof fetch;
}

export function mockJsonFetch(body: unknown): typeof fetch {
  return mockFetch(JSON.stringify(body), "application/json");
}

export function mockSseFetch(sseBody: string): typeof fetch {
  return mockFetch(sseBody, "text/event-stream");
}
/* eslint-enable @typescript-eslint/consistent-type-assertions */
