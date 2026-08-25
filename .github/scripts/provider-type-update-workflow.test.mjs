import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(
  new URL("../workflows/update-provider-types.yml", import.meta.url),
  "utf8"
);
const autofixWorkflow = readFileSync(
  new URL("../workflows/provider-type-codex-autofix.yml", import.meta.url),
  "utf8"
);
const claudeRetryAction = readFileSync(
  new URL("../actions/claude-code-with-retry/action.yml", import.meta.url),
  "utf8"
);
const providerUpdateSkill = readFileSync(
  new URL(
    "../../.claude/skills/provider-type-update/SKILL.md",
    import.meta.url
  ),
  "utf8"
);
const providerAuditors = [
  "provider-spec-auditor.md",
  "provider-capability-auditor.md",
  "provider-semantic-auditor.md",
  "provider-coverage-auditor.md",
].map((name) =>
  readFileSync(new URL(`../../.claude/agents/${name}`, import.meta.url), "utf8")
);

test("regenerates provider Rust after implementation before safety checks", () => {
  const regeneration = workflow.indexOf(
    "- name: Regenerate Rust provider types after implementation"
  );
  const semanticPolicy = workflow.indexOf(
    "- name: Enforce provider semantic policy"
  );

  assert.notEqual(regeneration, -1);
  assert.notEqual(semanticPolicy, -1);
  assert.ok(regeneration < semanticPolicy);
  assert.match(
    workflow,
    /run: make generate-provider-types PROVIDER="\$\{\{ matrix\.provider \}\}"/
  );
});

