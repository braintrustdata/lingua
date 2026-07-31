import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(
  new URL("../workflows/update-provider-types.yml", import.meta.url),
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
