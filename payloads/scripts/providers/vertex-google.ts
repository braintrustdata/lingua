import { readFileSync } from "fs";
import { resolve, isAbsolute } from "path";
import { createSign } from "crypto";
import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type GoogleGenerateContentRequest,
  VERTEX_GOOGLE_MODEL,
} from "../../cases";

export const vertexGoogleCases: Record<string, GoogleGenerateContentRequest> =
  {};

getCaseNames(allTestCases).forEach((caseName) => {
  if (hasExpectation(allTestCases, caseName)) {
    return;
  }
  const caseData = getCaseForProvider(allTestCases, caseName, "vertex-google");
  if (caseData) {
    vertexGoogleCases[caseName] = caseData;
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

function buildUrl(model: string, stream: boolean): string {
  const project = process.env.VERTEX_PROJECT;
  if (!project) {
    throw new Error("VERTEX_PROJECT environment variable is required");
  }
  const location = process.env.VERTEX_LOCATION ?? "us-central1";
  const method = stream ? "streamGenerateContent?alt=sse" : "generateContent";
  return `https://${location}-aiplatform.googleapis.com/v1/projects/${project}/locations/${location}/${model}:${method}`;
}

// Response type matching Google's GenerateContent response
type GenerateContentResponse = {
  candidates?: Array<{
    content?: {
      role?: string;
      parts?: Array<{
        text?: string;
        functionCall?: { name: string; args?: Record<string, unknown> };
        [key: string]: unknown;
      }>;
    };
    [key: string]: unknown;
  }>;
  [key: string]: unknown;
};

type Content = {
  role: string;
  parts: Array<Record<string, unknown>>;
};

async function parseJsonResponse(
  response: Response
): Promise<GenerateContentResponse> {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- response.json() returns unknown
  return (await response.json()) as GenerateContentResponse;
}

async function parseSSEStream(
  response: Response
): Promise<GenerateContentResponse[]> {
  const chunks: GenerateContentResponse[] = [];
  const text = await response.text();
  const lines = text.split("\n");
  for (const line of lines) {
    if (line.startsWith("data: ")) {
      const jsonStr = line.slice(6);
      if (jsonStr.trim()) {
        try {
          chunks.push(JSON.parse(jsonStr));
        } catch {
          // skip malformed JSON
        }
      }
    }
  }
  return chunks;
}

function toContent(content: { role?: string; parts?: unknown[] }): Content {
  const parts: Array<Record<string, unknown>> = [];
  for (const part of content.parts ?? []) {
    if (part && typeof part === "object") {
      // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- validated above
      parts.push(part as Record<string, unknown>);
    }
  }
  return { role: content.role ?? "user", parts };
}

async function vertexGoogleRequest(
  model: string,
  payload: GoogleGenerateContentRequest,
  token: string,
  stream: boolean
): Promise<Response> {
  const url = buildUrl(model, stream);
  const { model: _model, ...body } = payload;

  return fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(body),
  });
}

type ParallelResult =
  | { type: "response"; data: GenerateContentResponse }
  | { type: "streamingResponse"; data: GenerateContentResponse[] };

export async function executeVertexGoogle(
  _caseName: string,
  payload: GoogleGenerateContentRequest,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    GoogleGenerateContentRequest,
    GenerateContentResponse,
    GenerateContentResponse
  >
> {
  const { stream } = options ?? {};
  const token = await getAccessToken();
  const model = payload.model ?? VERTEX_GOOGLE_MODEL;

  const result: CaptureResult<
    GoogleGenerateContentRequest,
    GenerateContentResponse,
    GenerateContentResponse
  > = { request: payload };

  try {
    const promises: Promise<ParallelResult>[] = [];

    if (stream !== true) {
      promises.push(
        (async () => {
          const response = await vertexGoogleRequest(
            model,
            payload,
            token,
            false
          );
          if (!response.ok) {
            const text = await response.text();
            throw new Error(`ApiError: ${text}`);
          }
          const data = await parseJsonResponse(response);
          return { type: "response" as const, data };
        })()
      );
    }

    if (stream !== false) {
      promises.push(
        (async () => {
          const response = await vertexGoogleRequest(
            model,
            payload,
            token,
            true
          );
          if (!response.ok) {
            const text = await response.text();
            throw new Error(`ApiError: ${text}`);
          }
          const chunks = await parseSSEStream(response);
          return { type: "streamingResponse" as const, data: chunks };
        })()
      );
    }

    const initialResults = await Promise.all(promises);

    for (const r of initialResults) {
      if (r.type === "response") {
        result.response = r.data;
      } else if (r.type === "streamingResponse") {
        result.streamingResponse = r.data;
      }
    }

    if (result.response) {
      const assistantContent = result.response.candidates?.[0]?.content;

      if (assistantContent) {
        const followUpContents: Content[] = [
          ...payload.contents.map(toContent),
          toContent(assistantContent),
        ];

        const assistantParts = assistantContent.parts ?? [];
        let hasToolCalls = false;

        for (const part of assistantParts) {
          if ("functionCall" in part && part.functionCall) {
            hasToolCalls = true;
            followUpContents.push({
              role: "user",
              parts: [
                {
                  functionResponse: {
                    name: part.functionCall.name,
                    response: { temperature: "71 degrees" },
                  },
                },
              ],
            });
          }
        }

        if (!hasToolCalls) {
          followUpContents.push({
            role: "user",
            parts: [{ text: "What should I do next?" }],
          });
        }

        const followUpPayload: GoogleGenerateContentRequest = {
          ...payload,
          contents: followUpContents,
        };

        result.followupRequest = followUpPayload;

        type FollowupResult =
          | { type: "followupResponse"; data: GenerateContentResponse }
          | {
              type: "followupStreamingResponse";
              data: GenerateContentResponse[];
            };

        const followupPromises: Promise<FollowupResult>[] = [];

        if (stream !== true) {
          followupPromises.push(
            (async () => {
              const response = await vertexGoogleRequest(
                model,
                followUpPayload,
                token,
                false
              );
              if (!response.ok) {
                const text = await response.text();
                throw new Error(`ApiError: ${text}`);
              }
              const data = await parseJsonResponse(response);
              return { type: "followupResponse" as const, data };
            })()
          );
        }

        if (stream !== false) {
          followupPromises.push(
            (async () => {
              const response = await vertexGoogleRequest(
                model,
                followUpPayload,
                token,
                true
              );
              if (!response.ok) {
                const text = await response.text();
                throw new Error(`ApiError: ${text}`);
              }
              const chunks = await parseSSEStream(response);
              return {
                type: "followupStreamingResponse" as const,
                data: chunks,
              };
            })()
          );
        }

        if (followupPromises.length > 0) {
          const followupResults = await Promise.all(followupPromises);
          for (const r of followupResults) {
            if (r.type === "followupResponse") {
              result.followupResponse = r.data;
            } else if (r.type === "followupStreamingResponse") {
              result.followupStreamingResponse = r.data;
            }
          }
        }
      }
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const vertexGoogleExecutor: ProviderExecutor<
  GoogleGenerateContentRequest,
  GenerateContentResponse,
  GenerateContentResponse
> = {
  name: "vertex-google",
  cases: vertexGoogleCases,
  execute: executeVertexGoogle,
};
