import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import {
  containerArguments,
  validationPassed,
} from "./provider-type-update-isolation.mjs";
import {
  allowedPath,
  applyProposal,
  patchDigest,
} from "./provider-type-update-patch.mjs";
import { createInferenceProxy } from "./provider-type-update-proxy.mjs";
import {
  checkEnvironment,
  credentialRedactor,
  redactor,
} from "./provider-type-update-security.mjs";

test("check environments retain toolchain locations and exclude credentials and code injection settings", () => {
  const environment = checkEnvironment({
    PATH: "/bin",
    HOME: "/home/node",
    CARGO_HOME: "/home/node/.cargo",
    BRAINTRUST_API_KEY: "key",
    AWS_ACCESS_KEY_ID: "key",
    AWS_SECRET_ACCESS_KEY: "secret",
    GITHUB_TOKEN: "token",
    ACTIONS_RUNTIME_TOKEN: "token",
    SSH_AUTH_SOCK: "/socket",
    NODE_OPTIONS: "--require /evil.cjs",
    PYTHONPATH: "/evil",
    LD_PRELOAD: "/evil.so",
    GIT_CONFIG_COUNT: "1",
    BASH_ENV: "/evil",
    ANTHROPIC_BASE_URL: "http://proxy",
  });
  assert.deepEqual(environment, {
    CARGO_TERM_COLOR: "never",
    PATH: "/bin",
    HOME: "/home/node",
    CARGO_HOME: "/home/node/.cargo",
  });
});

test("diagnostic redaction covers raw, URL-encoded, base64, and recognizable credentials", () => {
  const secret = "fixture secret+/=";
  const redact = redactor({ BRAINTRUST_API_KEY: secret });
  for (const value of [
    secret,
    encodeURIComponent(secret),
    Buffer.from(secret).toString("base64"),
    "ghp_12345678901234567890",
    "Bearer fixture-token",
    "api_key=fixture-value",
  ]) {
    assert.ok(!redact(value).includes(value));
    assert.match(redact(value), /REDACTED/);
  }
  assert.equal(redact("test example ... ok"), "test example ... ok");
});

test("patch credential checks preserve authentication code and placeholder documentation", () => {
  const redact = credentialRedactor({
    BRAINTRUST_API_KEY: "actual-fixture-credential",
  });
  const code = "api_key=example; Authorization: Bearer placeholder-token";
  assert.equal(redact(code), code);
  assert.equal(redact("actual-fixture-credential"), "[REDACTED]");
});

test("empty, missing, failed, and incomplete validation records cannot pass", () => {
  for (const report of [
    {},
    { steps: {} },
    { steps: { build: "pass" } },
    { steps: { build: "fail" } },
  ]) {
    assert.equal(validationPassed(report), false);
  }
});

test("containers cannot access host mounts, host namespaces, credentials, or the Docker socket", () => {
  const args = containerArguments("fixture-container");
  assert.ok(args.includes("--read-only"));
  assert.equal(args[args.indexOf("--user") + 1], "1000:1000");
  assert.equal(args[args.indexOf("--cap-drop") + 1], "ALL");
  assert.ok(args.includes("no-new-privileges"));
  assert.equal(args[args.indexOf("--network") + 1], "bridge");
  assert.equal(args.filter((arg) => arg.startsWith("type=volume,")).length, 2);
  assert.ok(
    !args.some(
      (arg) =>
        /type=bind|docker.sock|--privileged|--pid|--env-file/.test(arg) &&
        arg !== "--pids-limit"
    )
  );
  assert.ok(!args.includes("--env"));
});

test("inference proxy supplies auth only to the fixed upstream and preserves streaming", async (t) => {
  let forwarded;
  const proxy = await createInferenceProxy({
    key: "upstream-fixture-secret",
    host: "127.0.0.1",
    fetchImpl: async (url, options) => {
      forwarded = { url, options };
      return new Response('event: message_start\ndata: {"ok":true}\n\n', {
        headers: {
          "content-type": "text/event-stream",
          "x-secret": "upstream-fixture-secret",
        },
      });
    },
  });
  t.after(() => proxy.close());
  const result = await fetch(
    `http://127.0.0.1:${proxy.port}/v1/messages?beta=true`,
    {
      method: "POST",
      headers: {
        "x-api-key": proxy.token,
        authorization: "attacker",
        "x-bt-project-name": "attacker",
        "anthropic-beta": "fixture-2026-01-01",
        "x-target-url": "http://attacker.invalid",
      },
      body: '{"model":"claude-opus-5-5"}',
    }
  );
  assert.equal(result.status, 200);
  assert.match(await result.text(), /message_start/);
  assert.equal(result.headers.get("x-secret"), null);
  assert.equal(
    forwarded.url,
    "https://gateway.staging.braintrust.dev/v1/messages?beta=true"
  );
  assert.equal(forwarded.options.redirect, "manual");
  assert.equal(
    forwarded.options.headers.authorization,
    "Bearer upstream-fixture-secret"
  );
  assert.equal(
    forwarded.options.headers["x-bt-project-name"],
    "automations-spend-control"
  );
  assert.equal(forwarded.options.headers["x-target-url"], undefined);
  assert.equal(forwarded.options.headers["x-api-key"], undefined);
  assert.ok(!JSON.stringify(forwarded).includes(proxy.token));
});

