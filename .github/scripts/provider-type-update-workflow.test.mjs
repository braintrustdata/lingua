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

test("retries every provider automation Claude phase once", () => {
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
  assert.match(claudeRetryAction, /if: steps\.primary\.outcome == 'failure'/);
  assert.match(
    claudeRetryAction,
    /- name: Check Claude Code outcome\s+id: outcome\s+if: always\(\)/
  );
  assert.match(
    claudeRetryAction,
    /::error::Claude Code failed on both bounded attempts/
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
    /require_success "mechanical validation" "\$\{\{ steps\.mechanical_validation\.outcome \}\}"/
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
    "- name: Check publication readiness"
  );
  const readinessEnd = workflow.indexOf("- name: Create PR", readinessStart);
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
    /steps\.plan_validation\.outcome \}\}" != "success"/
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

test("keeps deterministic validation failures blocking publication", () => {
  for (const check of [
    "mechanical validation",
    "Lingua WASM build",
    "payload fixture sync",
    "payload tests",
    "typed boundary check",
    "cross-provider guard",
    "recoverable patch",
  ]) {
    assert.match(workflow, new RegExp(`require_success "${check}"`));
  }
});

test("does not spend a phase timeout before the retry can start", () => {
  assert.doesNotMatch(workflow, /timeout-minutes:/);

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
