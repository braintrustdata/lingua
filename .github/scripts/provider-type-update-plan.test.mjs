import assert from "node:assert/strict";
import test from "node:test";

import {
  captureCases,
  hasBlockers,
  renderBlockers,
  renderProviderOnly,
  validatePlan,
  validateVerification,
} from "./provider-type-update-plan.mjs";

function status(value = "not_affected") {
  return { status: value, evidence: `${value} evidence` };
}

function validPlan() {
  return {
    schema_version: 1,
    provider: "anthropic",
    spec_review: {
      complete: true,
      evidence: ["specification.json: StopReason"],
    },
    audit_reports: {
      spec: "Audited StopReason in specification.json.",
      capabilities: "No model matcher changes in capabilities.rs.",
      semantics: "Traced stop reason through response and streaming adapters.",
      coverage: "Found focused response and streaming payload cases.",
    },
    changes: [
      {
        id: "context-window-stop-reason",
        kind: "added",
        source: {
          path: "specification.json",
          symbol: "StopReason",
          summary: "Adds a context-window terminal reason.",
        },
        root_cause:
          "The provider added a terminal reason that existing adapters do not classify.",
        expected_behavior:
          "Imports and streaming exports preserve the reason and mark the response incomplete.",
        expected_diff_impact:
          "Focused response and streaming snapshots gain the new terminal reason.",
        generated_type_effect: status("affected"),
        surfaces: {
          model_capabilities: status(),
          request_import: status("not_applicable"),
          request_export: status("not_applicable"),
          response_import: status("affected"),
          response_export: status("affected"),
          streaming: status("affected"),
          universal_semantics: status("affected"),
          cross_provider: status("affected"),
        },
        mapping: {
          decision: "map",
          rationale: "Maps to the universal incomplete status.",
        },
        implementation_targets: [
          "crates/lingua/src/providers/anthropic/adapter.rs",
        ],
        tests: {
          unit: ["stop_reason_context_window_is_incomplete"],
          payload_cases: ["anthropicContextWindowExceeded"],
          live_capture: {
            required: true,
            cases: ["anthropicContextWindowExceeded"],
            rationale:
              "Confirms the provider wire value and streaming behavior.",
          },
          commands: ["cargo test -p lingua anthropic"],
        },
      },
    ],
    blockers: [],
  };
}

function providerOnlyPlan() {
  const plan = validPlan();
  const change = plan.changes[0];
  change.id = "browser-toolset";
  change.source = {
    path: "specification.json",
    symbol: "BrowserToolset20260801",
    summary: "Adds a provider-defined browser harness toolset.",
  };
  change.root_cause =
    "The provider added a harness protocol that the caller executes.";
  change.expected_behavior =
    "Native requests validate and pass through byte-for-byte; cross-provider transforms reject the toolset explicitly.";
  change.expected_diff_impact =
    "Generated bindings and focused native passthrough tests change; cross-provider snapshots do not.";
  change.generated_type_effect = status("affected");
  change.surfaces = {
    model_capabilities: status(),
    request_import: status("affected"),
    request_export: status("not_applicable"),
    response_import: status("not_applicable"),
    response_export: status("not_applicable"),
    streaming: status("not_applicable"),
    universal_semantics: status("not_applicable"),
    cross_provider: status("affected"),
  };
  change.mapping = {
    decision: "provider_only",
    rationale:
      "Browser execution and state belong to the provider-defined harness, not the portable universal model.",
  };
  change.implementation_targets = [
    "crates/generate-types/src/main.rs",
    "crates/lingua/src/providers/anthropic/convert.rs",
  ];
  change.tests = {
    unit: [
      "browser_toolset_native_request_is_passthrough",
      "browser_toolset_cross_provider_transform_is_rejected",
    ],
    payload_cases: [],
    live_capture: {
      required: false,
      rationale:
        "Provider-only harness features use offline native passthrough and rejection tests.",
    },
    commands: ["cargo test -p lingua anthropic"],
  };
  return plan;
}

test("accepts a complete plan and extracts unique capture cases", () => {
  const plan = validPlan();
  assert.deepEqual(validatePlan(plan, "anthropic"), []);
  plan.changes.push({
    ...structuredClone(plan.changes[0]),
    id: "second-change",
  });
  assert.deepEqual(captureCases(plan), ["anthropicContextWindowExceeded"]);
});

