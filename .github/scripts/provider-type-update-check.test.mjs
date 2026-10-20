import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

function plan() {
  const status = (value) => ({ status: value, evidence: "Fixture evidence" });
  return {
    schema_version: 1,
    provider: "google",
    spec_review: { complete: true, evidence: ["spec.json: Example"] },
    audit_reports: {
      spec: "Example field was added.",
      capabilities: "No capability changes.",
      semantics: "Existing mapping is sufficient.",
      coverage: "Pin the existing behavior with a regression test.",
    },
    changes: [
      {
        id: "example",
        kind: "added",
        source: {
          path: "spec.json",
          symbol: "Example",
          summary: "Adds Example.",
        },
        root_cause: "The new field uses the existing mapping.",
        expected_behavior: "Preserve the field.",
        expected_diff_impact: "New regression tests.",
        generated_type_effect: status("affected"),
        surfaces: Object.fromEntries(
          [
            "model_capabilities",
            "request_import",
            "request_export",
            "response_import",
            "response_export",
            "streaming",
            "universal_semantics",
            "cross_provider",
          ].map((name) => [name, status("not_affected")])
        ),
        mapping: {
          decision: "map",
          rationale: "Uses the existing representation.",
        },
        implementation_targets: [
          "crates/lingua/src/providers/google/convert.rs",
        ],
        tests: {
          unit: ["example_test"],
          offline_end_to_end: [],
          payload_cases: [],
          commands: ["cargo test -p lingua --lib example_test"],
          live_capture: {
            required: false,
            cases: [],
            rationale: "Offline regression.",
          },
        },
      },
    ],
    blockers: [],
  };
}

function fixture(t, cargoBody) {
  const directory = mkdtempSync(join(tmpdir(), "lingua-check-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const repo = join(directory, "repo");
  const scripts = join(repo, ".github/scripts");
  const artifacts = join(directory, "artifacts");
  const bin = join(directory, "bin");
  for (const path of [scripts, artifacts, bin])
    mkdirSync(path, { recursive: true });
  writeFileSync(join(bin, "package.json"), '{"type":"module"}\n');
  for (const name of [
    "provider-type-update-plan.mjs",
    "provider-type-update-check.mjs",
  ]) {
    copyFileSync(new URL(name, import.meta.url), join(scripts, name));
  }
  const git = (args) => execFileSync("git", args, { cwd: repo, stdio: "pipe" });
  git(["init", "--quiet"]);
  git(["add", "."]);
  git([
    "-c",
    "user.name=Test",
    "-c",
    "user.email=test@example.com",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
    "--quiet",
    "-m",
    "fixture",
  ]);
  writeFileSync(join(bin, "cargo"), `#!${process.execPath}\n${cargoBody}\n`, {
    mode: 0o755,
  });
  const planPath = join(artifacts, "provider-type-update-plan.json");
  writeFileSync(planPath, JSON.stringify(plan()));
  const run = (args, env = {}) => {
    const result = spawnSync(
      process.execPath,
      [join(scripts, "provider-type-update-check.mjs"), ...args],
      {
        cwd: directory,
        env: {
          ...process.env,
          PATH: `${bin}:${process.env.PATH}`,
          RUNNER_TEMP: artifacts,
          PROVIDER_UPDATE_PLAN_PATH: planPath,
          PROVIDER_UPDATE_VERIFICATION_PATH: join(
            artifacts,
            "provider-type-update-verification.json"
          ),
          PROVIDER_UPDATE_CHECK_DIR: join(
            artifacts,
            "provider-type-update-checks"
          ),
          ...env,
        },
        encoding: "utf8",
        timeout: 10000,
      }
    );
    assert.equal(result.error, undefined);
    return { code: result.status, output: JSON.parse(result.stdout) };
  };
  return { directory, repo, artifacts, planPath, run };
}

const passing = `
console.log("cwd=" + process.cwd());
console.log("test example_test ... ok");
console.log("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out");
`;

test("runs focused tests from the repository and saves source-bound evidence and logs", (t) => {
  const f = fixture(t, passing);
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 0);
  assert.equal(result.output.status, "pass");
  const check = result.output.checks[0];
  assert.deepEqual(check.argv, [
    "cargo",
    "test",
    "-p",
    "lingua",
    "--lib",
    "example_test",
  ]);
  assert.equal(check.tests.passed, 1);
  assert.deepEqual(check.tests.cases, [{ name: "example_test", status: "ok" }]);
  assert.ok(
    readFileSync(check.log_path, "utf8").startsWith(
      `cwd=${realpathSync(f.repo)}\n`
    )
  );
  assert.match(result.output.source.revision, /^[a-f0-9]{40}$/);
  assert.match(result.output.source.working_tree_sha256, /^[a-f0-9]{64}$/);
  const saved = JSON.parse(readFileSync(result.output.report_path, "utf8"));
  assert.equal(saved.status, "pass");
  writeFileSync(join(f.repo, "new-test.rs"), "new test source");
  const changed = f.run(["item", "example"]);
  assert.notEqual(
    changed.output.source.working_tree_sha256,
    result.output.source.working_tree_sha256
  );
});

test("rejects a successful cargo exit when no tests actually ran", (t) => {
  const f = fixture(
    t,
    'console.log("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 42 filtered out");'
  );
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 1);
  assert.match(result.output.checks[0].error, /No passing tests ran/);
  assert.equal(result.output.checks[0].exit_code, 0);
});

