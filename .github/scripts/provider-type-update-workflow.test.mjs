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

test("lets the implementation agent regenerate and validate generator fixes", () => {
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
  assert.doesNotMatch(
    autofixWorkflow,
    /uses: anthropics\/claude-code-action/
  );
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
