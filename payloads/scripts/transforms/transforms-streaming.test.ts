import { describe, test, expect } from "vitest";
import { existsSync, readFileSync } from "fs";
import { join } from "path";
import { transform_stream_chunk } from "@braintrust/lingua-wasm";
import {
  STREAMING_PAIRS,
  TRANSFORMS_DIR,
  getStreamingTransformableCases,
  getStreamingResponsePath,
} from "./helpers";

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

      if (!existsSync(streamingPath)) {
        console.warn(
          `Missing streaming capture: ${pair.source} → ${pair.target} / ${caseName}`
        );
      }

      test.skipIf(!existsSync(streamingPath))(caseName, () => {
        const pairKey = `${pair.source}_to_${pair.target}_streaming`;

        try {
          const rawChunks = JSON.parse(readFileSync(streamingPath, "utf-8"));
          const transformed = rawChunks.map((chunk: unknown) =>
            transform_stream_chunk(JSON.stringify(chunk), pair.wasmSource)
          );
          expect(transformed).toMatchSnapshot("streaming");
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
