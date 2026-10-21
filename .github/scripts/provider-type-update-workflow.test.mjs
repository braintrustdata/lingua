import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const workflow = read("../workflows/update-provider-types.yml");
const worker = read("provider-type-update-worker.mjs");
const controller = read("provider-type-update-isolation.mjs");
const publisher = read("provider-type-update-publish.mjs");
const dockerfile = read("../provider-type-update/Dockerfile");
const prompts = Object.fromEntries(
  ["repair", "plan", "implement", "repair-validation", "verify", "autofix"].map(
    (name) => [name, read(`../provider-type-update/prompts/${name}.md`)]
  )
);
const skill = read("../../.claude/skills/provider-type-update/SKILL.md");
const autofix = read("../workflows/provider-type-codex-autofix.yml");
const job = (name) =>
  workflow.split(`  ${name}:\n`)[1]?.split(/^  [\w-]+:\n/m)[0];

test("agent, validation, and publication run in separate jobs with minimum permissions", () => {
  assert.match(workflow, /permissions: \{\}/);
  for (const name of ["propose", "validate", "publish"]) {
    assert.match(job(name), /runs-on: ubuntu-latest/);
  }
  assert.match(job("propose"), /contents: read/);
  assert.doesNotMatch(
    job("propose"),
    /contents: write|pull-requests: write|github_token:/
  );
  assert.match(job("validate"), /contents: read/);
  assert.doesNotMatch(
    job("validate"),
    /secrets\.|contents: write|pull-requests: write/
  );
  assert.match(job("publish"), /contents: write\s+pull-requests: write/);
  assert.match(job("publish"), /needs: \[provider_matrix, propose, validate\]/);
});

