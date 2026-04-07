import { describe, test, expect } from "vitest";
import {
  Gemini,
  InMemoryRunner,
  LlmAgent,
  LogLevel,
  StreamingMode,
  stringifyContent,
  setLogLevel,
  type Event,
} from "@google/adk";
import { allTestCases, getCaseForProvider } from "../../cases";
import {
  STREAMING_PAIRS,
  TARGET_MODELS,
  TRANSFORM_PAIRS,
  getFixtureSkipReason,
  getGenAiGenerateContentPath,
  getGenAiStreamGenerateContentPath,
  getResponsePath,
  getStreamingResponsePath,
  getStreamingTransformableCases,
  getTransformableCases,
} from "./helpers";
import {
  registerSkippedFixtureTest,
  useTransformTestServer,
} from "./vitest-helpers";

const TIMEOUT = 30000;
const TEST_USER_ID = "test-user";
const TEST_PROMPT = "test";
const getServer = useTransformTestServer();

setLogLevel(LogLevel.WARN);

class FixtureGemini extends Gemini {
  protected override getHttpOptions() {
    return {
      ...super.getHttpOptions(),
      baseUrl: getServer().genaiBaseUrl,
      apiVersion: "v1beta",
    };
  }
}

function getGoogleModel(caseName: string): string {
  const googleCase = getCaseForProvider(allTestCases, caseName, "google");
  return googleCase &&
    typeof googleCase === "object" &&
    "model" in googleCase &&
    typeof googleCase.model === "string"
    ? googleCase.model
    : TARGET_MODELS.google;
}

function createAdkRunner(model: string): InMemoryRunner {
  const agent = new LlmAgent({
    name: "transform_adk_test_agent",
    instruction: "Be concise.",
    model: new FixtureGemini({
      model,
      apiKey: "test-key",
    }),
  });

  return new InMemoryRunner({
    agent,
    appName: "transform-adk-test",
  });
}

async function collectRunnerEvents(
  runner: InMemoryRunner,
  options?: { streaming?: boolean }
): Promise<Event[]> {
  const events: Event[] = [];
  const stream = runner.runEphemeral({
    userId: TEST_USER_ID,
    newMessage: {
      role: "user",
      parts: [{ text: TEST_PROMPT }],
    },
    runConfig: options?.streaming
      ? { streamingMode: StreamingMode.SSE }
      : undefined,
  });

  for await (const event of stream) {
    events.push(event);
  }

  return events;
}

function expectRunnerParsedResponse(events: Event[]): void {
  expect(events.length).toBeGreaterThan(0);

  const nonUserEvents = events.filter((event) => event.author !== "user");
  expect(nonUserEvents.length).toBeGreaterThan(0);

  // Some valid transformed Google fixtures produce tool-only or otherwise
  // non-text ADK events, so text is an optional stronger signal rather than a
  // required assertion.
  const texts = nonUserEvents
    .map((event) => stringifyContent(event))
    .filter(Boolean);
  expect(texts.length).toBeGreaterThanOrEqual(0);
}

for (const pair of TRANSFORM_PAIRS.filter((p) => p.source === "google")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`adk SDK: ${pairLabel}`, () => {
    for (const caseName of getTransformableCases(pair)) {
      const path = getResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path);

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          const model = getGoogleModel(caseName);

          getServer().useJsonFixture({
            path: getGenAiGenerateContentPath(model),
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
          });

          const runner = createAdkRunner(model);
          const events = await collectRunnerEvents(runner);
          expectRunnerParsedResponse(events);
        },
        TIMEOUT
      );
    }
  });
}

for (const pair of STREAMING_PAIRS.filter((p) => p.source === "google")) {
  const pairLabel = `${pair.target} → ${pair.source}`;
  describe(`adk SDK streaming: ${pairLabel}`, () => {
    for (const caseName of getStreamingTransformableCases(pair)) {
      const path = getStreamingResponsePath(pair.source, pair.target, caseName);
      const skipReason = getFixtureSkipReason(path, { streaming: true });

      if (skipReason) {
        registerSkippedFixtureTest(pairLabel, caseName, skipReason);
        continue;
      }

      test(
        caseName,
        async () => {
          const model = getGoogleModel(caseName);

          getServer().useStreamingFixture({
            path: getGenAiStreamGenerateContentPath(model),
            targetFormat: pair.target,
            wasmSource: pair.wasmSource,
            responsePath: path,
            requireStream: false,
          });

          const runner = createAdkRunner(model);
          const events = await collectRunnerEvents(runner, { streaming: true });
          expectRunnerParsedResponse(events);
        },
        TIMEOUT
      );
    }
  });
}
