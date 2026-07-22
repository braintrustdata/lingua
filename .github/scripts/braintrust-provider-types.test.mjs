import assert from "node:assert/strict";
import test from "node:test";

import {
  CODEX_BOT,
  GITHUB_ACTIONS_BOT,
  evaluateCodexAutofixEligibility,
  parseRawDiffModes,
  validateAutofixPatch,
} from "./braintrust-provider-types.mjs";

const repository = "braintrustdata/lingua";
const runId = "29947153780";
const sourceSha = "375fa5bfcb696ccb4550cfb8caee68af135c8679";
const headSha = "5d1ee1576286fc316fffc191defe48535be19ee9";

function bot(expected) {
  return { ...expected, type: "Bot" };
}

function fixture() {
  const reviewId = 4734762885;
  return {
    event: {
      review: {
        id: reviewId,
        commit_id: headSha,
        html_url: "https://github.com/braintrustdata/lingua/pull/373#review",
        user: bot(CODEX_BOT),
      },
      pull_request: {
        number: 373,
        html_url: "https://github.com/braintrustdata/lingua/pull/373",
        state: "open",
        draft: false,
        user: bot(GITHUB_ACTIONS_BOT),
        base: { ref: "main", repo: { full_name: repository } },
        head: {
          ref: `update-anthropic-provider-types-${sourceSha.slice(0, 8)}-${runId}`,
          sha: headSha,
          repo: { full_name: repository },
        },
        labels: [{ name: "auto-sync" }],
        body: `<!-- braintrust-provider-type-update
{"version":1,"kind":"provider-type-update","project":"lingua-provider-type-updates","root_span_id":"root","span_id":"span","provider":"anthropic","repository":"${repository}","run_id":"${runId}","run_attempt":"1","workflow":"Update provider types","sha":"${sourceSha}"}
-->`,
      },
    },
    repository,
    inlineComments: [
      {
        id: 3614146973,
        pull_request_review_id: reviewId,
        user: bot(CODEX_BOT),
        body: "Fix the converter.",
        path: "crates/lingua/src/providers/anthropic/convert.rs",
        line: 42,
        html_url: "https://github.com/comment",
      },
    ],
    workflowRun: {
      id: Number(runId),
      path: ".github/workflows/update-provider-types.yml",
      name: "Update provider types",
      event: "schedule",
      repository: { full_name: repository },
      head_branch: "main",
      head_sha: sourceSha,
      run_attempt: 1,
    },
    issueComments: [],
  };
}

function attemptMarker(reviewId, attempt) {
  return {
    user: bot(GITHUB_ACTIONS_BOT),
    body: `<!-- provider-type-codex-autofix
{"version":1,"kind":"attempt","review_id":"${reviewId}","attempt":${attempt}}
-->`,
  };
}

test("accepts a current Codex review on a verified provider update PR", () => {
  const result = evaluateCodexAutofixEligibility(fixture());
  assert.equal(result.eligible, true);
  assert.equal(result.provider, "anthropic");
  assert.equal(result.rootSpanId, "root");
  assert.equal(result.sourceSpanId, "span");
  assert.equal(result.attempt, 1);
  assert.equal(result.inlineComments.length, 1);
});

test("rejects a fork PR with copied labels and metadata", () => {
  const input = fixture();
  input.event.pull_request.head.repo.full_name = "attacker/lingua";
  input.event.pull_request.user = {
    login: "attacker",
    id: 123,
    type: "User",
  };
  assert.equal(evaluateCodexAutofixEligibility(input).eligible, false);
});

test("rejects a lookalike Codex bot", () => {
  const input = fixture();
  input.event.review.user = {
    login: "helpful-codex[bot]",
    id: CODEX_BOT.id + 1,
    type: "Bot",
  };
  assert.match(
    evaluateCodexAutofixEligibility(input).reason,
    /exact Codex bot/,
  );
});

test("rejects a stale review", () => {
  const input = fixture();
  input.event.review.commit_id = sourceSha;
  assert.match(evaluateCodexAutofixEligibility(input).reason, /stale/);
});

test("rejects reviews without exact-bot inline comments", () => {
  const input = fixture();
  input.inlineComments = [];
  assert.match(
    evaluateCodexAutofixEligibility(input).reason,
    /no inline comments/,
  );
});

test("deduplicates an already reserved review", () => {
  const input = fixture();
  input.issueComments = [attemptMarker(input.event.review.id, 1)];
  const result = evaluateCodexAutofixEligibility(input);
  assert.equal(result.eligible, false);
  assert.equal(result.duplicate, true);
});

test("rejects metadata that points at a different workflow run", () => {
  const input = fixture();
  input.workflowRun.path = ".github/workflows/other.yml";
  assert.match(
    evaluateCodexAutofixEligibility(input).reason,
    /Actions run/,
  );
});

test("stops after two attempts", () => {
  const input = fixture();
  input.issueComments = [attemptMarker(1, 1), attemptMarker(2, 2)];
  const result = evaluateCodexAutofixEligibility(input);
  assert.equal(result.eligible, false);
  assert.equal(result.exhausted, true);
});

test("rejects prohibited, binary, oversized, and unsafe-mode patches", () => {
  const result = validateAutofixPatch({
    files: [".github/workflows/evil.yml"],
    changedLines: 801,
    modes: [{ oldMode: "100644", newMode: "100755" }],
    hasBinary: true,
  });
  assert.equal(result.valid, false);
  assert.equal(result.errors.length, 4);
});

test("accepts a small handwritten provider patch", () => {
  const result = validateAutofixPatch({
    files: ["crates/lingua/src/providers/openai/convert.rs"],
    changedLines: 24,
    modes: [{ oldMode: "100644", newMode: "100644" }],
    hasBinary: false,
  });
  assert.deepEqual(result, { valid: true, errors: [] });
});

test("parses raw diff headers without treating path records as modes", () => {
  const oldSha = "a".repeat(40);
  const newSha = "b".repeat(40);
  const raw = `:100644 100644 ${oldSha} ${newSha} M\0crates/lingua/src/providers/openai/convert.rs\0`;

  assert.deepEqual(parseRawDiffModes(raw), [
    {
      oldMode: "100644",
      newMode: "100644",
      path: "crates/lingua/src/providers/openai/convert.rs",
    },
  ]);
});
