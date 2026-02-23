import { readFileSync } from "fs";
import { resolve, isAbsolute } from "path";
import { createSign } from "crypto";
import Anthropic from "@anthropic-ai/sdk";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type AnthropicMessageCreateParams,
} from "../../cases";

const VERTEX_ANTHROPIC_VERSION = "vertex-2023-10-16";

export const vertexAnthropicCases: Record<
  string,
  AnthropicMessageCreateParams
> = {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(
    allTestCases,
    caseName,
    "vertex-anthropic"
  );
  if (caseData) {
    vertexAnthropicCases[caseName] = caseData;
  }
});

interface ServiceAccountKey {
  client_email: string;
  private_key: string;
  token_uri: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseAccessTokenResponse(value: unknown): {
  access_token: string;
  expires_in: number;
} {
  if (!isRecord(value)) {
    throw new Error("Invalid access token response: expected object");
  }
  const accessToken = value.access_token;
  const expiresIn = value.expires_in;
  if (typeof accessToken !== "string") {
    throw new Error(
      "Invalid access token response: access_token must be string"
    );
  }
  if (typeof expiresIn !== "number") {
    throw new Error("Invalid access token response: expires_in must be number");
  }
  return { access_token: accessToken, expires_in: expiresIn };
}

function parseAnthropicMessageResponse(
  value: unknown
): Anthropic.Messages.Message {
  if (!isRecord(value)) {
    throw new Error("Invalid Anthropic response: expected object");
  }
  const id = value.id;
  const type = value.type;
  const role = value.role;
  const model = value.model;
  const content = value.content;
  const stopReason = value.stop_reason;
  const stopSequence = value.stop_sequence;
  const usage = value.usage;

  if (typeof id !== "string") {
    throw new Error("Invalid Anthropic response: id must be string");
  }
  if (type !== "message") {
    throw new Error("Invalid Anthropic response: type must be 'message'");
  }
  if (role !== "assistant") {
    throw new Error("Invalid Anthropic response: role must be 'assistant'");
  }
  if (typeof model !== "string") {
    throw new Error("Invalid Anthropic response: model must be string");
  }
  if (!Array.isArray(content)) {
    throw new Error("Invalid Anthropic response: content must be array");
  }
  if (stopReason !== null && typeof stopReason !== "string") {
    throw new Error(
      "Invalid Anthropic response: stop_reason must be string or null"
    );
  }
  if (stopSequence !== null && typeof stopSequence !== "string") {
    throw new Error(
      "Invalid Anthropic response: stop_sequence must be string or null"
    );
  }
  if (!isRecord(usage)) {
    throw new Error("Invalid Anthropic response: usage must be object");
  }
  const inputTokens = usage.input_tokens;
  const outputTokens = usage.output_tokens;
  if (typeof inputTokens !== "number") {
    throw new Error(
      "Invalid Anthropic response: usage.input_tokens must be number"
    );
  }
  if (typeof outputTokens !== "number") {
    throw new Error(
      "Invalid Anthropic response: usage.output_tokens must be number"
    );
  }

  if (!isAnthropicMessage(value)) {
    throw new Error(
      "Invalid Anthropic response: does not match Anthropic message type"
    );
  }
  return value;
}

function isAnthropicMessage(
  value: unknown
): value is Anthropic.Messages.Message {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.id === "string" &&
    value.type === "message" &&
    value.role === "assistant" &&
    Array.isArray(value.content) &&
    typeof value.model === "string" &&
    (value.stop_reason === null || typeof value.stop_reason === "string") &&
    (value.stop_sequence === null || typeof value.stop_sequence === "string") &&
    isRecord(value.usage) &&
    typeof value.usage.input_tokens === "number" &&
    typeof value.usage.output_tokens === "number"
  );
}

function loadServiceAccountKey(): ServiceAccountKey {
  const credPath = process.env.GOOGLE_APPLICATION_CREDENTIALS;
  if (!credPath) {
    throw new Error(
      "GOOGLE_APPLICATION_CREDENTIALS environment variable is required"
    );
  }
  const resolvedPath = isAbsolute(credPath)
    ? credPath
    : resolve(process.cwd(), credPath);
  const raw = readFileSync(resolvedPath, "utf-8");
  return JSON.parse(raw);
}

