#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const PROVIDERS = new Set(["openai", "anthropic", "google"]);
const CHANGE_KINDS = new Set(["added", "changed", "removed"]);
const SURFACE_STATUSES = new Set([
  "affected",
  "not_affected",
  "not_applicable",
  "blocked",
]);
const MAPPING_DECISIONS = new Set([
  "map",
  "provider_only",
  "reject",
  "not_applicable",
  "blocked",
]);
const SURFACES = [
  "model_capabilities",
  "request_import",
  "request_export",
  "response_import",
  "response_export",
  "streaming",
  "universal_semantics",
  "cross_provider",
];
const PAYLOAD_SURFACES = [
  "request_import",
  "request_export",
  "response_import",
  "response_export",
  "streaming",
  "universal_semantics",
  "cross_provider",
];
const FORBIDDEN_IMPLEMENTATION_TARGET_FRAGMENTS = [
  ".github/workflows/",
  ".github/scripts/",
  "pipelines/",
  "scripts/",
  "Makefile",
  "AGENTS.md",
];
const PROVIDER_ONLY_FORBIDDEN_TARGET_FRAGMENTS = [
  "crates/lingua/src/universal/",
  "expected_differences",
  "payloads/",
];
const VERIFICATION_CHECKS = [
  "plan_coverage",
  "generated_source_integrity",
  "model_capabilities",
  "request_response_paths",
  "streaming_semantics",
  "universal_and_cross_provider",
  "focused_tests",
];

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function nonEmptyStringArray(value) {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    value.every((item) => nonEmptyString(item))
  );
}

function push(errors, condition, path, message) {
  if (!condition) {
    errors.push(`${path}: ${message}`);
  }
}

function validateEvidenceStatus(errors, value, path) {
  push(errors, isObject(value), path, "must be an object");
  if (!isObject(value)) return;
  push(
    errors,
    SURFACE_STATUSES.has(value.status),
    `${path}.status`,
    `must be one of ${[...SURFACE_STATUSES].join(", ")}`
  );
  push(
    errors,
    nonEmptyString(value.evidence),
    `${path}.evidence`,
    "must be a non-empty string"
  );
}

