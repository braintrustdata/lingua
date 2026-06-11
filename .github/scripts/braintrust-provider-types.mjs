#!/usr/bin/env node

import { appendFileSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

function requireEnv(name) {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

function optionalEnv(name) {
  return process.env[name] || undefined;
}

function loadBraintrust() {
  try {
    return require("braintrust");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to load the Braintrust SDK: ${message}`);
  }
}

async function flushBraintrust(braintrust) {
  if (typeof braintrust.flush === "function") {
    await braintrust.flush();
  }
}

function writeGithubOutput(values) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) {
    return;
  }

  for (const [key, value] of Object.entries(values)) {
    appendFileSync(outputPath, `${key}=${value ?? ""}\n`);
  }
}

function writeGithubOutputValue(key, value) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) {
    return;
  }

  const text = value == null ? "" : String(value);
  if (!text.includes("\n")) {
    appendFileSync(outputPath, `${key}=${text}\n`);
    return;
  }

  const delimiter = `EOF_${key}_${Date.now()}`;
  appendFileSync(outputPath, `${key}<<${delimiter}\n${text}\n${delimiter}\n`);
}

function workflowMetadata(extra = {}) {
  return {
    repository: optionalEnv("GITHUB_REPOSITORY"),
    workflow: optionalEnv("GITHUB_WORKFLOW"),
    job: optionalEnv("GITHUB_JOB"),
    run_id: optionalEnv("GITHUB_RUN_ID"),
    run_attempt: optionalEnv("GITHUB_RUN_ATTEMPT"),
    ref: optionalEnv("GITHUB_REF"),
    sha: optionalEnv("GITHUB_SHA"),
    actor: optionalEnv("GITHUB_ACTOR"),
    provider: optionalEnv("PROVIDER"),
    ...extra,
  };
}

async function createWorkflowTrace() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const projectName =
    optionalEnv("BRAINTRUST_PROJECT") || "lingua-provider-type-updates";
  const provider = requireEnv("PROVIDER");
  const logger = braintrust.initLogger({ projectName });
  const span = logger.startSpan({
    name: `Update ${provider} provider types`,
  });
  const spanId = span.id || span.spanId;
  const rootSpanId = span.rootSpanId || span.root_span_id || spanId;

  span.log({
    input: {
      provider,
      event: optionalEnv("GITHUB_EVENT_NAME"),
      run_id: optionalEnv("GITHUB_RUN_ID"),
      run_attempt: optionalEnv("GITHUB_RUN_ATTEMPT"),
    },
    metadata: workflowMetadata({
      braintrust_project: projectName,
      root_span_id: rootSpanId,
      span_id: spanId,
    }),
  });
  span.end();
  await flushBraintrust(braintrust);

  writeGithubOutput({
    project: projectName,
    root_span_id: rootSpanId,
    span_id: spanId,
  });
}

function emitPrMetadata() {
  const metadata = {
    version: 1,
    kind: "provider-type-update",
    project: requireEnv("BRAINTRUST_PROJECT"),
    root_span_id: requireEnv("BRAINTRUST_ROOT_SPAN_ID"),
    span_id: requireEnv("BRAINTRUST_SPAN_ID"),
    provider: requireEnv("PROVIDER"),
    repository: requireEnv("GITHUB_REPOSITORY"),
    run_id: requireEnv("GITHUB_RUN_ID"),
    run_attempt: requireEnv("GITHUB_RUN_ATTEMPT"),
    workflow: requireEnv("GITHUB_WORKFLOW"),
    sha: requireEnv("GITHUB_SHA"),
  };

  console.log("<!-- braintrust-provider-type-update");
  console.log(JSON.stringify(metadata));
  console.log("-->");
}

function extractHiddenMetadata(body) {
  const match = body.match(
    /<!--\s*braintrust-provider-type-update\s*\n([\s\S]*?)\n-->/,
  );
  if (!match) {
    return undefined;
  }

  return JSON.parse(match[1]);
}

function extractFeedbackEvent() {
  const eventPath = requireEnv("GITHUB_EVENT_PATH");
  const event = JSON.parse(readFileSync(eventPath, "utf8"));
  const command = (event.comment?.body || "").trim();
  const commandMatch = command.match(/^\/bt\s+(good|bad)\s*$/i);
  const isPullRequest = Boolean(event.issue?.pull_request);
  const labels = event.issue?.labels || [];
  const labelNames = labels.map((label) => label.name);
  const allowedAssociations = new Set([
    "COLLABORATOR",
    "MEMBER",
    "OWNER",
  ]);
  const authorAssociation = event.comment?.author_association;

  if (!commandMatch || !isPullRequest || !labelNames.includes("auto-sync")) {
    writeGithubOutput({
      should_log: "false",
    });
    return;
  }

  if (!allowedAssociations.has(authorAssociation)) {
    writeGithubOutput({
      should_log: "false",
      reason: `Ignored /bt feedback from ${authorAssociation || "unknown"} author`,
    });
    return;
  }

  const metadata = extractHiddenMetadata(event.issue?.body || "");
  if (!metadata?.root_span_id || !metadata?.span_id || !metadata?.project) {
    writeGithubOutput({
      should_log: "false",
      reason: "Could not find Braintrust metadata in the PR body",
    });
    return;
  }

  writeGithubOutputValue("metadata", JSON.stringify(metadata));
  writeGithubOutput({
    should_log: "true",
    rating: commandMatch[1].toLowerCase(),
    command,
    comment_id: event.comment.id,
    comment_url: event.comment.html_url,
    comment_author: event.comment.user?.login,
    pr_number: event.issue.number,
    pr_url: event.issue.html_url,
  });
}

async function logFeedback() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const metadata = JSON.parse(requireEnv("BRAINTRUST_PR_METADATA"));
  const rating = requireEnv("BT_RATING");
  const score = rating === "good" ? 1 : 0;
  const projectName = metadata.project;
  const parentSpanId = metadata.span_id || metadata.root_span_id;
  const logger = braintrust.initLogger({ projectName });
  const feedbackMetadata = workflowMetadata({
    provider: metadata.provider,
    rating,
    feedback_source: "github_issue_comment",
    feedback_command: optionalEnv("BT_COMMAND"),
    feedback_comment_id: optionalEnv("BT_COMMENT_ID"),
    feedback_comment_url: optionalEnv("BT_COMMENT_URL"),
    feedback_author: optionalEnv("BT_COMMENT_AUTHOR"),
    pr_number: optionalEnv("BT_PR_NUMBER"),
    pr_url: optionalEnv("BT_PR_URL"),
    target_span_id: parentSpanId,
    target_root_span_id: metadata.root_span_id,
    target_run_id: metadata.run_id,
    target_run_attempt: metadata.run_attempt,
  });

  if (typeof logger.traced === "function") {
    await logger.traced(
      async (span) => {
        span.log({
          scores: {
            github_pr_feedback: score,
          },
          comment: optionalEnv("BT_COMMENT_BODY"),
          metadata: feedbackMetadata,
        });
      },
      {
        name: "github_pr_feedback",
        parent: parentSpanId,
      },
    );
  } else {
    logger.log({
      id: `github-pr-feedback-${optionalEnv("BT_COMMENT_ID")}`,
      input: {
        command: optionalEnv("BT_COMMAND"),
        pr_number: optionalEnv("BT_PR_NUMBER"),
      },
      output: {
        rating,
        target_span_id: parentSpanId,
        target_root_span_id: metadata.root_span_id,
      },
      scores: {
        github_pr_feedback: score,
      },
      metadata: feedbackMetadata,
    });
  }

  await flushBraintrust(braintrust);
}

const command = process.argv[2];

try {
  if (command === "create-workflow-trace") {
    await createWorkflowTrace();
  } else if (command === "emit-pr-metadata") {
    emitPrMetadata();
  } else if (command === "extract-feedback-event") {
    extractFeedbackEvent();
  } else if (command === "log-feedback") {
    await logFeedback();
  } else {
    throw new Error(
      "Usage: braintrust-provider-types.mjs create-workflow-trace|emit-pr-metadata|extract-feedback-event|log-feedback",
    );
  }
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(message);
  process.exit(1);
}
