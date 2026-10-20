#!/usr/bin/env node

import { spawn, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  closeSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  validatePlan,
  validateVerification,
} from "./provider-type-update-plan.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const artifactRoot = process.env.RUNNER_TEMP || tmpdir();
const planPath =
  process.env.PROVIDER_UPDATE_PLAN_PATH ||
  join(artifactRoot, "provider-type-update-plan.json");
const verificationPath =
  process.env.PROVIDER_UPDATE_VERIFICATION_PATH ||
  join(artifactRoot, "provider-type-update-verification.json");
const providers = new Set(["openai", "anthropic", "google"]);

function saveJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.${process.pid}.tmp`;
  writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`);
  renameSync(temporary, path);
}

function loadPlan(provider) {
  const plan = JSON.parse(readFileSync(planPath, "utf8"));
  const errors = validatePlan(plan, provider);
  if (errors.length) throw new Error(errors.join("\n"));
  return plan;
}

function testCommand(command) {
  if (command.startsWith("cargo test -p coverage-report ")) return null;
  if (!command.startsWith("cargo test ")) return null;
  const words = command.trim().split(/\s+/);
  const [cargo, test, flag, pkg, ...args] = words;
  const invalid = () => {
    throw new Error(`Unsupported focused test command: ${command}`);
  };
  if (
    cargo !== "cargo" ||
    test !== "test" ||
    flag !== "-p" ||
    !["lingua", "generate-types"].includes(pkg)
  ) {
    invalid();
  }
  let index = 0;
  let target;
  if (args[index] === "--lib") index++;
  else if (args[index] === "--test") {
    target = args[index + 1];
    if (!target || !/^[A-Za-z0-9_-]+$/.test(target)) invalid();
    index += 2;
  }
  let filter;
  if (args[index] && args[index] !== "--") {
    filter = args[index++];
    if (!/^[A-Za-z0-9_][A-Za-z0-9_:.-]*$/.test(filter)) invalid();
  }
  if (args[index] === "--") {
    index++;
    while (["--exact", "--nocapture"].includes(args[index])) index++;
  }
  if (index !== args.length || (pkg === "lingua" && !target && !filter)) {
    invalid();
  }
  return words;
}

function itemChecks(plan, id) {
  const change = plan.changes.find((item) => item.id === id);
  if (!change) throw new Error(`Unknown change id: ${id}`);
  if (plan.blockers.some((blocker) => blocker.change_id === id)) {
    throw new Error(`Change ${id} is blocked by a human decision.`);
  }
  const checks = change.tests.commands.flatMap((command) => {
    const argv = testCommand(command);
    return argv ? [{ name: id, argv, tests: true }] : [];
  });
  const deferred = change.tests.commands
    .filter((command) => !testCommand(command))
    .map((command) => ({ change_id: id, command }));
  if (
    !checks.length &&
    (change.tests.unit.length || change.tests.offline_end_to_end.length)
  ) {
    throw new Error(
      `Change ${id} has named tests but no supported cargo test command.`
    );
  }
  return { checks, deferred };
}

function plannedChecks(plan) {
  const blocked = new Set(plan.blockers.map((blocker) => blocker.change_id));
  const items = plan.changes
    .filter((change) => !blocked.has(change.id))
    .map((change) => itemChecks(plan, change.id));
  return {
    checks: items.flatMap((item) => item.checks),
    deferred: items.flatMap((item) => item.deferred),
  };
}

function sourceState() {
  const git = (args) =>
    execFileSync("git", args, { cwd: root, maxBuffer: 64 * 1024 * 1024 });
  const digest = createHash("sha256");
  digest.update(git(["diff", "--no-ext-diff", "--binary", "HEAD", "--"]));
  const paths = git(["ls-files", "--others", "--exclude-standard", "-z"])
    .toString()
    .split("\0")
    .filter(Boolean)
    .sort();
  for (const path of paths) {
    digest.update(`${path}\0`);
    digest.update(readFileSync(join(root, path)));
  }
  return {
    revision: git(["rev-parse", "HEAD"]).toString().trim(),
    working_tree_sha256: digest.digest("hex"),
  };
}