export function validatePlan(plan, expectedProvider) {
  const errors = [];
  push(errors, isObject(plan), "$", "must be an object");
  if (!isObject(plan)) return errors;

  push(errors, plan.schema_version === 1, "$.schema_version", "must equal 1");
  push(
    errors,
    PROVIDERS.has(plan.provider),
    "$.provider",
    "must be openai, anthropic, or google"
  );
  if (expectedProvider) {
    push(
      errors,
      plan.provider === expectedProvider,
      "$.provider",
      `must equal workflow provider '${expectedProvider}'`
    );
  }

  push(
    errors,
    isObject(plan.spec_review),
    "$.spec_review",
    "must be an object"
  );
  if (isObject(plan.spec_review)) {
    push(
      errors,
      plan.spec_review.complete === true,
      "$.spec_review.complete",
      "must be true after reviewing the complete specification diff"
    );
    push(
      errors,
      nonEmptyStringArray(plan.spec_review.evidence),
      "$.spec_review.evidence",
      "must contain file or symbol evidence"
    );
  }

  push(
    errors,
    isObject(plan.audit_reports),
    "$.audit_reports",
    "must be an object"
  );
  if (isObject(plan.audit_reports)) {
    for (const name of ["spec", "capabilities", "semantics", "coverage"]) {
      push(
        errors,
        nonEmptyString(plan.audit_reports[name]),
        `$.audit_reports.${name}`,
        "must summarize the corresponding audit with evidence"
      );
    }
  }

  push(errors, Array.isArray(plan.changes), "$.changes", "must be an array");
  if (Array.isArray(plan.changes) && plan.changes.length === 0) {
    push(
      errors,
      Array.isArray(plan.non_semantic_changes) &&
        plan.non_semantic_changes.length > 0 &&
        plan.non_semantic_changes.every(nonEmptyString),
      "$.non_semantic_changes",
      "must contain evidence when there are no semantic changes"
    );
  } else if (plan.non_semantic_changes !== undefined) {
    push(
      errors,
      Array.isArray(plan.non_semantic_changes) &&
        plan.non_semantic_changes.every(nonEmptyString),
      "$.non_semantic_changes",
      "must be an array of non-empty evidence strings"
    );
  }
  const ids = new Set();
  const blockedChangeIds = new Set();
  const providerOnlyChangeIds = new Set();
  if (Array.isArray(plan.changes)) {
    plan.changes.forEach((change, index) => {
      const path = `$.changes[${index}]`;
      push(errors, isObject(change), path, "must be an object");
      if (!isObject(change)) return;

      push(errors, nonEmptyString(change.id), `${path}.id`, "is required");
      if (nonEmptyString(change.id)) {
        push(errors, !ids.has(change.id), `${path}.id`, "must be unique");
        ids.add(change.id);
      }
      push(
        errors,
        CHANGE_KINDS.has(change.kind),
        `${path}.kind`,
        `must be one of ${[...CHANGE_KINDS].join(", ")}`
      );
      push(
        errors,
        isObject(change.source),
        `${path}.source`,
        "must be an object"
      );
      if (isObject(change.source)) {
        for (const field of ["path", "symbol", "summary"]) {
          push(
            errors,
            nonEmptyString(change.source[field]),
            `${path}.source.${field}`,
            "must be a non-empty string"
          );
        }
      }
      for (const field of [
        "root_cause",
        "expected_behavior",
        "expected_diff_impact",
      ]) {
        push(
          errors,
          nonEmptyString(change[field]),
          `${path}.${field}`,
          "must be a non-empty string"
        );
      }

      validateEvidenceStatus(
        errors,
        change.generated_type_effect,
        `${path}.generated_type_effect`
      );
      push(
        errors,
        isObject(change.surfaces),
        `${path}.surfaces`,
        "must be an object"
      );
      if (isObject(change.surfaces)) {
        for (const surface of SURFACES) {
          validateEvidenceStatus(
            errors,
            change.surfaces[surface],
            `${path}.surfaces.${surface}`
          );
          if (change.surfaces[surface]?.status === "blocked") {
            blockedChangeIds.add(change.id);
          }
        }
      }

      push(
        errors,
        isObject(change.mapping),
        `${path}.mapping`,
        "must be an object"
      );
      if (isObject(change.mapping)) {
        push(
          errors,
          MAPPING_DECISIONS.has(change.mapping.decision),
          `${path}.mapping.decision`,
          `must be one of ${[...MAPPING_DECISIONS].join(", ")}`
        );
        push(
          errors,
          nonEmptyString(change.mapping.rationale),
          `${path}.mapping.rationale`,
          "must be a non-empty string"
        );
        if (change.mapping.decision === "blocked") {
          blockedChangeIds.add(change.id);
        }
      }

      const providerOnly = change.mapping?.decision === "provider_only";
      if (providerOnly) {
        providerOnlyChangeIds.add(change.id);
        push(
          errors,
          change.generated_type_effect?.status === "affected",
          `${path}.generated_type_effect.status`,
          "provider_only changes must keep generated provider types current"
        );
        push(
          errors,
          change.surfaces?.universal_semantics?.status === "not_applicable",
          `${path}.surfaces.universal_semantics.status`,
          "provider_only changes require universal_semantics to be not_applicable"
        );
        push(
          errors,
          change.surfaces?.cross_provider?.status === "affected",
          `${path}.surfaces.cross_provider.status`,
          "provider_only changes require cross_provider to be affected by explicit rejection"
        );
        push(
          errors,
          ["request_import", "response_import", "streaming"].some(
            (surface) => change.surfaces?.[surface]?.status === "affected"
          ),
          `${path}.surfaces`,
          "provider_only changes must mark the relevant import or streaming path affected"
        );
        push(
          errors,
          !SURFACES.some(
            (surface) => change.surfaces?.[surface]?.status === "blocked"
          ),
          `${path}.surfaces`,
          "provider_only changes must not create human mapping blockers"
        );
      }

      const affectedSurfaces = SURFACES.filter(
        (surface) => change.surfaces?.[surface]?.status === "affected"
      );
      const payloadAffected = PAYLOAD_SURFACES.some(
        (surface) => change.surfaces?.[surface]?.status === "affected"
      );
      push(
        errors,
        affectedSurfaces.length === 0 ||
          nonEmptyStringArray(change.implementation_targets),
        `${path}.implementation_targets`,
        "must list targets when any surface is affected"
      );
      if (Array.isArray(change.implementation_targets)) {
        change.implementation_targets.forEach((target, targetIndex) => {
          const targetsAutomationInfrastructure =
            typeof target === "string" &&
            FORBIDDEN_IMPLEMENTATION_TARGET_FRAGMENTS.some((fragment) =>
              target.includes(fragment)
            );
          push(
            errors,
            !targetsAutomationInfrastructure,
            `${path}.implementation_targets[${targetIndex}]`,
            "must not target shared workflow, pipeline, script, Makefile, or AGENTS.md infrastructure"
          );
          const violatesProviderOnlyBoundary =
            providerOnly &&
            typeof target === "string" &&
            PROVIDER_ONLY_FORBIDDEN_TARGET_FRAGMENTS.some((fragment) =>
              target.includes(fragment)
            );
          push(
            errors,
            !violatesProviderOnlyBoundary,
            `${path}.implementation_targets[${targetIndex}]`,
            "provider_only changes must not target universal types, expected-difference files, or payload artifacts"
          );
        });
      }

      push(
        errors,
        isObject(change.tests),
        `${path}.tests`,
        "must be an object"
      );
      if (isObject(change.tests)) {
        push(
          errors,
          Array.isArray(change.tests.unit) &&
            change.tests.unit.every(nonEmptyString),
          `${path}.tests.unit`,
          "must be an array of strings"
        );
        push(
          errors,
          Array.isArray(change.tests.payload_cases) &&
            change.tests.payload_cases.every(nonEmptyString),
          `${path}.tests.payload_cases`,
          "must be an array of strings"
        );
        push(
          errors,
          Array.isArray(change.tests.commands) &&
            change.tests.commands.every(nonEmptyString),
          `${path}.tests.commands`,
          "must be an array of strings"
        );
        push(
          errors,
          affectedSurfaces.length === 0 ||
            nonEmptyStringArray(change.tests.commands),
          `${path}.tests.commands`,
          "must list validation commands when any surface is affected"
        );
        push(
          errors,
          change.surfaces?.model_capabilities?.status !== "affected" ||
            nonEmptyStringArray(change.tests.unit),
          `${path}.tests.unit`,
          "must include a focused test when model capabilities are affected"
        );
        push(
          errors,
          providerOnly ||
            !payloadAffected ||
            nonEmptyStringArray(change.tests.payload_cases),
          `${path}.tests.payload_cases`,
          "must include a payload case when a semantic data path is affected"
        );
        if (providerOnly) {
          push(
            errors,
            nonEmptyStringArray(change.tests.unit) &&
              change.tests.unit.length >= 2,
            `${path}.tests.unit`,
            "provider_only changes must include focused unit tests for both native passthrough and cross-provider rejection"
          );
          push(
            errors,
            Array.isArray(change.tests.payload_cases) &&
              change.tests.payload_cases.length === 0,
            `${path}.tests.payload_cases`,
            "provider_only changes require payload_cases to be empty"
          );
        }

        const live = change.tests.live_capture;
        push(
          errors,
          isObject(live),
          `${path}.tests.live_capture`,
          "must be an object"
        );
        if (isObject(live)) {
          push(
            errors,
            typeof live.required === "boolean",
            `${path}.tests.live_capture.required`,
            "must be a boolean"
          );
          push(
            errors,
            live.cases === undefined ||
              (Array.isArray(live.cases) && live.cases.every(nonEmptyString)),
            `${path}.tests.live_capture.cases`,
            "must be an array of case names when present"
          );
          push(
            errors,
            nonEmptyString(live.rationale),
            `${path}.tests.live_capture.rationale`,
            "must explain why capture is or is not required"
          );
          push(
            errors,
            live.required !== true || nonEmptyStringArray(live.cases),
            `${path}.tests.live_capture.cases`,
            "must contain a case when live capture is required"
          );
          if (providerOnly) {
            push(
              errors,
              live.required === false,
              `${path}.tests.live_capture.required`,
              "provider_only changes require live_capture.required to be false"
            );
            push(
              errors,
              live.cases === undefined ||
                (Array.isArray(live.cases) && live.cases.length === 0),
              `${path}.tests.live_capture.cases`,
              "provider_only changes must not schedule live capture cases"
            );
          }
        }
      }
    });
  }

  push(errors, Array.isArray(plan.blockers), "$.blockers", "must be an array");
  const blockerIds = new Set();
  if (Array.isArray(plan.blockers)) {
    plan.blockers.forEach((blocker, index) => {
      const path = `$.blockers[${index}]`;
      push(errors, isObject(blocker), path, "must be an object");
      if (!isObject(blocker)) return;
      push(
        errors,
        nonEmptyString(blocker.change_id),
        `${path}.change_id`,
        "is required"
      );
      push(
        errors,
        ids.has(blocker.change_id),
        `${path}.change_id`,
        "must reference a change id"
      );
      push(
        errors,
        !providerOnlyChangeIds.has(blocker.change_id),
        `${path}.change_id`,
        "provider_only changes must not be listed as a human blocker"
      );
      push(
        errors,
        nonEmptyString(blocker.question),
        `${path}.question`,
        "must be a non-empty string"
      );
      for (const field of ["evidence", "recommendation"]) {
        push(
          errors,
          nonEmptyString(blocker[field]),
          `${path}.${field}`,
          "must be a non-empty string"
        );
      }
      for (const field of [
        "alternatives",
        "affected_files",
        "validation_commands",
      ]) {
        push(
          errors,
          nonEmptyStringArray(blocker[field]),
          `${path}.${field}`,
          "must be a non-empty array of strings"
        );
      }
      blockerIds.add(blocker.change_id);
    });
  }
  for (const id of blockedChangeIds) {
    push(
      errors,
      blockerIds.has(id),
      "$.blockers",
      `must include a blocker for change '${id}'`
    );
  }

  return errors;
}

