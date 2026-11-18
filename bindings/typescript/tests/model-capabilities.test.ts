import { expect, test } from "vitest";
import { supportsForcedToolChoice } from "../src/capabilities";

test.each([
  ["claude-sonnet-5-5", false],
  ["CLAUDE-OPUS-5.5", false],
  ["databricks-claude-sonnet-5-5", false],
  ["us.anthropic.claude-opus-5-5-v1:0", false],
  ["publishers/anthropic/models/claude-sonnet-5-5", false],
  ["claude-fable-5-1", false],
  ["claude-fable-5-2", false],
  ["claude-mythos-5-1", false],
  ["claude-mythos-5-2", false],
  ["claude-haiku-5-5", true],
  ["claude-fable-5", true],
  ["claude-opus-5", true],
  ["claude-sonnet-4-6", true],
  ["gpt-5-mini", true],
  ["claude-sonnet-5-6", true],
  ["claude-sonnet-5-10", true],
  ["not-claude-sonnet-5-5", true],
])("reports forced tool choice support for %s", (model, supported) => {
  expect(supportsForcedToolChoice(model)).toBe(supported);
});