function createSignedJwt(key: ServiceAccountKey): string {
  const now = Math.floor(Date.now() / 1000);
  const header = Buffer.from(
    JSON.stringify({ alg: "RS256", typ: "JWT" })
  ).toString("base64url");
  const payload = Buffer.from(
    JSON.stringify({
      iss: key.client_email,
      sub: key.client_email,
      aud: key.token_uri,
      iat: now,
      exp: now + 3600,
      scope: "https://www.googleapis.com/auth/cloud-platform",
    })
  ).toString("base64url");

  const signInput = `${header}.${payload}`;
  const signer = createSign("RSA-SHA256");
  signer.update(signInput);
  const signature = signer.sign(key.private_key, "base64url");

  return `${signInput}.${signature}`;
}

let cachedToken: { token: string; expiresAt: number } | null = null;

async function getAccessToken(): Promise<string> {
  if (cachedToken && Date.now() < cachedToken.expiresAt - 60_000) {
    return cachedToken.token;
  }

  const key = loadServiceAccountKey();
  const jwt = createSignedJwt(key);

  const response = await fetch(key.token_uri, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
      assertion: jwt,
    }),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Failed to get access token: ${response.status} ${text}`);
  }

  const json: unknown = await response.json();
  const data = parseAccessTokenResponse(json);
  cachedToken = {
    token: data.access_token,
    expiresAt: Date.now() + data.expires_in * 1000,
  };
  return cachedToken.token;
}

type VertexAnthropicBody = Omit<
  AnthropicMessageCreateParams,
  "model" | "stream"
> & { anthropic_version: string };

function buildVertexBody(payload: AnthropicMessageCreateParams): {
  model: string;
  body: string;
  bodyObj: VertexAnthropicBody;
} {
  const { model, stream: _stream, ...rest } = payload;
  const bodyObj: VertexAnthropicBody = {
    anthropic_version: VERTEX_ANTHROPIC_VERSION,
    ...rest,
  };
  return { model, body: JSON.stringify(bodyObj), bodyObj };
}

function buildUrl(model: string, stream: boolean): string {
  const project = process.env.VERTEX_PROJECT;
  if (!project) {
    throw new Error("VERTEX_PROJECT environment variable is required");
  }
  const location = process.env.VERTEX_LOCATION ?? "us-east5";
  const method = stream ? "streamRawPredict" : "rawPredict";
  return `https://${location}-aiplatform.googleapis.com/v1/projects/${project}/locations/${location}/${model}:${method}`;
}

type ParallelResult =
  | { type: "response"; data: Anthropic.Messages.Message }
  | { type: "streamingResponse"; data: Array<unknown> };

async function fetchVertex(
  url: string,
  body: string,
  token: string
): Promise<Response> {
  return fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body,
  });
}

function parseStreamChunks(text: string): unknown[] {
  const chunks: unknown[] = [];
  const lines = text.split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Vertex streamRawPredict returns SSE format: "data: {...}"
    if (trimmed.startsWith("data: ")) {
      const data = trimmed.slice(6).trim();
      if (data) {
        chunks.push(JSON.parse(data));
      }
      continue;
    }

    // Also handle raw NDJSON (one JSON object per line)
    if (trimmed.startsWith("{")) {
      chunks.push(JSON.parse(trimmed));
    }
  }
  return chunks;
}