export function hasBlockers(plan) {
  return Array.isArray(plan.blockers) && plan.blockers.length > 0;
}

export function captureCases(plan) {
  const cases = new Set();
  const blockerIds = new Set(
    (plan.blockers ?? []).map((blocker) => blocker.change_id)
  );
  for (const change of plan.changes ?? []) {
    if (blockerIds.has(change.id)) continue;
    if (change.mapping?.decision === "provider_only") continue;
    for (const name of change.tests?.payload_cases ?? []) {
      cases.add(name);
    }
    if (change.tests?.live_capture?.required) {
      for (const name of change.tests.live_capture.cases ?? []) {
        cases.add(name);
      }
    }
  }
  return [...cases].sort();
}

export function renderBlockers(plan) {
  if (!hasBlockers(plan)) return "";

  const changes = new Map(
    (plan.changes ?? []).map((change) => [change.id, change])
  );
  const lines = [
    "## Human decisions required",
    "",
    "The agent completed every unblocked item. These decisions require human input before the update can be finished:",
    "",
  ];

  for (const blocker of plan.blockers) {
    const change = changes.get(blocker.change_id);
    const label = change?.source?.symbol ?? blocker.change_id;
    lines.push(`- [ ] \`${label}\` (\`${blocker.change_id}\`)`);
    lines.push(`  - **Question:** ${blocker.question}`);
    lines.push(`  - **Evidence:** ${blocker.evidence}`);
    lines.push(`  - **Recommended option:** ${blocker.recommendation}`);
    lines.push("  - **Alternatives:**");
    for (const alternative of blocker.alternatives) {
      lines.push(`    - ${alternative}`);
    }
    lines.push("  - **Likely files:**");
    for (const file of blocker.affected_files) {
      lines.push(`    - \`${file}\``);
    }
    lines.push("  - **Validation commands:**");
    for (const command of blocker.validation_commands) {
      lines.push(`    - \`${command}\``);
    }
    lines.push("");
  }

  return lines.join("\n").trimEnd();
}