test("only the host inference proxy receives the reusable credential", () => {
  assert.equal(workflow.match(/secrets\./g)?.length, 1);
  assert.match(
    workflow,
    /BRAINTRUST_API_KEY: \$\{\{ secrets\.BRAINTRUST_API_KEY \}\}/
  );
  assert.match(
    controller,
    /createInferenceProxy\(\{\s*key: process\.env\.BRAINTRUST_API_KEY,?\s*\}\)/
  );
  assert.match(controller, /ANTHROPIC_API_KEY=\$\{proxy.token\}/);
  assert.doesNotMatch(controller, /ANTHROPIC_API_KEY=\$\{process.env/);
  assert.doesNotMatch(worker, /BRAINTRUST_API_KEY|github.token|GITHUB_TOKEN/);
  assert.doesNotMatch(
    workflow,
    /LINGUA_CAPTURE_|claude-code-with-retry|actions\/cache/
  );
});

test("all jobs check out the immutable workflow revision without persisted credentials", () => {
  assert.equal(workflow.match(/ref: \$\{\{ github.sha \}\}/g)?.length, 3);
  assert.equal(workflow.match(/persist-credentials: false/g)?.length, 3);
  assert.match(
    job("propose"),
    /if: >-\s+github\.ref == 'refs\/heads\/main' \|\|\s+\(github\.event_name == 'workflow_dispatch' &&\s+github\.ref == 'refs\/heads\/codex\/provider-implementation-efficiency'\)/
  );
});

test("publication never executes candidate code and only opens drafts", () => {
  const publication = job("publish");
  assert.doesNotMatch(
    publication,
    /run:.*(?:cargo |pnpm |npm |make |\.\/)|uses: \.\//
  );
  assert.doesNotMatch(publication, /secrets\.|auto-sync|draft: false/);
  assert.equal(publication.match(/draft: true/g)?.length, 2);
  assert.equal(
    publication.match(/GIT_CONFIG_KEY_0: core.hooksPath/g)?.length,
    2
  );
  assert.match(publisher, /metadata.sha256 !== patchDigest\(patch\)/);
  assert.match(publisher, /verified.sha256 !== metadata.sha256/);
  assert.match(publisher, /applyProposal\(/);
});

test("untrusted case names and other model output never become workflow shell source", () => {
  assert.doesNotMatch(
    workflow,
    /\$\{\{ steps\.(?:capture_cases|integration_plan|integration_implementation).*\}\}/
  );
  for (const name of ["propose", "validate", "publish"]) {
    assert.match(job(name), /PROVIDER: \$\{\{ matrix.provider \}\}/);
    assert.match(job(name), /\.mjs .*"\$PROVIDER"/);
  }
  assert.doesNotMatch(JSON.stringify(prompts), /\$\{\{/);
});

test("artifacts are fixed-name sanitized exports rather than raw logs or agent directories", () => {
  assert.doesNotMatch(
    workflow,
    /provider-type-update-checks|\.log\b|\/home\/|\.claude\/|path:.*RUNNER_TEMP.*$/m
  );
  assert.match(controller, /redactValue\(report, redact\)/);
  assert.match(controller, /redactPatch\(patch\) !== patch/);
  assert.match(controller, /stdio: \["ignore", "ignore", "ignore"\]/);
  assert.doesNotMatch(controller, /docker\(\["cp"/);
});

test("the immutable worker preserves bounded planning, repair, and verification", () => {
  assert.match(worker, /phase\("plan", 1800000, 50\)/);
  assert.match(worker, /phase\("implement", 2400000, 100\)/);
  assert.match(worker, /phase\("repair-validation", 1800000, 60\)/);
  assert.match(worker, /phase\("verify", 1200000, 40\)/);
  for (const action of [
    "provider-tests",
    "generator-tests",
    "build",
    "clippy",
  ]) {
    assert.match(
      worker,
      new RegExp(`provider-type-update-check\\.mjs",\\s*"${action}"`)
    );
  }
  assert.match(worker, /immutable-plan/);
  assert.match(dockerfile, /@anthropic-ai\/claude-code@2.1.304/);
  assert.match(dockerfile, /USER 1000:1000/);
});

test("the semantic policy and provider-only audit contract survive isolation", () => {
  assert.match(prompts.plan, /First invoke only `@provider-spec-auditor`/);
  assert.match(
    prompts.plan,
    /Classify provider-hosted execution[\s\S]*`provider_only`/
  );
  assert.match(
    prompts.implement,
    /For every `provider_only` item:[\s\S]*byte-preserving same-format passthrough[\s\S]*explicit unsupported error/
  );
  assert.match(
    prompts.verify,
    /For every `provider_only` plan item, verify native wire acceptance/
  );
  assert.match(
    prompts.verify,
    /Treat any universal-model expansion or expected-difference entry/
  );
  assert.match(
    worker,
    /\/opt\/updater\/provider-type-update-semantic-policy.mjs/
  );
  assert.match(
    skill,
    /Planned live captures remain incomplete until a human reviews/
  );
  assert.match(prompts.plan, /Live capture is deferred to human review/);
});

test("review autofix uses trusted isolation code and rechecks the patch after validation", () => {
  const propose = autofix.split("  propose:\n")[1].split("  validate:\n")[0];
  assert.doesNotMatch(
    propose,
    /uses: \.\/|run:.*(?:cargo|pnpm|npm install)|anthropic_api_key|contents: write|pull-requests: write/
  );
  assert.equal(propose.match(/persist-credentials: false/g)?.length, 2);
  assert.equal(propose.match(/secrets\./g)?.length, 1);
  assert.match(
    propose,
    /node trusted\/\.github\/scripts\/provider-type-update-isolation.mjs autofix/
  );
  const publish = autofix.split("  publish:\n")[1].split("  report:\n")[0];
  assert.match(publish, /path: trusted/);
  assert.match(publish, /path: reviewed/);
  assert.match(publish, /provider-type-update-apply.mjs/);
  assert.match(publish, /GIT_CONFIG_KEY_0: core.hooksPath/);
  assert.doesNotMatch(publish, /run:.*(?:cargo|pnpm|npm install)|uses: \.\//);
});