test("preserves nonzero exit codes and complete diagnostics in the log", (t) => {
  const f = fixture(
    t,
    `console.error("x".repeat(9000)); console.error("compiler failure"); process.exitCode = 23;`
  );
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 1);
  const check = result.output.checks[0];
  assert.equal(check.exit_code, 23);
  assert.match(check.error, /23/);
  assert.equal(check.diagnostics.length, 4000);
  assert.match(check.diagnostics, /compiler failure/);
  assert.ok(readFileSync(check.log_path, "utf8").length > 9000);
});

test("supports integration targets and never interprets shell syntax from the plan", (t) => {
  const f = fixture(t, passing);
  const value = plan();
  value.changes[0].tests.commands = [
    "cargo test -p lingua --test example_target",
  ];
  writeFileSync(f.planPath, JSON.stringify(value));
  assert.equal(f.run(["item", "example"]).code, 0);
  value.changes[0].tests.commands = [
    "cargo test -p lingua example_test; touch injected",
  ];
  writeFileSync(f.planPath, JSON.stringify(value));
  const rejected = f.run(["item", "example"]);
  assert.equal(rejected.code, 1);
  assert.match(rejected.output.error, /Unsupported focused test command/);
  assert.equal(existsSync(join(f.repo, "injected")), false);
});

test("refuses unknown and blocked items before running commands", (t) => {
  const f = fixture(t, passing);
  assert.match(f.run(["item", "missing"]).output.error, /Unknown change id/);
  const value = plan();
  value.changes[0].mapping.decision = "blocked";
  value.blockers = [
    {
      change_id: "example",
      question: "Which representation?",
      evidence: "Ambiguous field.",
      recommendation: "Choose a representation.",
      alternatives: ["Reject the feature."],
      affected_files: ["convert.rs"],
      validation_commands: ["cargo test -p lingua example_test"],
    },
  ];
  writeFileSync(f.planPath, JSON.stringify(value));
  assert.match(
    f.run(["item", "example"]).output.error,
    /blocked by a human decision/
  );
  assert.equal(
    existsSync(join(f.artifacts, "provider-type-update-checks")),
    false
  );
});

test("CI checks the provider suite plus the plan's focused commands", (t) => {
  const f = fixture(t, passing);
  const result = f.run(["provider-tests", "google"]);
  assert.equal(result.code, 0);
  assert.equal(result.output.checks.length, 2);
  assert.deepEqual(result.output.checks[0].argv, [
    "cargo",
    "test",
    "-p",
    "lingua",
    "google",
  ]);
  assert.deepEqual(result.output.checks[1].argv, [
    "cargo",
    "test",
    "-p",
    "lingua",
    "--lib",
    "example_test",
  ]);
  assert.equal(f.run(["provider-tests", "anthropic"]).code, 1);
  rmSync(f.planPath);
  const noPlan = f.run(["provider-tests", "google"]);
  assert.equal(noPlan.code, 0);
  assert.equal(noPlan.output.plan, "unavailable");
  assert.equal(noPlan.output.checks.length, 1);
});

test("reports live captures and cross-provider guards as deferred rather than executing them", (t) => {
  const f = fixture(t, passing);
  const value = plan();
  const deferred = [
    "make capture FILTER=example",
    "cargo test -p coverage-report --test cross_provider_test cross_provider_transformations_have_no_unexpected_failures",
  ];
  value.changes[0].tests.commands.push(...deferred);
  writeFileSync(f.planPath, JSON.stringify(value));
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 0);
  assert.equal(result.output.checks.length, 1);
  assert.deepEqual(
    result.output.deferred,
    deferred.map((command) => ({ change_id: "example", command }))
  );
});

