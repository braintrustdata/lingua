import { expect, test } from "vitest";
import { supportsForcedToolChoice } from "../src/index";

test.each([
  ["claude-sonnet-5-5", false],
  ["us.anthropic.claude-opus-5-5-v1:0", false],
  ["publishers/anthropic/models/claude-sonnet-5-5", false],
  ["claude-sonnet-4-6", true],
  ["gpt-5-mini", true],
])("reports forced tool choice support for %s", (model, supported) => {
  expect(supportsForcedToolChoice(model)).toBe(supported);
});