export function renderProviderOnly(plan) {
  const changes = (plan.changes ?? []).filter(
    (change) => change.mapping?.decision === "provider_only"
  );
  if (changes.length === 0) return "";

  const lines = [
    "## Provider-only changes",
    "",
    "These native wire features are accepted and passed through unchanged, but cross-provider transformation is intentionally unsupported:",
    "",
  ];

  for (const change of changes) {
    const label = change.source?.symbol ?? change.id;
    lines.push(`- \`${label}\` (\`${change.id}\`)`);
    lines.push(`  - **Native contract:** ${change.expected_behavior}`);
    lines.push(`  - **Scope decision:** ${change.mapping.rationale}`);
  }

  return lines.join("\n").trimEnd();
}

export function validateVerification(report, expectedProvider) {
  const errors = [];
  push(errors, isObject(report), "$", "must be an object");
  if (!isObject(report)) return errors;
  push(errors, report.schema_version === 1, "$.schema_version", "must equal 1");
  push(
    errors,
    report.provider === expectedProvider,
    "$.provider",
    `must equal workflow provider '${expectedProvider}'`
  );
  push(
    errors,
    report.verdict === "pass" || report.verdict === "fail",
    "$.verdict",
    "must be pass or fail"
  );
  push(errors, isObject(report.checks), "$.checks", "must be an object");
  if (isObject(report.checks)) {
    for (const name of VERIFICATION_CHECKS) {
      const check = report.checks[name];
      push(errors, isObject(check), `$.checks.${name}`, "must be an object");
      if (isObject(check)) {
        push(
          errors,
          check.status === "pass" || check.status === "fail",
          `$.checks.${name}.status`,
          "must be pass or fail"
        );
        push(
          errors,
          nonEmptyString(check.evidence),
          `$.checks.${name}.evidence`,
          "must be a non-empty string"
        );
      }
    }
  }
  push(
    errors,
    Array.isArray(report.findings),
    "$.findings",
    "must be an array"
  );
  if (Array.isArray(report.findings)) {
    report.findings.forEach((finding, index) => {
      const path = `$.findings[${index}]`;
      push(errors, isObject(finding), path, "must be an object");
      if (!isObject(finding)) return;
      push(
        errors,
        finding.severity === "blocking" || finding.severity === "warning",
        `${path}.severity`,
        "must be blocking or warning"
      );
      for (const field of ["title", "evidence", "remediation"]) {
        push(
          errors,
          nonEmptyString(finding[field]),
          `${path}.${field}`,
          "must be a non-empty string"
        );
      }
    });
  }

  const failedCheck = Object.values(report.checks ?? {}).some(
    (check) => check?.status === "fail"
  );
  const blockingFinding = (report.findings ?? []).some(
    (finding) => finding?.severity === "blocking"
  );
  push(
    errors,
    report.verdict !== "pass" || (!failedCheck && !blockingFinding),
    "$.verdict",
    "cannot pass with a failed check or blocking finding"
  );
  return errors;
}

function loadJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(`Cannot read JSON from ${path}: ${error.message}`);
  }
}

function printErrors(kind, errors) {
  console.error(`${kind} validation failed:`);
  for (const error of errors) {
    console.error(`- ${error}`);
  }
}

function main(argv) {
  const [command, path, provider] = argv;
  if (!command || !path) {
    throw new Error(
      "Usage: provider-type-update-plan.mjs validate|validate-structure|has-blockers|render-blockers|render-provider-only|capture-cases|verify|verify-structure|verdict <path> [provider]"
    );
  }
  const value = loadJson(path);

  if (command === "validate" || command === "validate-structure") {
    const errors = validatePlan(value, provider);
    if (errors.length) {
      printErrors("Plan", errors);
      process.exitCode = 1;
    } else if (command === "validate" && hasBlockers(value)) {
      console.error("Plan is valid but has unresolved blockers:");
      for (const blocker of value.blockers) {
        console.error(`- ${blocker.change_id}: ${blocker.question}`);
      }
      process.exitCode = 1;
    } else {
      console.log(
        `Validated ${value.changes.length} ${value.provider} plan item(s).`
      );
    }
    return;
  }
  if (command === "has-blockers") {
    const errors = validatePlan(value, provider);
    if (errors.length) {
      printErrors("Plan", errors);
      process.exitCode = 1;
    } else {
      console.log(hasBlockers(value) ? "true" : "false");
    }
    return;
  }
  if (command === "render-blockers") {
    const errors = validatePlan(value, provider);
    if (errors.length) {
      printErrors("Plan", errors);
      process.exitCode = 1;
    } else {
      console.log(renderBlockers(value));
    }
    return;
  }
  if (command === "render-provider-only") {
    const errors = validatePlan(value, provider);
    if (errors.length) {
      printErrors("Plan", errors);
      process.exitCode = 1;
    } else {
      console.log(renderProviderOnly(value));
    }
    return;
  }
  if (command === "capture-cases") {
    const errors = validatePlan(value, provider);
    if (errors.length) {
      printErrors("Plan", errors);
      process.exitCode = 1;
    } else {
      console.log(captureCases(value).join(","));
    }
    return;
  }
  if (command === "verify" || command === "verify-structure") {
    const errors = validateVerification(value, provider);
    if (errors.length) {
      printErrors("Verification report", errors);
      process.exitCode = 1;
    } else if (command === "verify" && value.verdict !== "pass") {
      console.error("Verification verdict is fail.");
      process.exitCode = 1;
    } else {
      console.log(
        command === "verify"
          ? `Verification passed for ${value.provider}.`
          : `Verification report is structurally valid for ${value.provider}.`
      );
    }
    return;
  }
  if (command === "verdict") {
    const errors = validateVerification(value, provider);
    if (errors.length) {
      printErrors("Verification report", errors);
      process.exitCode = 1;
    } else {
      console.log(value.verdict);
    }
    return;
  }
  throw new Error(`Unknown command '${command}'`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