test("atomically accepts valid artifacts and leaves an accepted plan intact on validation failure", (t) => {
  const f = fixture(t, passing);
  const candidate = join(f.artifacts, "candidate.json");
  const value = plan();
  writeFileSync(candidate, JSON.stringify(value));
  const accepted = f.run(["submit-plan", candidate, "google"]);
  assert.equal(accepted.code, 0);
  assert.equal(accepted.output.artifact_path, f.planPath);
  assert.deepEqual(JSON.parse(readFileSync(f.planPath, "utf8")), value);
  value.schema_version = 99;
  writeFileSync(candidate, JSON.stringify(value));
  assert.equal(f.run(["submit-plan", candidate, "google"]).code, 1);
  assert.equal(JSON.parse(readFileSync(f.planPath, "utf8")).schema_version, 1);
  assert.equal(
    readdirSync(f.artifacts).some((name) => name.endsWith(".tmp")),
    false
  );
});

test("accepts a structurally valid fail verdict without turning it into a pass", (t) => {
  const f = fixture(t, passing);
  const report = {
    schema_version: 1,
    provider: "google",
    verdict: "fail",
    checks: Object.fromEntries(
      [
        "plan_coverage",
        "generated_source_integrity",
        "model_capabilities",
        "request_response_paths",
        "streaming_semantics",
        "universal_and_cross_provider",
        "focused_tests",
      ].map((name) => [
        name,
        { status: "pass", evidence: "Reviewed fixture behavior." },
      ])
    ),
    findings: [
      {
        severity: "blocking",
        title: "Design needed",
        evidence: "Unresolved semantics.",
        remediation: "Choose the mapping.",
      },
    ],
  };
  const candidate = join(f.artifacts, "verification-candidate.json");
  writeFileSync(candidate, JSON.stringify(report));
  const result = f.run(["submit-verification", candidate, "google"]);
  assert.equal(result.code, 0);
  assert.equal(
    JSON.parse(readFileSync(result.output.artifact_path, "utf8")).verdict,
    "fail"
  );
  report.verdict = "pass";
  writeFileSync(candidate, JSON.stringify(report));
  assert.equal(f.run(["submit-verification", candidate, "google"]).code, 1);
  assert.equal(
    JSON.parse(readFileSync(result.output.artifact_path, "utf8")).verdict,
    "fail"
  );
});

test("rejects plan submission when named tests cannot be executed by the helper", (t) => {
  const f = fixture(t, passing);
  const value = plan();
  value.changes[0].tests.commands = [
    "cargo test -p lingua example_test | tail -5",
  ];
  const candidate = join(f.artifacts, "candidate.json");
  writeFileSync(candidate, JSON.stringify(value));
  const result = f.run(["submit-plan", candidate, "google"]);
  assert.equal(result.code, 1);
  assert.match(result.output.error, /Unsupported focused test command/);
  assert.deepEqual(JSON.parse(readFileSync(f.planPath, "utf8")), plan());
});

test("invalidates evidence if sources change while the checks are running", (t) => {
  const f = fixture(
    t,
    `import { writeFileSync } from "node:fs"; writeFileSync("changed.rs", "changed"); ${passing}`
  );
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 1);
  assert.equal(result.output.checks[0].status, "pass");
  assert.match(result.output.error, /Sources changed/);
  assert.notEqual(
    result.output.source.working_tree_sha256,
    result.output.source_after.working_tree_sha256
  );
});

test("keeps full test names in the saved report while bounding the tool response", (t) => {
  const f = fixture(
    t,
    `for (let i = 0; i < 70; i++) console.log("test case_" + i + " ... ok");`
  );
  const result = f.run(["item", "example"]);
  assert.equal(result.code, 0);
  assert.equal(result.output.checks[0].tests.passed, 70);
  assert.equal(result.output.checks[0].tests.cases.length, 50);
  assert.equal(result.output.checks[0].tests.truncated, true);
  const full = JSON.parse(readFileSync(result.output.report_path, "utf8"));
  assert.equal(full.checks[0].tests.cases.length, 70);
});

test("terminates a timed-out process group and preserves its diagnostics", async (t) => {
  const f = fixture(
    t,
    `
import { spawn } from "node:child_process";
spawn(process.execPath, ["-e", 'process.on("SIGTERM", () => {}); setTimeout(() => require("node:fs").writeFileSync("escaped", "bad"), 1000)'], { stdio: "ignore" });
console.log("waiting on child");
setInterval(() => {}, 1000);
`
  );
  const result = f.run(["item", "example"], {
    PROVIDER_UPDATE_CHECK_TIMEOUT_MS: "300",
  });
  assert.equal(result.code, 1);
  assert.match(result.output.checks[0].error, /deadline/);
  assert.match(result.output.checks[0].diagnostics, /waiting on child/);
  await new Promise((done) => setTimeout(done, 1100));
  assert.equal(existsSync(join(f.repo, "escaped")), false);
});
