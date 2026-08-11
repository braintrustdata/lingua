import { CaptureResult, ExecuteOptions, ProviderExecutor } from "../types";
import {
  allTestCases,
  getCaseNames,
  getCaseForProvider,
  hasExpectation,
  type GoogleGenerateContentRequest,
} from "../../cases";
import { parseGoogleSseStream } from "../transforms/helpers";
import { getVertexAccessToken } from "./vertex-anthropic";

type VertexGoogleResponse = Record<string, unknown>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseVertexGoogleResponse(value: unknown): VertexGoogleResponse {
  if (!isRecord(value)) {
    throw new Error("Invalid Vertex Google response: expected object");
  }
  return value;
}

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

function vertexGoogleUrl(model: string, stream: boolean): string {
  const project = process.env.VERTEX_PROJECT;
  if (!project) {
    throw new Error("VERTEX_PROJECT environment variable is required");
  }
  const location = process.env.VERTEX_LOCATION ?? "us-east5";
  const method = stream ? "streamGenerateContent?alt=sse" : "generateContent";
  return `https://${location}-aiplatform.googleapis.com/v1/projects/${project}/locations/${location}/${model}:${method}`;
}

async function requestVertexGoogle(
  model: string,
  payload: GoogleGenerateContentRequest,
  token: string,
  stream: boolean
): Promise<Response> {
  const { model: _model, ...body } = payload;
  return fetch(vertexGoogleUrl(model, stream), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(body),
  });
}

export async function executeVertexGoogle(
  _caseName: string,
  payload: GoogleGenerateContentRequest,
  options?: ExecuteOptions
): Promise<
  CaptureResult<
    GoogleGenerateContentRequest,
    VertexGoogleResponse,
    VertexGoogleResponse
  >
> {
  const { stream } = options ?? {};
  const model = payload.model;
  if (!model) {
    throw new Error("Vertex Google requests require a model");
  }

  const result: CaptureResult<
    GoogleGenerateContentRequest,
    VertexGoogleResponse,
    VertexGoogleResponse
  > = { request: payload };

  try {
    const token = await getVertexAccessToken();
    if (stream !== true) {
      const response = await requestVertexGoogle(model, payload, token, false);
      if (!response.ok) {
        throw new Error(
          `Vertex generateContent failed: ${response.status} ${await response.text()}`
        );
      }
      const json: unknown = await response.json();
      result.response = parseVertexGoogleResponse(json);
    }
    if (stream !== false) {
      const response = await requestVertexGoogle(model, payload, token, true);
      if (!response.ok) {
        throw new Error(
          `Vertex streamGenerateContent failed: ${response.status} ${await response.text()}`
        );
      }
      result.streamingResponse =
        await parseGoogleSseStream<VertexGoogleResponse>(response);
    }
  } catch (error) {
    result.error = String(error);
  }

  return result;
}

export const vertexGoogleExecutor: ProviderExecutor<
  GoogleGenerateContentRequest,
  VertexGoogleResponse,
  VertexGoogleResponse
> = {
  name: "vertex-google",
  cases: vertexGoogleCases,
  execute: executeVertexGoogle,
};
