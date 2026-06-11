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
  const spanId = span.spanId || span.id;
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

function isCodexBot(user) {
  const login = user?.login || "";
  return user?.type === "Bot" && /codex/i.test(login);
}

async function githubApi(path) {
  const token = requireEnv("GITHUB_TOKEN");
  const response = await fetch(`https://api.github.com${path}`, {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "lingua-provider-type-feedback",
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`GitHub API request failed (${response.status}): ${body}`);
  }

  return response.json();
}

async function githubApiPages(path) {
  const results = [];
  let page = 1;

  while (true) {
    const separator = path.includes("?") ? "&" : "?";
    const pageResults = await githubApi(
      `${path}${separator}per_page=100&page=${page}`,
    );
    results.push(...pageResults);

    if (pageResults.length < 100) {
      return results;
    }

    page += 1;
  }
}

function formatReviewComment(comment) {
  const location = [
    comment.path,
    comment.line || comment.original_line || comment.position,
  ]
    .filter(Boolean)
    .join(":");
  const prefix = location ? `${location}\n` : "";
  const url = comment.html_url ? `\n${comment.html_url}` : "";
  return `${prefix}${comment.body || ""}${url}`.trim();
}

async function extractCodexReviewEvent() {
  const eventPath = requireEnv("GITHUB_EVENT_PATH");
  const event = JSON.parse(readFileSync(eventPath, "utf8"));
  const review = event.review;
  const pullRequest = event.pull_request;
  const labels = pullRequest?.labels || [];
  const labelNames = labels.map((label) => label.name);

  if (!review || !pullRequest || !labelNames.includes("auto-sync")) {
    writeGithubOutput({
      should_log: "false",
    });
    return;
  }

  if (!isCodexBot(review.user)) {
    writeGithubOutput({
      should_log: "false",
      reason: `Ignored review from ${review.user?.login || "unknown"}`,
    });
    return;
  }

  const metadata = extractHiddenMetadata(pullRequest.body || "");
  if (!metadata?.root_span_id || !metadata?.span_id || !metadata?.project) {
    writeGithubOutput({
      should_log: "false",
      reason: "Could not find Braintrust metadata in the PR body",
    });
    return;
  }

  const [owner, repo] = requireEnv("GITHUB_REPOSITORY").split("/");
  const comments = await githubApiPages(
    `/repos/${owner}/${repo}/pulls/${pullRequest.number}/comments`,
  );
  const reviewComments = comments
    .filter((comment) => comment.pull_request_review_id === review.id)
    .map(formatReviewComment)
    .filter(Boolean);
  const reviewBody = (review.body || "").trim();
  const output = [reviewBody, ...reviewComments]
    .filter(Boolean)
    .join("\n\n---\n\n");

  if (!output) {
    writeGithubOutput({
      should_log: "false",
      reason: "Codex review had no body or inline comments to log",
    });
    return;
  }

  writeGithubOutputValue("metadata", JSON.stringify(metadata));
  writeGithubOutputValue("review_output", output);
  writeGithubOutput({
    should_log: "true",
    review_id: review.id,
    review_url: review.html_url,
    review_author: review.user?.login,
    review_state: review.state,
    pr_number: pullRequest.number,
    pr_url: pullRequest.html_url,
    inline_comment_count: reviewComments.length,
  });
}

function targetParentSpanIds(metadata) {
  const parentSpanId = metadata.span_id || metadata.root_span_id;
  if (!parentSpanId || !metadata.root_span_id) {
    throw new Error(
      "Braintrust PR metadata must include root_span_id and span_id",
    );
  }

  return {
    spanId: parentSpanId,
    rootSpanId: metadata.root_span_id,
  };
}

async function logFeedback() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const metadata = JSON.parse(requireEnv("BRAINTRUST_PR_METADATA"));
  const rating = requireEnv("BT_RATING");
  const score = rating === "good" ? 1 : 0;
  const projectName = metadata.project;
  const parentSpanIds = targetParentSpanIds(metadata);
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
    target_span_id: parentSpanIds.spanId,
    target_root_span_id: metadata.root_span_id,
    target_run_id: metadata.run_id,
    target_run_attempt: metadata.run_attempt,
  });

  await logger.traced(
    async (span) => {
      span.log({
        input: {
          command: optionalEnv("BT_COMMAND"),
          pr_number: optionalEnv("BT_PR_NUMBER"),
        },
        output: {
          rating,
          comment: optionalEnv("BT_COMMENT_BODY"),
        },
        scores: {
          github_pr_feedback: score,
        },
        metadata: feedbackMetadata,
      });
    },
    {
      name: "github_pr_feedback",
      parentSpanIds,
    },
  );

  await flushBraintrust(braintrust);
}

async function logCodexReview() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const metadata = JSON.parse(requireEnv("BRAINTRUST_PR_METADATA"));
  const parentSpanIds = targetParentSpanIds(metadata);
  const logger = braintrust.initLogger({ projectName: metadata.project });

  await logger.traced(
    async (span) => {
      span.log({
        input: {
          review_id: optionalEnv("BT_REVIEW_ID"),
          pr_number: optionalEnv("BT_PR_NUMBER"),
        },
        output: {
          review: requireEnv("BT_REVIEW_OUTPUT"),
        },
        scores: {
          github_codex_review: 0,
        },
        metadata: workflowMetadata({
          provider: metadata.provider,
          feedback_source: "github_pull_request_review",
          feedback_actor: "codex",
          review_id: optionalEnv("BT_REVIEW_ID"),
          review_url: optionalEnv("BT_REVIEW_URL"),
          review_author: optionalEnv("BT_REVIEW_AUTHOR"),
          review_state: optionalEnv("BT_REVIEW_STATE"),
          inline_comment_count: optionalEnv("BT_INLINE_COMMENT_COUNT"),
          pr_number: optionalEnv("BT_PR_NUMBER"),
          pr_url: optionalEnv("BT_PR_URL"),
          target_span_id: parentSpanIds.spanId,
          target_root_span_id: metadata.root_span_id,
          target_run_id: metadata.run_id,
          target_run_attempt: metadata.run_attempt,
        }),
      });
    },
    {
      name: "github_codex_review",
      parentSpanIds,
    },
  );

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
  } else if (command === "extract-codex-review-event") {
    await extractCodexReviewEvent();
  } else if (command === "log-feedback") {
    await logFeedback();
  } else if (command === "log-codex-review") {
    await logCodexReview();
  } else {
    throw new Error(
      "Usage: braintrust-provider-types.mjs create-workflow-trace|emit-pr-metadata|extract-feedback-event|extract-codex-review-event|log-feedback|log-codex-review",
    );
  }
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(message);
  process.exit(1);
}