export async function executeVertexAnthropic(
  _caseName: string,
  payload: AnthropicMessageCreateParams | VertexAnthropicBody,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    AnthropicMessageCreateParams | VertexAnthropicBody,
    Anthropic.Messages.Message,
    unknown
  >
> {
  if (!("model" in payload)) {
    throw new Error(
      "vertex-anthropic executor expects input format, not wire format"
    );
  }
  const { stream } = options ?? {};
  const token = await getAccessToken();
  const { model, body, bodyObj } = buildVertexBody(payload);
  const result: CaptureResult<
    AnthropicMessageCreateParams | VertexAnthropicBody,
    Anthropic.Messages.Message,
    unknown
  > = { request: bodyObj };

  try {
    const promises: Promise<ParallelResult>[] = [];

    if (stream !== true) {
      const url = buildUrl(model, false);
      promises.push(
        fetchVertex(url, body, token).then(async (response) => {
          if (!response.ok) {
            const text = await response.text();
            throw new Error(
              `Vertex rawPredict failed: ${response.status} ${text}`
            );
          }
          const json: unknown = await response.json();
          const data = parseAnthropicMessageResponse(json);
          return { type: "response", data };
        })
      );
    }

    if (stream !== false) {
      const url = buildUrl(model, true);
      promises.push(
        fetchVertex(url, body, token).then(async (response) => {
          if (!response.ok) {
            const text = await response.text();
            throw new Error(
              `Vertex streamRawPredict failed: ${response.status} ${text}`
            );
          }
          const text = await response.text();
          const chunks = parseStreamChunks(text);
          return { type: "streamingResponse", data: chunks };
        })
      );
    }

    const initialResults = await Promise.all(promises);

    for (const result_ of initialResults) {
      if (result_.type === "response") {
        result.response = result_.data;
      } else if (result_.type === "streamingResponse") {
        result.streamingResponse = result_.data;
      }
    }

    // Follow-up conversation
    if (
      result.response &&
      typeof result.response === "object" &&
      result.response !== null &&
      "content" in result.response &&
      Array.isArray(result.response.content)
    ) {
      const assistantMessage: Anthropic.MessageParam = {
        role: "assistant",
        content: result.response.content,
      };

      const followUpMessages: Anthropic.MessageParam[] = [
        ...payload.messages,
        assistantMessage,
      ];

      const assistantContent = Array.isArray(assistantMessage.content)
        ? assistantMessage.content
        : [assistantMessage.content];

      let hasToolCalls = false;
      for (const contentBlock of assistantContent) {
        if (
          typeof contentBlock === "object" &&
          contentBlock !== null &&
          "type" in contentBlock &&
          contentBlock.type === "tool_use" &&
          "id" in contentBlock &&
          typeof contentBlock.id === "string"
        ) {
          hasToolCalls = true;
          followUpMessages.push({
            role: "user",
            content: [
              {
                type: "tool_result",
                tool_use_id: contentBlock.id,
                content: "71 degrees",
              },
            ],
          });
        }
      }

      if (!hasToolCalls) {
        followUpMessages.push({
          role: "user",
          content: "What should I do next?",
        });
      }

      const followUpPayload: AnthropicMessageCreateParams = {
        ...payload,
        messages: followUpMessages,
      };

      const {
        model: followupModel,
        body: followupBody,
        bodyObj: followupBodyObj,
      } = buildVertexBody(followUpPayload);
      result.followupRequest = followupBodyObj;

      type FollowupResult =
        | { type: "followupResponse"; data: Anthropic.Messages.Message }
        | { type: "followupStreamingResponse"; data: Array<unknown> };

      const followupPromises: Promise<FollowupResult>[] = [];

      if (stream !== true) {
        const url = buildUrl(followupModel, false);
        followupPromises.push(
          fetchVertex(url, followupBody, token).then(async (response) => {
            if (!response.ok) {
              const text = await response.text();
              throw new Error(
                `Vertex followup rawPredict failed: ${response.status} ${text}`
              );
            }
            const json: unknown = await response.json();
            const data = parseAnthropicMessageResponse(json);
            return {
              type: "followupResponse",
              data,
            };
          })
        );
      }

      if (stream !== false) {
        const url = buildUrl(followupModel, true);
        followupPromises.push(
          fetchVertex(url, followupBody, token).then(async (response) => {
            if (!response.ok) {
              const text = await response.text();
              throw new Error(
                `Vertex followup streamRawPredict failed: ${response.status} ${text}`
              );
            }
            const text = await response.text();
            const chunks = parseStreamChunks(text);
            return { type: "followupStreamingResponse", data: chunks };
          })
        );
      }

      if (followupPromises.length > 0) {
        const followupResults = await Promise.all(followupPromises);

        for (const result_ of followupResults) {
          if (result_.type === "followupResponse") {
            result.followupResponse = result_.data;
          } else if (result_.type === "followupStreamingResponse") {
            result.followupStreamingResponse = result_.data;
          }
        }
      }
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const vertexAnthropicExecutor: ProviderExecutor<
  AnthropicMessageCreateParams | VertexAnthropicBody,
  Anthropic.Messages.Message,
  unknown
> = {
  name: "vertex-anthropic",
  cases: vertexAnthropicCases,
  execute: executeVertexAnthropic,
  ignoredFields: ["id", "content.*.text", "usage"],
};