test("inference proxy rejects arbitrary paths, methods, queries, and unauthenticated requests", async (t) => {
  let calls = 0;
  const proxy = await createInferenceProxy({
    key: "fixture-secret",
    host: "127.0.0.1",
    fetchImpl: async () => {
      calls++;
      throw new Error("must not forward");
    },
  });
  t.after(() => proxy.close());
  const url = `http://127.0.0.1:${proxy.port}`;
  for (const path of [
    "/api-key",
    "/v1/messages/extra",
    "/v1/messages?target=https://attacker.invalid",
    "/v1/messages?beta=false",
    "/v1/messages?beta=true&x=1",
  ]) {
    const result = await fetch(url + path, {
      method: "POST",
      headers: { "x-api-key": proxy.token },
      body: "{}",
    });
    assert.equal(result.status, 403);
    await result.text();
  }
  assert.equal(
    (
      await fetch(url + "/v1/messages", {
        headers: { "x-api-key": proxy.token },
      })
    ).status,
    403
  );
  assert.equal(
    (await fetch(url + "/v1/messages", { method: "POST", body: "{}" })).status,
    401
  );
  assert.equal(calls, 0);
});

test("inference proxy hides upstream auth-bearing errors and refuses redirects", async (t) => {
  for (const status of [302, 400, 401, 429, 500]) {
    const proxy = await createInferenceProxy({
      key: "upstream-fixture-secret",
      host: "127.0.0.1",
      fetchImpl: async () =>
        new Response("upstream-fixture-secret", {
          status,
          headers: {
            location: "https://attacker.invalid",
            "x-key": "upstream-fixture-secret",
          },
        }),
    });
    t.after(() => proxy.close());
    const result = await fetch(`http://127.0.0.1:${proxy.port}/v1/messages`, {
      method: "POST",
      headers: { "x-api-key": proxy.token },
      body: "{}",
    });
    assert.equal(result.status, status === 302 ? 502 : status);
    assert.ok(!(await result.text()).includes("upstream-fixture-secret"));
    assert.equal(result.headers.get("location"), null);
    await proxy.close();
  }
});

test("even a successful upstream response cannot echo the credential across stream chunks", async (t) => {
  const secret = "upstream-fixture-sensitive-credential";
  const stream = new ReadableStream({
    start(controller) {
      for (const text of [
        "event: message\ndata: prefix ",
        secret.slice(0, 12),
        secret.slice(12),
        " suffix\n\n",
      ]) {
        controller.enqueue(new TextEncoder().encode(text));
      }
      controller.close();
    },
  });
  const proxy = await createInferenceProxy({
    key: secret,
    host: "127.0.0.1",
    fetchImpl: async () => new Response(stream),
  });
  t.after(() => proxy.close());
  const result = await fetch(`http://127.0.0.1:${proxy.port}/v1/messages`, {
    method: "POST",
    headers: { "x-api-key": proxy.token },
    body: "{}",
  });
  const output = await result.text();
  assert.equal(output, "event: message\ndata: prefix [REDACTED] suffix\n\n");
});

function repository(t) {
  const directory = mkdtempSync(join(tmpdir(), "provider-patch-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const root = join(directory, "repo");
  mkdirSync(root);
  const git = (args) =>
    execFileSync("git", ["-c", "core.hooksPath=/dev/null", ...args], {
      cwd: root,
      encoding: "utf8",
    });
  const file = "crates/lingua/src/providers/google/adapter.rs";
  mkdirSync(dirname(join(root, file)), { recursive: true });
  writeFileSync(join(root, file), "// baseline\n");
  git(["init", "--quiet"]);
  git(["add", "-A"]);
  git([
    "-c",
    "user.name=Fixture",
    "-c",
    "user.email=fixture@localhost",
    "commit",
    "--quiet",
    "-m",
    "baseline",
  ]);
  const patch = (path, content = "// proposal\n", mode = "file") => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    if (mode === "symlink") symlinkSync("/tmp/secret", join(root, path));
    else writeFileSync(join(root, path), content);
    if (mode === "executable") chmodSync(join(root, path), 0o755);
    git(["add", "-A"]);
    const result = git([
      "diff",
      "--cached",
      "--binary",
      "--full-index",
      "HEAD",
    ]);
    const patchPath = join(directory, "proposal.patch");
    writeFileSync(patchPath, result);
    git(["reset", "--hard", "--quiet", "HEAD"]);
    return patchPath;
  };
  return { root, git, patch, file };
}

test("publisher accepts only scoped regular files and binds their exact patch bytes", (t) => {
  const f = repository(t);
  const path = f.patch(f.file);
  const result = applyProposal(f.root, path, "google");
  assert.equal(result.files, 1);
  assert.equal(result.sha256, patchDigest(readFileSync(path)));
  assert.equal(readFileSync(join(f.root, f.file), "utf8"), "// proposal\n");
});

test("publisher refuses infrastructure, traversal, dotfiles, shell names, executable files, and symlinks", (t) => {
  for (const path of [
    ".github/workflows/evil.yml",
    "scripts/evil.sh",
    "crates/lingua/AGENTS.md",
    "crates/lingua/src/.cargo/config.toml",
    "crates/lingua/build.rs",
    "payloads/package.json",
    "specs/openai/../../.github/evil.yml",
    "specs/google/evil.json\n",
    "specs/google/evil$(id).json",
  ]) {
    assert.equal(allowedPath(path, "google"), false, path);
  }
  for (const [path, mode] of [
    [".github/workflows/evil.yml", "file"],
    ["crates/lingua/src/evil.rs", "executable"],
    ["crates/lingua/src/evil.rs", "symlink"],
  ]) {
    const f = repository(t);
    const proposal = f.patch(path, "// no code should execute\n", mode);
    assert.throws(
      () => applyProposal(f.root, proposal, "google"),
      /forbidden path|regular, non-executable/
    );
    assert.equal(f.git(["status", "--porcelain"]).trim(), "");
  }
  assert.equal(allowedPath("specs/anthropic/.stats.yml", "anthropic"), true);
  assert.equal(allowedPath("specs/openai/openapi.yml", "google"), false);
});