test("captures payload cases even when extra live evidence is not required", () => {
  const plan = validPlan();
  plan.changes[0].tests.live_capture.required = false;
  delete plan.changes[0].tests.live_capture.cases;

  assert.deepEqual(validatePlan(plan, "anthropic"), []);
  assert.deepEqual(captureCases(plan), ["anthropicContextWindowExceeded"]);
});

test("accepts a grouped non-semantic update without plan items", () => {
  const plan = validPlan();
  plan.changes = [];
  plan.non_semantic_changes = [
    "Grouped description and example churn has no generated wire-shape effect.",
  ];
  plan.audit_reports.capabilities =
    "Skipped because the compact inventory contains no capability changes.";
  plan.audit_reports.semantics =
    "Skipped because the compact inventory contains no semantic changes.";
  plan.audit_reports.coverage =
    "Skipped because no behavior requires additional coverage.";

  assert.deepEqual(validatePlan(plan, "anthropic"), []);
  assert.deepEqual(captureCases(plan), []);
});

test("accepts provider-only harness changes without a human blocker", () => {
  const plan = providerOnlyPlan();

  assert.deepEqual(validatePlan(plan, "anthropic"), []);
  assert.equal(hasBlockers(plan), false);
  assert.deepEqual(captureCases(plan), []);
});

test("provider-only changes cannot affect universal semantics", () => {
  const plan = providerOnlyPlan();
  plan.changes[0].surfaces.universal_semantics = status("affected");
  plan.changes[0].implementation_targets.push(
    "crates/lingua/src/universal/message.rs"
  );

  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /provider_only.*universal_semantics.*not_applicable/
  );
});

test("provider-only changes require explicit cross-provider rejection", () => {
  const plan = providerOnlyPlan();
  plan.changes[0].surfaces.cross_provider = status("not_affected");

  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /provider_only.*cross_provider.*affected/
  );
});

test("provider-only changes require focused native and rejection tests", () => {
  const plan = providerOnlyPlan();
  plan.changes[0].tests.unit = [];

  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /provider_only.*focused unit tests/
  );
});

test("provider-only changes do not schedule payload or live captures", () => {
  const plan = providerOnlyPlan();
  plan.changes[0].tests.payload_cases = ["anthropicBrowserToolsetParam"];
  plan.changes[0].tests.live_capture = {
    required: true,
    cases: ["anthropicBrowserToolsetParam"],
    rationale: "Exercise the provider endpoint.",
  };

  const errors = validatePlan(plan, "anthropic").join("\n");
  assert.match(errors, /provider_only.*payload_cases.*empty/);
  assert.match(errors, /provider_only.*live_capture.required.*false/);
  assert.deepEqual(captureCases(plan), []);
});

test("provider-only changes cannot target universal, expected-difference, or payload files", () => {
  for (const target of [
    "crates/lingua/src/universal/message.rs",
    "crates/coverage-report/src/requests_expected_differences.json",
    "payloads/cases/params.ts",
  ]) {
    const plan = providerOnlyPlan();
    plan.changes[0].implementation_targets.push(target);

    assert.match(
      validatePlan(plan, "anthropic").join("\n"),
      /provider_only.*must not target universal types, expected-difference files, or payload artifacts/
    );
  }
});

test("provider-only changes cannot be reported as human blockers", () => {
  const plan = providerOnlyPlan();
  plan.blockers = [
    {
      change_id: "browser-toolset",
      question: "What universal browser representation should be added?",
      evidence: "No universal browser type exists.",
      recommendation: "Add a universal browser type.",
      alternatives: ["Reject the feature."],
      affected_files: ["crates/lingua/src/universal/message.rs"],
      validation_commands: ["cargo test -p lingua anthropic"],
    },
  ];

  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /provider_only.*must not be listed as a human blocker/
  );
});

test("renders provider-only scope decisions for the PR body", () => {
  const markdown = renderProviderOnly(providerOnlyPlan());

  assert.match(markdown, /## Provider-only changes/);
  assert.match(markdown, /BrowserToolset20260801/);
  assert.match(markdown, /pass through byte-for-byte/);
  assert.match(markdown, /cross-provider transforms reject/);
  assert.match(markdown, /provider-defined harness/);
});

test("requires evidence for an empty semantic change list", () => {
  const plan = validPlan();
  plan.changes = [];

  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /must contain evidence when there are no semantic changes/
  );
});