test("lets planning invoke its auditors and implementation run required commands", () => {
  assert.match(
    workflow,
    /--allowedTools "Agent,Skill,Read,Glob,Grep,LS,Write,Bash\(git diff:\*\)/
  );
  assert.doesNotMatch(workflow, /Agent\(provider-spec-auditor\)/);
  assert.match(
    workflow,
    /Bash\(make generate-provider-types:\*\).*Bash\(make generate-types:\*\)/
  );
  assert.match(workflow, /Bash\(cargo clippy:\*\)/);
  assert.match(
    workflow,
    /If you change `crates\/generate-types`, run\s+`make generate-provider-types/
  );
});

test("separates deterministic generated-name fixes from semantic blockers", () => {
  assert.match(
    workflow,
    /A generated-name normalization or\s+collision repair must be its own unblocked change id/
  );
  assert.match(
    workflow,
    /Public-name normalization and collision repairs are deterministic\s+generator work/
  );
});

test("runs each provider-update Claude phase at most once", () => {
  assert.equal(
    workflow.match(/uses: \.\/\.github\/actions\/claude-code-with-retry/g)
      ?.length,
    5
  );
  assert.equal(
    autofixWorkflow.match(
      /uses: \.\/\.github\/actions\/claude-code-with-retry/g
    )?.length,
    1
  );
  assert.doesNotMatch(workflow, /uses: anthropics\/claude-code-action/);
  assert.doesNotMatch(autofixWorkflow, /uses: anthropics\/claude-code-action/);
  assert.equal(
    claudeRetryAction.match(
      /uses: anthropics\/claude-code-action@fbda2eb1bdc90d319b8d853f5deb53bca199a7c1/g
    )?.length,
    2
  );
  assert.match(
    claudeRetryAction,
    /- name: Check Claude Code outcome\s+id: outcome\s+if: always\(\)/
  );
  assert.match(
    claudeRetryAction,
    /::error::Claude Code failed on both bounded attempts/
  );
  assert.equal(workflow.match(/retry_on_failure: "false"/g)?.length, 5);
  assert.match(
    claudeRetryAction,
    /steps\.primary\.outcome == 'failure' &&\s+inputs\.retry_on_failure == 'true'/
  );
  assert.match(
    claudeRetryAction,
    /Claude Code failed and retry is disabled for this phase/
  );
});

test("retries partial generation before invoking Claude repair", () => {
  const initialGeneration = workflow.indexOf(
    "- name: Update provider specifications and types"
  );
  const deterministicRetry = workflow.indexOf(
    "- name: Retry provider update after partial generation"
  );
  const claudeRepair = workflow.indexOf("- name: Repair failed generation");

  assert.ok(initialGeneration < deterministicRetry);
  assert.ok(deterministicRetry < claudeRepair);
  assert.match(
    workflow,
    /steps\.generate\.outcome == 'failure' &&\s+steps\.retry\.outcome == 'failure'/
  );
  assert.match(
    workflow,
    /require_ready "provider generation after repair" "\$\{\{ steps\.repair_retry\.outcome \}\}"/
  );
});

test("runs integration planning at medium effort", () => {
  const planningStart = workflow.indexOf(
    "- name: Plan generated provider integration"
  );
  const planningEnd = workflow.indexOf(
    "- name: Validate integration plan",
    planningStart
  );
  const planningStep = workflow.slice(planningStart, planningEnd);

  assert.notEqual(planningStart, -1);
  assert.notEqual(planningEnd, -1);
  assert.match(planningStep, /--model claude-opus-5\s+--effort medium/);
  assert.match(planningStep, /timeout-minutes: 30/);
  assert.match(planningStep, /--max-turns 50/);
  assert.match(planningStep, /display_report: "false"/);
  assert.match(planningStep, /retry_on_failure: "false"/);
  assert.match(planningStep, /exactly `Wrote provider update plan\.`/);
});

test("stages semantic auditing instead of multiplying full-diff reviews", () => {
  assert.match(
    workflow,
    /First invoke only `@provider-spec-auditor`[\s\S]*If that inventory contains semantic changes/
  );
  assert.match(
    workflow,
    /They must not independently re-audit the complete\s+raw specification diff/
  );
  assert.match(
    providerUpdateSkill,
    /Run `provider-spec-auditor` first[\s\S]*Do not have each agent independently\s+re-read the complete raw specification diff/
  );
  assert.match(providerAuditors[0], /maxTurns: 25/);
  assert.match(providerAuditors[0], /under 2,500 words/);
  for (const auditor of providerAuditors.slice(1)) {
    assert.match(
      auditor,
      /Do not\s+independently re-audit the complete raw\s+specification diff/
    );
  }
});

test("uses one bounded repair pass and revalidates before publication", () => {
  assert.equal(
    workflow.match(/- name: Repair deterministic provider validation/g)?.length,
    1
  );
  assert.match(
    workflow,
    /TRACE_PHASE: repair deterministic provider validation/
  );
  assert.match(
    workflow,
    /require_ready "mechanical validation" "\$\{\{ steps\.mechanical_validation\.outcome \}\}"/
  );
  assert.match(workflow, /steps\.publication_readiness\.outcome == 'success'/);
});

test("uses validated artifacts instead of raw Claude step outcomes", () => {
  assert.doesNotMatch(workflow, /require_success "integration plan"/);
  assert.doesNotMatch(workflow, /require_success "implementation"/);
  assert.doesNotMatch(workflow, /require_success "read-only verification"/);
  assert.doesNotMatch(
    workflow,
    /steps\.integration_plan\.outcome != 'success'/
  );
  assert.doesNotMatch(
    workflow,
    /steps\.integration_implementation\.outcome != 'success'/
  );
  assert.doesNotMatch(
    workflow,
    /steps\.integration_verification\.outcome != 'success'/
  );
  assert.equal(
    workflow.match(
      /steps\.stage_plan\.outcome == 'success' &&\s+steps\.post_implementation_plan\.outcome == 'success' &&\s+steps\.mechanical_validation\.outcome == 'success'/g
    )?.length,
    3
  );
});

test("treats missing AI artifacts as a draft instead of a failed run", () => {
  const readinessStart = workflow.indexOf(
    "- name: Assess publication safety and mode"
  );
  const readinessEnd = workflow.indexOf(
    "- name: Build PR body",
    readinessStart
  );
  const readinessStep = workflow.slice(readinessStart, readinessEnd);
  const failureStart = workflow.indexOf(
    "- name: Fail workflow if validation failed"
  );
  const failureEnd = workflow.indexOf("- name: Summary", failureStart);
  const failureStep = workflow.slice(failureStart, failureEnd);

  assert.doesNotMatch(readinessStep, /require_success "plan validation"/);
  assert.doesNotMatch(readinessStep, /require_success "plan staging"/);
  assert.doesNotMatch(readinessStep, /require_success "verification report"/);
  assert.match(
    readinessStep,
    /"integration plan:\$\{\{ steps\.plan_validation\.outcome \}\}"/
  );
  assert.match(
    readinessStep,
    /steps\.verification_report\.outputs\.verdict \}\}" != "pass"/
  );
  assert.match(readinessStep, /mode="draft"/);
  assert.doesNotMatch(failureStep, /plan_validation|verification_report/);
  assert.match(
    failureStep,
    /steps\.publication_readiness\.outcome != 'success'/
  );
});

test("retries final PR publication without rerunning the provider update", () => {
  assert.equal(
    workflow.match(
      /uses: peter-evans\/create-pull-request@22a9089034f40e5a961c8808d113e2c98fb63676/g
    )?.length,
    2
  );
  assert.match(
    workflow,
    /- name: Retry continuation PR publication\s+id: create_pr_retry\s+if: steps\.create_pr\.outcome == 'failure'/
  );
  assert.match(
    workflow,
    /steps\.create_pr\.outcome != 'success' &&\s+steps\.create_pr_retry\.outcome != 'success'/
  );
});

test("turns deterministic validation failures into continuation drafts", () => {
  for (const check of [
    "mechanical validation",
    "Lingua WASM build",
    "payload fixture sync",
    "payload tests",
    "typed boundary check",
    "cross-provider guard",
  ]) {
    assert.match(workflow, new RegExp(`require_ready "${check}"`));
    assert.doesNotMatch(workflow, new RegExp(`require_safe "${check}"`));
  }
  assert.match(workflow, /review_reasons\+=/);
  assert.match(workflow, /mode="draft"/);
  assert.match(workflow, /This draft preserves the automation's partial work/);
});

test("publication safety hard-stops only unsafe paths or an unrecoverable local patch", () => {
  assert.match(
    workflow,
    /require_safe "provider update path policy" "\$\{\{ steps\.path_policy\.outcome \}\}"/
  );
  assert.match(
    workflow,
    /require_safe "provider update path policy" "\$\{\{ steps\.post_repair_path_policy\.outcome \}\}"/
  );
  assert.match(
    workflow,
    /require_safe "recoverable patch" "\$\{\{ steps\.update_patch\.outcome \}\}"/
  );
  assert.equal(workflow.match(/require_safe "/g)?.length, 3);
  assert.doesNotMatch(workflow, /require_safe "provider semantic policy"/);
});

test("bounds Claude phases and suppresses oversized action reports", () => {
  for (const [name, minutes] of [
    ["Repair failed generation", 30],
    ["Plan generated provider integration", 30],
    ["Implement generated provider integration", 40],
    ["Repair deterministic provider validation", 30],
    ["Verify generated provider integration", 20],
  ]) {
    assert.match(
      workflow,
      new RegExp(`- name: ${name}[\\s\\S]*?timeout-minutes: ${minutes}`)
    );
  }
  assert.equal(workflow.match(/display_report: "false"/g)?.length, 5);
  assert.doesNotMatch(workflow, /display_report: "true"/);
  assert.match(claudeRetryAction, /display_report:[\s\S]*?default: "false"/);
  assert.match(workflow, /exactly `Wrote provider update verification\.`/);

  const claudeStart = autofixWorkflow.indexOf(
    "- name: Propose fixes for the Codex review"
  );
  const claudeEnd = autofixWorkflow.indexOf("- name:", claudeStart + 10);
  const claudeStep = autofixWorkflow.slice(claudeStart, claudeEnd);
  assert.doesNotMatch(claudeStep, /timeout-minutes:/);
  assert.match(autofixWorkflow, /propose:[\s\S]*?timeout-minutes: 90/);
});

test("protects AGENTS.md in planning, implementation, and path policy", () => {
  assert.match(
    workflow,
    /`Makefile`, or `AGENTS\.md` in implementation targets/
  );
  assert.match(workflow, /Do not edit `plan\.md`, `AGENTS\.md`/);
  assert.match(workflow, /':\(glob\)\*\*\/AGENTS\.md'/);
});

test("limits transform capture to providers with configured credentials", () => {
  assert.match(
    workflow,
    /capture_providers=\(\).*ANTHROPIC_API_KEY.*AWS_BEARER_TOKEN_BEDROCK.*GOOGLE_API_KEY.*OPENAI_API_KEY/s
  );
  assert.match(workflow, /CAPTURE_PROVIDERS="\$capture_provider_list"/);
  assert.doesNotMatch(workflow, /GOOGLE_APPLICATION_CREDENTIALS:/);
  assert.doesNotMatch(workflow, /VERTEX_PROJECT:/);
});
