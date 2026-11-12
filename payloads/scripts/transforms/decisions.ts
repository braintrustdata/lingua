import { existsSync, mkdirSync, writeFileSync } from "fs";
import { dirname } from "path";
import {
  transform_decision_request,
  transform_decision_response,
  validate_decision_request,
  validate_decision_response,
} from "@braintrust/lingua-wasm/full";
import { allTestCases, getCaseNames, getCaseForProvider } from "../../cases";
import { OPENAI_DECISIONS_MODEL, TYPESAFE_MODEL } from "../../cases/models";
import { callTypeSafe } from "../providers/typesafe";
import { getResponsePath } from "./helpers";

export type DecisionFormat = "decisions" | "typesafe";

export interface DecisionTransformPair {
  source: DecisionFormat;
  target: DecisionFormat;
}

// Scorers send OpenAI Decisions requests; the gateway converts them for TypeSafe.
export const DECISION_TRANSFORM_PAIRS: DecisionTransformPair[] = [
  { source: "decisions", target: "typesafe" },
];

const TARGET_MODELS: Record<DecisionFormat, string> = {
  decisions: OPENAI_DECISIONS_MODEL,
  typesafe: TYPESAFE_MODEL,
};

const CALL_TARGET: Record<DecisionFormat, (body: string) => Promise<unknown>> =
  {
    decisions: () => {
      throw new Error("No decision transforms target OpenAI Decisions");
    },
    typesafe: (body) => callTypeSafe(body),
  };

interface DecisionTransformResult {
  data: unknown;
  json: string;
}

function isDecisionTransformResult(
  value: unknown
): value is DecisionTransformResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "data" in value &&
    "json" in value &&
    typeof value.json === "string"
  );
}

export function getDecisionTransformableCases(
  pair: DecisionTransformPair,
  filter?: string
): string[] {
  return getCaseNames(allTestCases).filter((caseName) => {
    if (filter && !caseName.includes(filter)) return false;
    return (
      getCaseForProvider(allTestCases, caseName, pair.source) != null &&
      !allTestCases[caseName]?.expect
    );
  });
}

// The source request as the gateway forwards it: rewritten to the target's model.
export function getDecisionSourceRequest(
  pair: DecisionTransformPair,
  caseName: string
): string {
  const input = getCaseForProvider(allTestCases, caseName, pair.source);
  if (!input) {
    throw new Error(`Case ${caseName} has no ${pair.source} request`);
  }
  return JSON.stringify({ ...input, model: TARGET_MODELS[pair.target] });
}

/** The target request lingua produces, exactly as it would be sent. */
export function transformDecisionRequest(
  pair: DecisionTransformPair,
  caseName: string
): string {
  const result: unknown = transform_decision_request(
    getDecisionSourceRequest(pair, caseName),
    pair.source,
    pair.target
  );
  if (!isDecisionTransformResult(result)) {
    throw new Error("transform_decision_request returned no JSON payload");
  }
  validate_decision_request(result.json, pair.target);
  return result.json;
}

/** Convert a captured target response back to the source format. */
export function transformDecisionResponse(
  pair: DecisionTransformPair,
  caseName: string,
  responseJson: string
): unknown {
  validate_decision_response(responseJson, pair.target);
  const result: unknown = transform_decision_response(
    getDecisionSourceRequest(pair, caseName),
    responseJson,
    pair.source,
    pair.target
  );
  if (typeof result !== "object" || result === null || !("data" in result)) {
    throw new Error("transform_decision_response returned no data");
  }
  validate_decision_response(JSON.stringify(result.data), pair.source);
  return result.data;
}

export async function captureDecisionTransforms(
  filter?: string,
  force?: boolean,
  requestedPair?: { source: string; target: string },
  requestedCases?: string[],
  requestedCaptureProviders?: string[]
): Promise<{ captured: number; skipped: number; failed: number }> {
  let captured = 0,
    skipped = 0,
    failed = 0;

  for (const pair of DECISION_TRANSFORM_PAIRS) {
    if (
      requestedCaptureProviders &&
      !requestedCaptureProviders.includes(pair.target)
    ) {
      continue;
    }
    if (
      requestedPair &&
      (pair.source !== requestedPair.source ||
        pair.target !== requestedPair.target)
    ) {
      continue;
    }

    const requested = requestedCases ? new Set(requestedCases) : undefined;
    const cases = getDecisionTransformableCases(pair, filter).filter(
      (caseName) => !requested || requested.has(caseName)
    );

    await Promise.all(
      cases.map(async (caseName) => {
        const responsePath = getResponsePath(
          pair.source,
          pair.target,
          caseName
        );
        mkdirSync(dirname(responsePath), { recursive: true });
        if (existsSync(responsePath) && !force) {
          skipped++;
          return;
        }

        let request: string | undefined;
        try {
          request = transformDecisionRequest(pair, caseName);
          const responseJson = JSON.stringify(
            await CALL_TARGET[pair.target](request),
            null,
            2
          );
          validate_decision_response(responseJson, pair.target);
          writeFileSync(responsePath, responseJson);
          console.log(`✅ ${pair.source} → ${pair.target} / ${caseName}`);
          captured++;
        } catch (e) {
          writeFileSync(
            responsePath,
            JSON.stringify(
              {
                ...(request ? { request: JSON.parse(request) } : {}),
                error: e instanceof Error ? e.message : String(e),
              },
              null,
              2
            )
          );
          console.error(
            `❌ ${pair.source} → ${pair.target} / ${caseName}: ${e}`
          );
          failed++;
        }
      })
    );
  }

  return { captured, skipped, failed };
}