async function execute(check, logPath, timeoutMs) {
  const fd = openSync(logPath, "w");
  const started = Date.now();
  let timedOut = false;
  let interrupted = false;
  let spawnError;
  let killTimer;
  const grouped = process.platform !== "win32";
  const child = spawn(check.argv[0], check.argv.slice(1), {
    cwd: root,
    env: { ...process.env, CARGO_TERM_COLOR: "never" },
    detached: grouped,
    stdio: ["ignore", fd, fd],
  });
  closeSync(fd);
  const kill = (signal) => {
    try {
      if (grouped && child.pid) process.kill(-child.pid, signal);
      else child.kill(signal);
    } catch (error) {
      if (error.code !== "ESRCH") throw error;
    }
  };
  const stop = () => {
    kill("SIGTERM");
    killTimer = setTimeout(() => kill("SIGKILL"), 2000);
  };
  const onInterrupt = () => {
    interrupted = true;
    stop();
  };
  process.once("SIGTERM", onInterrupt);
  process.once("SIGINT", onInterrupt);
  const timer = setTimeout(() => {
    timedOut = true;
    stop();
  }, timeoutMs);
  child.on("error", (error) => {
    spawnError = error.message;
  });
  const [exitCode, signal] = await new Promise((done) => {
    child.once("close", (code, terminatedBy) => done([code, terminatedBy]));
  });
  clearTimeout(timer);
  if (killTimer) clearTimeout(killTimer);
  if ((timedOut || interrupted) && grouped) kill("SIGKILL");
  process.removeListener("SIGTERM", onInterrupt);
  process.removeListener("SIGINT", onInterrupt);
  const output = readFileSync(logPath, "utf8");
  const tests = [
    ...output.matchAll(/^test (.+) \.\.\. (ok|FAILED|ignored)\b/gm),
  ].map((match) => ({ name: match[1], status: match[2] }));
  const passed = tests.filter((test) => test.status === "ok").length;
  const failed = tests.filter((test) => test.status === "FAILED").length;
  const error = timedOut
    ? `Validation exceeded its ${timeoutMs}ms remaining deadline.`
    : (interrupted ? "Validation was interrupted." : null) ||
      spawnError ||
      (exitCode !== 0 ? `Command exited with ${exitCode ?? signal}.` : null) ||
      (check.tests && !passed
        ? "No passing tests ran; check the test filter."
        : null) ||
      (failed ? `${failed} test(s) failed.` : null);
  return {
    name: check.name,
    argv: check.argv,
    status: error ? "fail" : "pass",
    exit_code: exitCode,
    signal,
    duration_ms: Date.now() - started,
    log_path: logPath,
    ...(check.tests ? { tests: { passed, failed, cases: tests } } : {}),
    ...(error ? { error, diagnostics: output.slice(-4000) } : {}),
  };
}

