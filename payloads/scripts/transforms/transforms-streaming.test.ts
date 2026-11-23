import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import {
  STREAMING_PAIRS,
  TRANSFORMS_DIR,
  buildSse,
  getFixtureSkipReason,
  getStreamingTransformableCases,
  getStreamingResponsePath,
} from "./helpers";
import { registerSkippedFixtureTest } from "./vitest-helpers";

const ERRORS_PATH = join(TRANSFORMS_DIR, "transform_errors.json");
const transformErrors: Record<string, Record<string, string>> = existsSync(
  ERRORS_PATH
)
  ? JSON.parse(readFileSync(ERRORS_PATH, "utf-8"))
  : {};

for (const pair of STREAMING_PAIRS) {
  describe(`streaming: ${pair.source} → ${pair.target}`, () => {
    const cases = getStreamingTransformableCases(pair);

    for (const caseName of cases) {
      const streamingPath = getStreamingResponsePath(
        pair.source,
        pair.target,
        caseName
      );
      const pairLabel = `${pair.source} → ${pair.target}`;
      const skipReason = getFixtureSkipReason(streamingPath, {
        streaming: true,
      });

      if (!existsSync(streamingPath)) {
        console.warn(
          `Missing streaming capture: ${pair.source} → ${pair.target} / ${caseName}`
        );
      }

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(caseName, () => {
        const pairKey = `${pair.source}_to_${pair.target}_streaming`;

        try {
          const rawChunks = JSON.parse(readFileSync(streamingPath, "utf-8"));
          const sse = buildSse(rawChunks, pair.wasmSource);
          const normalizedSse = sse.replace(
            /^data: (.+)$/gm,
            (line: string, data: string) => {
              if (data === "[DONE]") {
                return line;
              }
              const payload: unknown = JSON.parse(data);
              return `data: ${JSON.stringify(
                payload,
                (key: string, value: unknown) => {
                  if (
                    key === "properties" ||
                    typeof value !== "object" ||
                    value === null ||
                    Array.isArray(value)
                  ) {
                    return value;
                  }
                  return Object.fromEntries(
                    Object.entries(value).sort(([left], [right]) =>
                      left < right ? -1 : left > right ? 1 : 0
                    )
                  );
                }
              )}`;
            }
          );
          expect(normalizedSse).toMatchSnapshot("streaming");
        } catch (e) {
          const errorReason = transformErrors[pairKey]?.[caseName];
          if (errorReason) {
            return;
          }
          throw e;
        }
      });
    }
  });
}
