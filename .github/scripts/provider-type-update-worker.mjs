// Runs only inside the isolated container. The controller and prompts are
// root-owned image files, separate from the agent-writable candidate checkout.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { applyProposal } from "./provider-type-update-patch.mjs";
import { checkEnvironment } from "./provider-type-update-security.mjs";
import {
  captureCases,
  renderImplementation,
  validatePlan,
  validateVerification,
} from "./provider-type-update-plan.mjs";

const [mode, provider] = process.argv.slice(2);
if (
  !["propose", "validate", "autofix"].includes(mode) ||
  !["openai", "anthropic", "google"].includes(provider)
) {
  throw new Error("Invalid worker mode or provider.");
}
const scratch = "/tmp/provider-type-update";
mkdirSync(scratch, { recursive: true });
const status = {};
const diagnostics = {};
const planPath = join(scratch, "provider-type-update-plan.json");
const verificationPath = join(
  scratch,
  "provider-type-update-verification.json"
);
const saveStatus = () =>
  writeFileSync(join(scratch, "status.json"), JSON.stringify(status, null, 2));

async function run(
  name,
  argv,
  { timeout = 600000, environment = {}, input } = {}
) {
  const output = [];
  let bytes = 0;
  let killed = false;
  const child = spawn(argv[0], argv.slice(1), {
    cwd: "/workspace",
    env: { ...checkEnvironment(), ...environment },
    detached: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const kill = () => {
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch (error) {
      if (error.code !== "ESRCH") throw error;
    }
  };
  const timer = setTimeout(() => {
    killed = true;
    kill();
  }, timeout);
  let spawnError;
  child.on("error", () => {
    spawnError = true;
  });
  for (const stream of [child.stdout, child.stderr])
    stream.on("data", (chunk) => {
      bytes += chunk.length;
      if (bytes < 16 * 1024 * 1024) output.push(chunk);
      else {
        killed = true;
        kill();
      }
    });
  child.stdin.on("error", () => {});
  child.stdin.end(input);
  const code = await new Promise((resolve) => child.once("close", resolve));
  clearTimeout(timer);
  kill();
  const text = Buffer.concat(output).toString("utf8");
  writeFileSync(join(scratch, `${name}.log`), text);
  status[name] = killed || spawnError || code !== 0 ? "fail" : "pass";
  if (status[name] === "fail") diagnostics[name] = text.slice(-4000);
  saveStatus();
  return { ok: status[name] === "pass", text };
}

async function phase(name, timeout, turns) {
  const prompt = readFileSync(
    `/opt/updater/prompts/${name}.md`,
    "utf8"
  ).replaceAll("{{provider}}", provider);
  // Tool permissions limit mistakes. Container isolation is the security
  // boundary even for arbitrary code executed through cargo, sed, or hooks.
  return run(
    name,
    [
      "claude",
      "-p",
      "--model",
      "claude-opus-5-5",
      "--effort",
      "medium",
      "--max-turns",
      String(turns),
      "--permission-mode",
      "bypassPermissions",
      "--add-dir",
      scratch,
      "--strict-mcp-config",
      "--mcp-config",
      '{"mcpServers":{}}',
      "--tools",
      name === "plan"
        ? "Agent,Skill,Read,Glob,Grep,Bash,Write"
        : "Skill,Read,Glob,Grep,Bash,Edit,Write",
    ],
    {
      timeout,
      input: prompt,
      environment: {
        ANTHROPIC_BASE_URL: process.env.ANTHROPIC_BASE_URL,
        ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
        CLAUDE_CODE_DISABLE_BACKGROUND_TASKS: "1",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
        DISABLE_AUTOUPDATER: "1",
        BASH_DEFAULT_TIMEOUT_MS: "600000",
        RUNNER_TEMP: scratch,
        PROVIDER_UPDATE_PLAN_PATH: planPath,
        PROVIDER_UPDATE_VERIFICATION_PATH: verificationPath,
      },
    }
  );
}

async function checks(prefix = "") {
  const commands = [
    ["rust-types", ["make", "generate-provider-types", `PROVIDER=${provider}`]],
    ["ts-types", ["make", "generate-types", `PROVIDER=${provider}`]],
    ["format", ["cargo", "fmt", "--all", "--", "--check"]],
    [
      "semantic-policy",
      ["node", "/opt/updater/provider-type-update-semantic-policy.mjs"],
    ],
    [
      "provider-tests",
      [
        "node",
        ".github/scripts/provider-type-update-check.mjs",
        "provider-tests",
        provider,
      ],
    ],
    [
      "generator-tests",
      [
        "node",
        ".github/scripts/provider-type-update-check.mjs",
        "generator-tests",
      ],
    ],
    [
      "build",
      ["node", ".github/scripts/provider-type-update-check.mjs", "build"],
    ],
    [
      "clippy",
      ["node", ".github/scripts/provider-type-update-check.mjs", "clippy"],
    ],
    [
      "payload-sync",
      [
        "pnpm",
        "--dir",
        "payloads",
        "exec",
        "vitest",
        "run",
        "scripts/sync.test.ts",
      ],
    ],
    ["payload-tests", ["make", "test-payloads"]],
    ["typed-boundaries", ["make", "typed-boundary-check"]],
    [
      "cross-provider",
      [
        "cargo",
        "test",
        "-p",
        "coverage-report",
        "--test",
        "cross_provider_test",
        "cross_provider_transformations_have_no_unexpected_failures",
      ],
    ],
  ];
  for (const [name, argv] of commands) {
    await run(prefix + name, argv, {
      timeout: 900000,
      environment: {
        RUNNER_TEMP: scratch,
        PROVIDER_UPDATE_PLAN_PATH: planPath,
      },
    });
  }
}

await run("git-init", ["git", "init", "--quiet"]);
await run("git-add", ["git", "add", "-A"]);
await run("git-baseline", [
  "git",
  "-c",
  "core.hooksPath=/dev/null",
  "-c",
  "user.name=Updater",
  "-c",
  "user.email=updater@localhost",
  "commit",
  "--quiet",
  "-m",
  "Trusted baseline",
]);
const installed = await run(
  "dependencies",
  [
    "pnpm",
    "install",
    "--frozen-lockfile",
    "--filter",
    "@lingua/quicktype-toolchain",
    "--filter",
    "@braintrust/lingua-wasm...",
    "--filter",
    "@braintrust/payload-capture...",
  ],
  { timeout: 900000 }
);
if (!installed.ok || status["git-baseline"] !== "pass")
  throw new Error("Worker setup failed.");

if (mode === "autofix") {
  writeFileSync(
    join(scratch, "autofix-review.json"),
    readFileSync("/artifacts/review.json")
  );
  await phase("autofix", 3600000, 100);
  await run("format-autofix", ["cargo", "fmt", "--all"]);
  await checks();
  await run("stage-proposal", ["git", "add", "-A"]);
  const patch = await run("proposal-patch", [
    "git",
    "diff",
    "--cached",
    "--binary",
    "--full-index",
    "--no-ext-diff",
    "--no-textconv",
    "HEAD",
  ]);
  writeFileSync("/artifacts/proposal.patch", patch.text);
} else if (mode === "validate") {
  applyProposal("/workspace", "/artifacts/proposal.patch", provider);
  if (existsSync("/artifacts/plan.json")) {
    const candidate = JSON.parse(readFileSync("/artifacts/plan.json", "utf8"));
    if (validatePlan(candidate, provider).length === 0)
      writeFileSync(planPath, JSON.stringify(candidate));
    else status.plan = "fail";
  } else status.plan = "fail";
  await checks();
  // Validation must not silently change or regenerate the submitted patch.
  await run("stage-validated", ["git", "add", "-A"]);
  const current = await run("validated-patch", [
    "git",
    "diff",
    "--cached",
    "--binary",
    "--full-index",
    "--no-ext-diff",
    "--no-textconv",
    "HEAD",
  ]);
  status["unchanged-patch"] =
    current.text === readFileSync("/artifacts/proposal.patch", "utf8")
      ? "pass"
      : "fail";
} else {
  const generated = await run(
    "generation",
    ["./pipelines/generate-provider-types.sh", provider],
    { timeout: 900000 }
  );
  if (!generated.ok) {
    const retry = await run(
      "generation-retry",
      ["./pipelines/generate-provider-types.sh", provider],
      { timeout: 900000 }
    );
    if (!retry.ok) {
      await phase("repair", 1800000, 60);
      await run(
        "generation-repaired",
        ["make", "generate-provider-types", `PROVIDER=${provider}`],
        { timeout: 900000 }
      );
    }
  }
  await run(
    "initial-ts-types",
    ["make", "generate-types", `PROVIDER=${provider}`],
    { timeout: 900000 }
  );
  const changed = await run("changes", ["git", "status", "--porcelain"]);
  if (changed.text.trim()) {
    await phase("plan", 1800000, 50);
    let plan;
    try {
      plan = JSON.parse(readFileSync(planPath, "utf8"));
      if (validatePlan(plan, provider).length) throw new Error("Invalid plan");
      status.plan = "pass";
    } catch {
      status.plan = "fail";
    }
    if (status.plan === "pass") {
      const accepted = readFileSync(planPath, "utf8");
      writeFileSync("plan.md", renderImplementation(plan));
      await phase("implement", 2400000, 100);
      status["immutable-plan"] =
        accepted === readFileSync(planPath, "utf8") ? "pass" : "fail";
      await run("format-implementation", ["cargo", "fmt", "--all"]);
      await checks();
      if (
        Object.entries(status).some(
          ([name, result]) =>
            /^(rust-types|ts-types|format|provider-tests|generator-tests|build|clippy)$/.test(
              name
            ) && result !== "pass"
        )
      ) {
        await phase("repair-validation", 1800000, 60);
        await run("format-repair", ["cargo", "fmt", "--all"]);
        await checks("repaired-");
      }
      // The original accepted plan remains the source of capture requirements.
      status.capture_cases = captureCases(plan);
      status.live_capture = status.capture_cases.length
        ? "deferred"
        : "not_required";
      writeFileSync("/artifacts/plan.json", accepted);
      await phase("verify", 1200000, 40);
      try {
        const report = JSON.parse(readFileSync(verificationPath, "utf8"));
        status.verification = validateVerification(report, provider).length
          ? "fail"
          : report.verdict;
        writeFileSync("/artifacts/verification.json", JSON.stringify(report));
      } catch {
        status.verification = "fail";
      }
      await run("remove-plan", ["rm", "-f", "plan.md"]);
    }
    await run("stage-proposal", ["git", "add", "-A"]);
    const patch = await run("proposal-patch", [
      "git",
      "diff",
      "--cached",
      "--binary",
      "--full-index",
      "--no-ext-diff",
      "--no-textconv",
      "HEAD",
    ]);
    writeFileSync("/artifacts/proposal.patch", patch.text);
  } else if (Object.values(status).includes("fail")) {
    throw new Error("Generation failed without producing a proposal.");
  }
}
saveStatus();
writeFileSync(
  "/artifacts/report.json",
  JSON.stringify({ steps: status, diagnostics }, null, 2)
);
