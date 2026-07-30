import assert from "node:assert/strict";
import test from "node:test";

import {
  captureCases,
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

test("accepts a complete plan and extracts unique capture cases", () => {
  const plan = validPlan();
  assert.deepEqual(validatePlan(plan, "anthropic"), []);
  plan.changes.push({
    ...structuredClone(plan.changes[0]),
    id: "second-change",
  });
  assert.deepEqual(captureCases(plan), ["anthropicContextWindowExceeded"]);
});

test("rejects a semantic change without payload coverage", () => {
  const plan = validPlan();
  plan.changes[0].tests.payload_cases = [];
  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /must include a payload case/
  );
});

test("rejects implementation targets under GitHub workflows", () => {
  const plan = validPlan();
  plan.changes[0].implementation_targets.push(
    "` .github/workflows/ci.yml`: generated type check"
  );
  assert.match(
    validatePlan(plan, "anthropic").join("\n"),
    /must not target GitHub workflow definitions/
  );
});

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
    },
  ];
  assert.deepEqual(validatePlan(plan, "anthropic"), []);
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
