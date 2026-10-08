import { readFileSync } from "fs";
import { join } from "path";
import Anthropic from "@anthropic-ai/sdk";
import { describe, expect, test } from "vitest";
import {
  validate_anthropic_request,
  validate_anthropic_response,
} from "@braintrust/lingua-wasm";

const cases = [
  "anthropicSonnet5AdaptiveThinkingDisplaySummarizedParam",
  "anthropicOpus5AdaptiveThinkingDisplaySummarizedParam",
];

for (const caseName of cases) {
  describe(caseName, () => {
    for (const provider of ["anthropic", "bedrock-anthropic"]) {
      test(`${provider} capture includes nonempty summarized thinking`, () => {
        const snapshotDir = join(
          __dirname,
          "../../snapshots",
          caseName,
          provider
        );
        const responseJson = readFileSync(
          join(snapshotDir, "response.json"),
          "utf-8"
        );
        if (provider === "anthropic") {
          validate_anthropic_request(
            readFileSync(join(snapshotDir, "request.json"), "utf-8")
          );
        }
        validate_anthropic_response(responseJson);
        const response: Anthropic.Messages.Message = JSON.parse(responseJson);
        expect(
          response.content.some(
            (block) =>
              block.type === "thinking" && block.thinking.trim().length > 0
          )
        ).toBe(true);

        const stream: Anthropic.Messages.MessageStreamEvent[] = JSON.parse(
          readFileSync(join(snapshotDir, "response-streaming.json"), "utf-8")
        );
        expect(
          stream.some(
            (event) =>
              event.type === "content_block_delta" &&
              event.delta.type === "thinking_delta" &&
              event.delta.thinking.trim().length > 0
          )
        ).toBe(true);
        expect(stream.at(-1)?.type).toBe("message_stop");
      });
    }
  });
}