test("rejects a semantic change without payload coverage", () => {
  const plan = validPlan();
  plan.changes[0].tests.payload_cases = [];
  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /must include a payload case/
  );
});

for (const target of [
  "`.github/workflows/ci.yml`: generated type check",
  "`.github/scripts/provider-type-update-plan.mjs`: loosen validation",
  "`pipelines/generate-provider-types.sh`: add a provider workaround",
  "`scripts/regenerate-typescript-bindings.mjs`: add a drift check",
  "`Makefile`: add a generated type target",
  "`AGENTS.md`: add provider-specific generation instructions",
]) {
  test(`rejects shared automation implementation target ${target}`, () => {
    const plan = validPlan();
    plan.changes[0].implementation_targets.push(target);
    assert.match(
      validatePlan(plan, "anthropic").join("\n"),
      /must not target shared workflow, pipeline, script, Makefile, or AGENTS\.md infrastructure/
    );
  });
}

test("reports invalid implementation target types without throwing", () => {
  const plan = validPlan();
  plan.changes[0].implementation_targets.push(null);
  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /implementation_targets/
  );
});

test("requires a question for blocked mappings", () => {
  const plan = validPlan();
  plan.changes[0].mapping.decision = "blocked";
  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /must include a blocker/
  );
});

test("accepts a structurally complete blocked plan for reporting", () => {
  const plan = validPlan();
  plan.changes[0].mapping.decision = "blocked";
  plan.blockers = [
    {
      change_id: "context-window-stop-reason",
      question:
        "Which universal incomplete reason should represent this value?",
      evidence:
        "The provider value has no existing universal incomplete-reason equivalent.",
      recommendation:
        "Add an explicit universal context-window incomplete reason.",
      alternatives: [
        "Reject imports containing this value.",
        "Keep the provider type without universal conversion support.",
      ],
      affected_files: [
        "crates/lingua/src/universal/response.rs",
        "crates/lingua/src/providers/anthropic/adapter.rs",
      ],
      validation_commands: [
        "cargo test -p lingua anthropic",
        "make test-payloads",
      ],
    },
  ];
  assert.deepEqual(validatePlan(plan, "anthropic"), []);
});

test("blocked changes are excluded from live capture", () => {
  const plan = validPlan();
  plan.blockers = [
    {
      change_id: "context-window-stop-reason",
      question:
        "Which universal incomplete reason should represent this value?",
      evidence:
        "The provider value has no existing universal incomplete-reason equivalent.",
      recommendation:
        "Add an explicit universal context-window incomplete reason.",
      alternatives: ["Reject imports containing this value."],
      affected_files: ["crates/lingua/src/universal/response.rs"],
      validation_commands: ["cargo test -p lingua anthropic"],
    },
  ];

  assert.equal(hasBlockers(plan), true);
  assert.deepEqual(captureCases(plan), []);
});

test("renders a human decision checklist with actionable evidence", () => {
  const plan = validPlan();
  plan.blockers = [
    {
      change_id: "context-window-stop-reason",
      question:
        "Which universal incomplete reason should represent this value?",
      evidence:
        "The provider value has no existing universal incomplete-reason equivalent.",
      recommendation:
        "Add an explicit universal context-window incomplete reason.",
      alternatives: ["Reject imports containing this value."],
      affected_files: ["crates/lingua/src/universal/response.rs"],
      validation_commands: ["cargo test -p lingua anthropic"],
    },
  ];

  const markdown = renderBlockers(plan);
  assert.match(markdown, /## Human decisions required/);
  assert.match(markdown, /StopReason/);
  assert.match(markdown, /Recommended option/);
  assert.match(markdown, /cargo test -p lingua anthropic/);
});

test("rejects verification passes with blocking findings", () => {
  const checks = Object.fromEntries(
    [
      "plan_coverage",
      "generated_source_integrity",
      "model_capabilities",
      "request_response_paths",
      "streaming_semantics",
      "universal_and_cross_provider",
      "focused_tests",
    ].map((name) => [name, { status: "pass", evidence: `${name} checked` }])
  );
  const report = {
    schema_version: 1,
    provider: "anthropic",
    verdict: "pass",
    checks,
    findings: [
      {
        severity: "blocking",
        title: "Missing serializer",
        evidence: "adapter.rs has no export arm",
        remediation: "Implement and test the response export.",
      },
    ],
  };
  assert.match(
    validateVerification(report, "anthropic").join("\n"),
    /cannot pass/
  );
});