async function main([action, value, provider, ...extra]) {
  if (extra.length) throw new Error("Unexpected extra arguments.");
  if (["submit-plan", "submit-verification"].includes(action)) {
    if (!value || !providers.has(provider)) {
      throw new Error("Supply a candidate JSON path and provider.");
    }
    const candidate = JSON.parse(readFileSync(value, "utf8"));
    const isPlan = action === "submit-plan";
    const errors = isPlan
      ? validatePlan(candidate, provider)
      : validateVerification(candidate, provider);
    if (errors.length) throw new Error(errors.join("\n"));
    if (isPlan) plannedChecks(candidate);
    const path = isPlan ? planPath : verificationPath;
    saveJson(path, candidate);
    console.log(JSON.stringify({ status: "accepted", artifact_path: path }));
    return;
  }
  if (provider) throw new Error("Unexpected extra argument.");
  let checks;
  let deferred = [];
  let planStatus;
  if (action === "item") {
    ({ checks, deferred } = itemChecks(loadPlan(), value));
  } else if (action === "provider-tests") {
    if (!providers.has(value)) throw new Error(`Unknown provider: ${value}`);
    checks = [
      {
        name: `${value} provider suite`,
        argv: ["cargo", "test", "-p", "lingua", value],
        tests: true,
      },
    ];
    planStatus = existsSync(planPath) ? "available" : "unavailable";
    if (planStatus === "available") {
      const planned = plannedChecks(loadPlan(value));
      checks.push(...planned.checks);
      deferred = planned.deferred;
    }
  } else {
    if (value) throw new Error("Unexpected extra argument.");
    const commands = {
      "generator-tests": ["cargo", "test", "-p", "generate-types"],
      build: ["cargo", "build", "--all-features"],
      clippy: [
        "cargo",
        "clippy",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
      ],
    };
    if (!Object.hasOwn(commands, action)) {
      throw new Error(
        "Use item <change-id>, provider-tests <provider>, generator-tests, build, clippy, submit-plan <candidate> <provider>, or submit-verification <candidate> <provider>."
      );
    }
    checks = [
      {
        name: action,
        argv: commands[action],
        tests: action === "generator-tests",
      },
    ];
  }
  const timeoutMs = Number(
    process.env.PROVIDER_UPDATE_CHECK_TIMEOUT_MS || 480000
  );
  if (
    !Number.isSafeInteger(timeoutMs) ||
    timeoutMs <= 0 ||
    timeoutMs > 540000
  ) {
    throw new Error(
      "PROVIDER_UPDATE_CHECK_TIMEOUT_MS must be between 1 and 540000."
    );
  }
  const directory = resolve(
    process.env.PROVIDER_UPDATE_CHECK_DIR ||
      join(artifactRoot, "provider-type-update-checks")
  );
  if (!relative(root, directory).startsWith("..")) {
    throw new Error("Validation artifacts must be outside the repository.");
  }
  mkdirSync(directory, { recursive: true });
  const runDirectory = mkdtempSync(join(directory, `${action}-`));
  const reportPath = join(runDirectory, "result.json");
  const report = {
    schema_version: 1,
    action,
    ...(value ? { target: value } : {}),
    status: "running",
    source: sourceState(),
    ...(planStatus ? { plan: planStatus } : {}),
    deferred,
    report_path: reportPath,
    checks: [],
  };
  saveJson(reportPath, report);
  const deadline = Date.now() + timeoutMs;
  const seen = new Set();
  for (const check of checks) {
    const key = JSON.stringify(check.argv);
    if (seen.has(key)) continue;
    seen.add(key);
    const remaining = Math.max(1, deadline - Date.now());
    const result = await execute(
      check,
      join(runDirectory, `${report.checks.length + 1}.log`),
      remaining
    );
    report.checks.push(result);
    saveJson(reportPath, report);
    if (result.status === "fail") break;
  }
  report.status = report.checks.some((check) => check.status === "fail")
    ? "fail"
    : "pass";
  report.source_after = sourceState();
  if (JSON.stringify(report.source) !== JSON.stringify(report.source_after)) {
    report.status = "fail";
    report.error =
      "Sources changed during validation; rerun against the current files.";
  }
  saveJson(reportPath, report);
  const summary = {
    ...report,
    checks: report.checks.map((check) => ({
      ...check,
      ...(check.tests
        ? {
            tests: {
              ...check.tests,
              cases: check.tests.cases.slice(0, 50),
              truncated: check.tests.cases.length > 50,
            },
          }
        : {}),
    })),
  };
  console.log(JSON.stringify(summary, null, 2));
  if (report.status === "fail") process.exitCode = 1;
}

try {
  await main(process.argv.slice(2));
} catch (error) {
  console.log(JSON.stringify({ status: "fail", error: error.message }));
  process.exitCode = 1;
}
