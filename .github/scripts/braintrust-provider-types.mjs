#!/usr/bin/env node

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const CODEX_BOT = Object.freeze({
  login: "chatgpt-codex-connector[bot]",
  id: 199175422,
});
const GITHUB_ACTIONS_BOT = Object.freeze({
  login: "github-actions[bot]",
  id: 41898282,
});
const PROVIDER_TYPE_WORKFLOW_PATH =
  ".github/workflows/update-provider-types.yml";
const PROVIDER_TYPE_WORKFLOW_NAME = "Update provider types";
const AUTOFIX_MARKER = "provider-type-codex-autofix";
const AUTOFIX_MAX_ATTEMPTS = 2;

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

function gitOutput(...args) {
  try {
    return execFileSync("git", args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    return `${error.stdout ?? ""}${error.stderr ?? ""}`;
  }
}

function repositorySnapshot() {
  return {
    head: gitOutput("rev-parse", "HEAD").trim(),
    git_status: gitOutput("status", "--short"),
    git_diff_stat: gitOutput("diff", "--stat"),
    git_diff_names: gitOutput("diff", "--name-only"),
    git_diff: gitOutput("diff", "--no-color", "--no-ext-diff"),
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
  const snapshot = repositorySnapshot();

  span.log({
    input: {
      provider,
      event: optionalEnv("GITHUB_EVENT_NAME"),
      run_id: optionalEnv("GITHUB_RUN_ID"),
      run_attempt: optionalEnv("GITHUB_RUN_ATTEMPT"),
      head: snapshot.head,
    },
    output: snapshot,
    metadata: workflowMetadata({
      braintrust_project: projectName,
      root_span_id: rootSpanId,
      span_id: spanId,
      head: snapshot.head,
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

async function createTaskTrace() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const projectName =
    optionalEnv("BRAINTRUST_PROJECT") ||
    optionalEnv("BRAINTRUST_CC_PROJECT") ||
    "lingua-provider-type-updates";
  const provider = requireEnv("PROVIDER");
  const phase = requireEnv("TRACE_PHASE");
  const logger = braintrust.initLogger({ projectName });
  const parentSpanId = optionalEnv("BRAINTRUST_PARENT_SPAN_ID");
  const parentRootSpanId = optionalEnv("BRAINTRUST_ROOT_SPAN_ID");
  const parentSpanIds =
    parentSpanId && parentRootSpanId
      ? { spanId: parentSpanId, rootSpanId: parentRootSpanId }
      : undefined;
  const span = logger.startSpan({
    name: `Claude ${phase}: update ${provider} provider types`,
    parentSpanIds,
  });
  const spanId = span.spanId || span.id;
  const rootSpanId = span.rootSpanId || span.root_span_id || spanId;
  const snapshot = repositorySnapshot();

  span.log({
    input: {
      provider,
      phase,
      head: snapshot.head,
      github_run_id: optionalEnv("GITHUB_RUN_ID"),
      github_run_attempt: optionalEnv("GITHUB_RUN_ATTEMPT"),
      github_workflow: optionalEnv("GITHUB_WORKFLOW"),
      github_job: optionalEnv("GITHUB_JOB"),
    },
    output: snapshot,
    metadata: workflowMetadata({
      braintrust_project: projectName,
      phase,
      head: snapshot.head,
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

function renderActionMessage(template, values) {
  return Object.entries(values).reduce(
    (message, [name, value]) =>
      message.replaceAll(`{{${name}}}`, String(value)),
    template,
  );
}

function requireStringParameter(data, name) {
  const value = data[name];
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`Braintrust parameter '${name}' must be a non-empty string`);
  }
  return value.trim();
}

async function loadActionMessages() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const projectName =
    optionalEnv("BRAINTRUST_PROJECT") || "lingua-provider-type-updates";
  const slug =
    optionalEnv("BRAINTRUST_PARAMETERS_SLUG") ||
    "provider-type-update-messages";
  const phase = requireEnv("PROMPT_PHASE");
  const provider = requireEnv("PROVIDER");
  const fieldPrefix =
    phase === "repair" ? "repair" : phase === "review" ? "review" : undefined;

  if (!fieldPrefix) {
    throw new Error("PROMPT_PHASE must be 'repair' or 'review'");
  }

  const parameters = await braintrust.loadParameters({ projectName, slug });
  if (!parameters.data || typeof parameters.data !== "object") {
    throw new Error("Braintrust parameters did not contain an object data value");
  }
  const templateValues = {
    provider,
    generation_log_path:
      fieldPrefix === "repair" ? requireEnv("GENERATION_LOG_PATH") : "",
  };
  const systemMessage = renderActionMessage(
    requireStringParameter(parameters.data, `${fieldPrefix}_system_message`),
    templateValues,
  );
  const userMessage = renderActionMessage(
    requireStringParameter(parameters.data, `${fieldPrefix}_user_message`),
    templateValues,
  );

  writeGithubOutputValue("system_message", systemMessage);
  writeGithubOutputValue("user_message", userMessage);
  writeGithubOutput({
    parameters_id: parameters.id,
    parameters_version: parameters.version,
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

  try {
    return JSON.parse(match[1]);
  } catch {
    return undefined;
  }
}

function isExactBot(user, expected) {
  return (
    user?.type === "Bot" &&
    user?.login === expected.login &&
    Number(user?.id) === expected.id
  );
}

function extractAutofixMarker(body) {
  const match = (body || "").match(
    /<!--\s*provider-type-codex-autofix\s*\n([\s\S]*?)\n-->/,
  );
  if (!match) {
    return undefined;
  }

  try {
    return JSON.parse(match[1]);
  } catch {
    return undefined;
  }
}

function autofixMarker(metadata) {
  return `<!-- ${AUTOFIX_MARKER}\n${JSON.stringify(metadata)}\n-->`;
}

function ineligible(reason, extra = {}) {
  return {
    eligible: false,
    reason,
    ...extra,
  };
}

function evaluateCodexAutofixEligibility({
  event,
  repository,
  inlineComments,
  workflowRun,
  issueComments,
}) {
  const review = event?.review;
  const pullRequest = event?.pull_request;
  if (!review || !pullRequest) {
    return ineligible("Event is not a pull request review");
  }

  if (!isExactBot(review.user, CODEX_BOT)) {
    return ineligible("Review was not submitted by the exact Codex bot");
  }

  if (!isExactBot(pullRequest.user, GITHUB_ACTIONS_BOT)) {
    return ineligible("PR was not created by github-actions[bot]");
  }

  if (
    pullRequest.state !== "open" ||
    pullRequest.draft === true ||
    pullRequest.base?.ref !== "main"
  ) {
    return ineligible("PR is not an open, non-draft PR targeting main");
  }

  if (
    pullRequest.head?.repo?.full_name !== repository ||
    pullRequest.base?.repo?.full_name !== repository
  ) {
    return ineligible("PR head and base must both be in the current repository");
  }

  if (!review.commit_id || review.commit_id !== pullRequest.head?.sha) {
    return ineligible("Codex review is stale relative to the PR head");
  }

  const labels = (pullRequest.labels || []).map((label) => label.name);
  if (!labels.includes("auto-sync")) {
    return ineligible("PR is missing the auto-sync label");
  }

  const metadata = extractHiddenMetadata(pullRequest.body || "");
  if (
    metadata?.version !== 1 ||
    metadata?.kind !== "provider-type-update" ||
    metadata?.project !== "lingua-provider-type-updates" ||
    typeof metadata?.root_span_id !== "string" ||
    !metadata.root_span_id ||
    typeof metadata?.span_id !== "string" ||
    !metadata.span_id ||
    metadata?.repository !== repository ||
    metadata?.workflow !== PROVIDER_TYPE_WORKFLOW_NAME ||
    !["openai", "anthropic", "google"].includes(metadata?.provider) ||
    !/^\d+$/.test(String(metadata?.run_id || "")) ||
    !/^[1-9]\d*$/.test(String(metadata?.run_attempt || "")) ||
    !/^[0-9a-f]{40}$/.test(String(metadata?.sha || ""))
  ) {
    return ineligible("PR metadata is not a valid provider type update record");
  }

  const expectedBranch = `update-${metadata.provider}-provider-types-${metadata.sha.slice(0, 8)}-${metadata.run_id}`;
  if (pullRequest.head?.ref !== expectedBranch) {
    return ineligible("PR branch does not match provider update provenance");
  }

  if (
    Number(workflowRun?.id) !== Number(metadata.run_id) ||
    workflowRun?.path !== PROVIDER_TYPE_WORKFLOW_PATH ||
    workflowRun?.name !== PROVIDER_TYPE_WORKFLOW_NAME ||
    !["schedule", "workflow_dispatch"].includes(workflowRun?.event) ||
    workflowRun?.repository?.full_name !== repository ||
    workflowRun?.head_branch !== "main" ||
    workflowRun?.head_sha !== metadata.sha ||
    Number(workflowRun?.run_attempt) !== Number(metadata.run_attempt)
  ) {
    return ineligible("Referenced Actions run is not the provider type update run");
  }

  const actionableComments = (inlineComments || []).filter(
    (comment) =>
      Number(comment.pull_request_review_id) === Number(review.id) &&
      isExactBot(comment.user, CODEX_BOT) &&
      typeof comment.body === "string" &&
      comment.body.trim(),
  );
  if (actionableComments.length === 0) {
    return ineligible("Codex review has no inline comments");
  }

  const actionMarkers = (issueComments || [])
    .filter((comment) => isExactBot(comment.user, GITHUB_ACTIONS_BOT))
    .map((comment) => ({
      comment,
      marker: extractAutofixMarker(comment.body),
    }))
    .filter(({ marker }) => marker?.version === 1);
  const attempts = actionMarkers.filter(
    ({ marker }) => marker.kind === "attempt",
  );

  if (
    attempts.some(
      ({ marker }) => String(marker.review_id) === String(review.id),
    )
  ) {
    return ineligible("This Codex review already has an autofix attempt", {
      duplicate: true,
    });
  }

  if (attempts.length >= AUTOFIX_MAX_ATTEMPTS) {
    const exhaustionReported = actionMarkers.some(
      ({ marker }) =>
        marker.kind === "exhausted" &&
        String(marker.review_id) === String(review.id),
    );
    return ineligible("Autofix attempt limit reached", {
      exhausted: true,
      exhaustionReported,
    });
  }

  return {
    eligible: true,
    provider: metadata.provider,
    braintrustProject: metadata.project,
    rootSpanId: metadata.root_span_id,
    sourceSpanId: metadata.span_id,
    prNumber: pullRequest.number,
    prUrl: pullRequest.html_url,
    headRef: pullRequest.head.ref,
    headSha: pullRequest.head.sha,
    reviewId: review.id,
    reviewUrl: review.html_url,
    attempt: attempts.length + 1,
    inlineComments: actionableComments.map((comment) => ({
      id: comment.id,
      path: comment.path,
      line: comment.line || comment.original_line || null,
      start_line: comment.start_line || comment.original_start_line || null,
      body: comment.body.trim(),
      url: comment.html_url,
    })),
  };
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
  return isExactBot(user, CODEX_BOT);
}

async function githubApi(path, options = {}) {
  const token = requireEnv("GITHUB_TOKEN");
  const response = await fetch(`https://api.github.com${path}`, {
    method: options.method || "GET",
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "lingua-provider-type-feedback",
    },
    body: options.body ? JSON.stringify(options.body) : undefined,
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

async function inspectCodexAutofixEvent() {
  const eventPath = requireEnv("GITHUB_EVENT_PATH");
  const repository = requireEnv("GITHUB_REPOSITORY");
  const event = JSON.parse(readFileSync(eventPath, "utf8"));
  const pullRequest = event.pull_request;
  const review = event.review;
  const metadata = extractHiddenMetadata(pullRequest?.body || "");
  const [owner, repo] = repository.split("/");

  let workflowRun;
  let inlineComments = [];
  let issueComments = [];
  if (pullRequest && review && /^\d+$/.test(String(metadata?.run_id || ""))) {
    [workflowRun, inlineComments, issueComments] = await Promise.all([
      githubApi(
        `/repos/${owner}/${repo}/actions/runs/${encodeURIComponent(metadata.run_id)}`,
      ),
      githubApiPages(`/repos/${owner}/${repo}/pulls/${pullRequest.number}/comments`),
      githubApiPages(`/repos/${owner}/${repo}/issues/${pullRequest.number}/comments`),
    ]);
  }

  const result = evaluateCodexAutofixEligibility({
    event,
    repository,
    inlineComments,
    workflowRun,
    issueComments,
  });

  if (!result.eligible) {
    if (result.exhausted && !result.exhaustionReported && pullRequest && review) {
      await githubApi(
        `/repos/${owner}/${repo}/issues/${pullRequest.number}/comments`,
        {
          method: "POST",
          body: {
            body: `${autofixMarker({
              version: 1,
              kind: "exhausted",
              review_id: String(review.id),
            })}\n\nProvider type Codex autofix has reached its ${AUTOFIX_MAX_ATTEMPTS}-attempt limit. This review needs manual follow-up.`,
          },
        },
      );
    }

    writeGithubOutput({
      eligible: "false",
      reason: result.reason,
      exhausted: result.exhausted ? "true" : "false",
      duplicate: result.duplicate ? "true" : "false",
    });
    return;
  }

  const reservation = await githubApi(
    `/repos/${owner}/${repo}/issues/${result.prNumber}/comments`,
    {
      method: "POST",
      body: {
        body: `${autofixMarker({
          version: 1,
          kind: "attempt",
          review_id: String(result.reviewId),
          attempt: result.attempt,
        })}\n\nProvider type Codex autofix attempt **${result.attempt}/${AUTOFIX_MAX_ATTEMPTS}** started for [review ${result.reviewId}](${result.reviewUrl}).`,
      },
    },
  );

  const reviewPath = requireEnv("AUTOFIX_REVIEW_PATH");
  writeFileSync(
    reviewPath,
    `${JSON.stringify(
      {
        version: 1,
        repository,
        provider: result.provider,
        braintrust_project: result.braintrustProject,
        root_span_id: result.rootSpanId,
        source_span_id: result.sourceSpanId,
        pr_number: result.prNumber,
        pr_url: result.prUrl,
        head_ref: result.headRef,
        head_sha: result.headSha,
        review_id: result.reviewId,
        review_url: result.reviewUrl,
        review_body: (review.body || "").trim(),
        inline_comments: result.inlineComments,
      },
      null,
      2,
    )}\n`,
  );

  writeGithubOutput({
    eligible: "true",
    provider: result.provider,
    root_span_id: result.rootSpanId,
    source_span_id: result.sourceSpanId,
    pr_number: result.prNumber,
    head_ref: result.headRef,
    head_sha: result.headSha,
    review_id: result.reviewId,
    attempt: result.attempt,
    marker_comment_id: reservation.id,
  });
}

async function updateCodexAutofixAttempt() {
  const repository = requireEnv("GITHUB_REPOSITORY");
  const [owner, repo] = repository.split("/");
  const commentId = requireEnv("AUTOFIX_COMMENT_ID");
  const reviewId = requireEnv("AUTOFIX_REVIEW_ID");
  const attempt = Number(requireEnv("AUTOFIX_ATTEMPT"));
  const status = requireEnv("AUTOFIX_STATUS");
  const detail = requireEnv("AUTOFIX_DETAIL");
  const commitSha = optionalEnv("AUTOFIX_COMMIT_SHA");
  const commitText = commitSha ? ` Commit: \`${commitSha}\`.` : "";

  await githubApi(`/repos/${owner}/${repo}/issues/comments/${commentId}`, {
    method: "PATCH",
    body: {
      body: `${autofixMarker({
        version: 1,
        kind: "attempt",
        review_id: String(reviewId),
        attempt,
      })}\n\nProvider type Codex autofix attempt **${attempt}/${AUTOFIX_MAX_ATTEMPTS}** ${status}. ${detail}${commitText}`,
    },
  });
}

async function requestCodexRereview() {
  const repository = requireEnv("GITHUB_REPOSITORY");
  const [owner, repo] = repository.split("/");
  const prNumber = requireEnv("AUTOFIX_PR_NUMBER");
  const attempt = requireEnv("AUTOFIX_ATTEMPT");

  await githubApi(`/repos/${owner}/${repo}/issues/${prNumber}/comments`, {
    method: "POST",
    body: {
      body: `@codex review\n\nProvider type autofix attempt ${attempt}/${AUTOFIX_MAX_ATTEMPTS} passed its focused validation.`,
    },
  });
}

function generatedProviderPath(provider) {
  if (!["openai", "anthropic", "google"].includes(provider)) {
    return null;
  }
  return `crates/lingua/src/providers/${provider}/generated.rs`;
}

function isProhibitedAutofixPath(path, allowedGeneratedPath) {
  const basename = path.split("/").at(-1);
  return (
    path.startsWith(".github/") ||
    path.startsWith("specs/") ||
    path.startsWith("pipelines/") ||
    path.split("/").includes("AGENTS.md") ||
    (basename === "generated.rs" && path !== allowedGeneratedPath) ||
    basename.endsWith(".lock") ||
    [
      "Cargo.toml",
      "Cargo.lock",
      "Makefile",
      ".gitmodules",
      "go.mod",
      "go.sum",
      "mise.toml",
      "package.json",
      "package-lock.json",
      "pnpm-lock.yaml",
      "pnpm-workspace.yaml",
      "pyproject.toml",
      "rust-toolchain.toml",
      "yarn.lock",
    ].includes(basename)
  );
}

function validateAutofixPatch({ files, modes, hasBinary, generatedProvider }) {
  const errors = [];
  const allowedGeneratedPath = generatedProviderPath(generatedProvider);
  if (files.length === 0) {
    errors.push("Claude produced no patch");
  }
  const prohibited = files.filter((path) =>
    isProhibitedAutofixPath(path, allowedGeneratedPath),
  );
  if (prohibited.length > 0) {
    errors.push(`Patch touches prohibited paths: ${prohibited.join(", ")}`);
  }
  if (
    allowedGeneratedPath &&
    files.includes(allowedGeneratedPath) &&
    !files.some((path) => path.startsWith("crates/generate-types/"))
  ) {
    errors.push(
      `Generated provider output requires a matching crates/generate-types change: ${allowedGeneratedPath}`,
    );
  }
  if (hasBinary) {
    errors.push("Patch contains binary changes");
  }
  const unsafeModes = modes.filter(
    ({ oldMode, newMode }) =>
      !["000000", "100644"].includes(oldMode) ||
      !["000000", "100644"].includes(newMode),
  );
  if (unsafeModes.length > 0) {
    errors.push("Patch changes executable, symlink, or submodule modes");
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

function parseRawDiffModes(raw) {
  const records = raw.split("\0");
  if (records.at(-1) === "") {
    records.pop();
  }

  const modes = [];
  for (let index = 0; index < records.length; index += 2) {
    const header = records[index];
    const path = records[index + 1];
    const match = header.match(/^:(\d{6}) (\d{6}) /);
    modes.push({
      oldMode: match?.[1] || "unknown",
      newMode: match?.[2] || "unknown",
      path: path || "unknown",
    });
  }
  return modes;
}

function validateCodexAutofixPatch() {
  const baseSha = requireEnv("AUTOFIX_BASE_SHA");
  const generatedProvider = optionalEnv("AUTOFIX_GENERATED_PROVIDER");
  const nameList = execFileSync(
    "git",
    ["diff", "--name-only", "-z", "--no-renames", baseSha, "--"],
    { encoding: "utf8" },
  );
  const numstat = execFileSync(
    "git",
    ["diff", "--numstat", "-z", "--no-renames", baseSha, "--"],
    { encoding: "utf8" },
  );
  const raw = execFileSync(
    "git",
    ["diff", "--raw", "-z", "--no-abbrev", "--no-renames", baseSha, "--"],
    { encoding: "utf8" },
  );
  const files = nameList.split("\0").filter(Boolean);
  let changedLines = 0;
  let hasBinary = false;
  for (const line of numstat.split("\0").filter(Boolean)) {
    const [added, deleted] = line.split("\t");
    if (added === "-" || deleted === "-") {
      hasBinary = true;
    } else {
      changedLines += Number(added) + Number(deleted);
    }
  }
  const modes = parseRawDiffModes(raw);
  const result = validateAutofixPatch({
    files,
    changedLines,
    modes,
    hasBinary,
    generatedProvider,
  });
  writeGithubOutput({
    patch_policy: result.valid ? "passed" : "failed",
    patch_file_count: files.length,
    patch_changed_lines: changedLines,
  });
  if (!result.valid) {
    throw new Error(result.errors.join("\n"));
  }

  console.log(
    `Validated autofix patch: ${files.length} files, ${changedLines} changed lines`,
  );
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
        metadata: feedbackMetadata,
      });
      span.logFeedback({
        scores: {
          github_pr_feedback: score,
        },
        comment: optionalEnv("BT_COMMENT_BODY"),
        metadata: feedbackMetadata,
        source: "external",
      });
    },
    {
      name: "github_pr_feedback",
      type: "score",
      spanAttributes: {
        purpose: "scorer",
      },
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
      span.logFeedback({
        scores: {
          github_codex_review: 0,
        },
        comment: requireEnv("BT_REVIEW_OUTPUT"),
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
        source: "external",
      });
    },
    {
      name: "github_codex_review",
      type: "score",
      spanAttributes: {
        purpose: "scorer",
      },
      parentSpanIds,
    },
  );

  await flushBraintrust(braintrust);
}

async function logCodexAutofixResult() {
  requireEnv("BRAINTRUST_API_KEY");
  const braintrust = loadBraintrust();
  const projectName = requireEnv("BRAINTRUST_PROJECT");
  const parentSpanIds = {
    spanId: requireEnv("BRAINTRUST_PARENT_SPAN_ID"),
    rootSpanId: requireEnv("BRAINTRUST_ROOT_SPAN_ID"),
  };
  const attempt = requireEnv("AUTOFIX_ATTEMPT");
  const publishResult = requireEnv("AUTOFIX_PUBLISH_RESULT");
  const status = publishResult === "success" ? "succeeded" : "failed";
  const logger = braintrust.initLogger({ projectName });

  await logger.traced(
    async (span) => {
      span.log({
        input: {
          provider: requireEnv("PROVIDER"),
          pr_number: requireEnv("AUTOFIX_PR_NUMBER"),
          review_id: requireEnv("AUTOFIX_REVIEW_ID"),
          attempt,
        },
        output: {
          status,
          proposal: {
            job_result: optionalEnv("AUTOFIX_PROPOSE_RESULT"),
            patch_policy: optionalEnv("AUTOFIX_PROPOSAL_POLICY"),
            patch_file_count: optionalEnv("AUTOFIX_PROPOSAL_FILE_COUNT"),
            patch_changed_lines: optionalEnv(
              "AUTOFIX_PROPOSAL_CHANGED_LINES",
            ),
          },
          validation: {
            job_result: optionalEnv("AUTOFIX_VALIDATE_RESULT"),
            patch_policy: optionalEnv("AUTOFIX_VALIDATED_POLICY"),
            patch_file_count: optionalEnv("AUTOFIX_VALIDATED_FILE_COUNT"),
            patch_changed_lines: optionalEnv(
              "AUTOFIX_VALIDATED_CHANGED_LINES",
            ),
            provider_tests: optionalEnv("AUTOFIX_TEST_RESULT"),
            clippy: optionalEnv("AUTOFIX_CLIPPY_RESULT"),
            typed_boundary: optionalEnv("AUTOFIX_TYPED_BOUNDARY_RESULT"),
          },
          publication: {
            job_result: publishResult,
            commit_sha: optionalEnv("AUTOFIX_COMMIT_SHA"),
            rereview_requested:
              optionalEnv("AUTOFIX_REREVIEW_RESULT") === "success",
            rereview_result: optionalEnv("AUTOFIX_REREVIEW_RESULT"),
          },
        },
        metadata: workflowMetadata({
          provider: requireEnv("PROVIDER"),
          phase: "codex_autofix_result",
          autofix_status: status,
          autofix_attempt: attempt,
          pr_number: requireEnv("AUTOFIX_PR_NUMBER"),
          review_id: requireEnv("AUTOFIX_REVIEW_ID"),
          target_span_id: parentSpanIds.spanId,
          target_root_span_id: parentSpanIds.rootSpanId,
        }),
      });
    },
    {
      name: `Codex autofix attempt ${attempt} result`,
      type: "task",
      spanAttributes: {
        purpose: "task",
      },
      parentSpanIds,
    },
  );

  await flushBraintrust(braintrust);
}

async function main(command) {
  if (command === "create-workflow-trace") {
    await createWorkflowTrace();
  } else if (command === "create-task-trace") {
    await createTaskTrace();
  } else if (command === "load-action-messages") {
    await loadActionMessages();
  } else if (command === "emit-pr-metadata") {
    emitPrMetadata();
  } else if (command === "extract-feedback-event") {
    extractFeedbackEvent();
  } else if (command === "extract-codex-review-event") {
    await extractCodexReviewEvent();
  } else if (command === "inspect-codex-autofix-event") {
    await inspectCodexAutofixEvent();
  } else if (command === "validate-codex-autofix-patch") {
    validateCodexAutofixPatch();
  } else if (command === "update-codex-autofix-attempt") {
    await updateCodexAutofixAttempt();
  } else if (command === "request-codex-rereview") {
    await requestCodexRereview();
  } else if (command === "log-feedback") {
    await logFeedback();
  } else if (command === "log-codex-review") {
    await logCodexReview();
  } else if (command === "log-codex-autofix-result") {
    await logCodexAutofixResult();
  } else {
    throw new Error(
      "Usage: braintrust-provider-types.mjs create-workflow-trace|create-task-trace|load-action-messages|emit-pr-metadata|extract-feedback-event|extract-codex-review-event|inspect-codex-autofix-event|validate-codex-autofix-patch|update-codex-autofix-attempt|request-codex-rereview|log-feedback|log-codex-review|log-codex-autofix-result",
    );
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    await main(process.argv[2]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(message);
    process.exit(1);
  }
}

export {
  AUTOFIX_MAX_ATTEMPTS,
  CODEX_BOT,
  GITHUB_ACTIONS_BOT,
  evaluateCodexAutofixEligibility,
  extractAutofixMarker,
  extractHiddenMetadata,
  parseRawDiffModes,
  validateAutofixPatch,
};
