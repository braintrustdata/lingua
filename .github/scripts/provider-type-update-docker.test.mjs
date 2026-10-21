import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  containerArguments,
  readArtifact,
} from "./provider-type-update-isolation.mjs";
import { createInferenceProxy } from "./provider-type-update-proxy.mjs";

const enabled = process.env.PROVIDER_UPDATE_DOCKER_TESTS === "1";
const docker = (args, input) =>
  execFileSync("docker", args, {
    encoding: "utf8",
    input,
    timeout: 60000,
    maxBuffer: 32 * 1024 * 1024,
    stdio: ["pipe", "pipe", "pipe"],
  });
function container(t) {
  const name = `lingua-updater-test-${process.pid}-${Math.random().toString(16).slice(2)}`;
  docker(containerArguments(name));
  t.after(() => {
    docker(["rm", "--force", name]);
    for (const suffix of ["workspace", "artifacts"])
      docker(["volume", "rm", `${name}-${suffix}`]);
  });
  docker(["start", name]);
  return name;
}

test(
  "artifact export rejects symlinks, special files, and oversized records",
  { skip: !enabled },
  (t) => {
    const name = container(t);
    docker([
      "exec",
      name,
      "node",
      "-e",
      `const f=require('fs');f.writeFileSync('/artifacts/report.json','{"ok":true}');f.symlinkSync('/etc/passwd','/artifacts/link.json');f.writeFileSync('/artifacts/large.json','');f.truncateSync('/artifacts/large.json',5*1024*1024)`,
    ]);
    docker(["exec", name, "mkfifo", "/artifacts/fifo.json"]);
    assert.deepEqual(JSON.parse(readArtifact(name, "report.json")), {
      ok: true,
    });
    assert.equal(readArtifact(name, "missing.json", true), null);
    for (const file of ["link.json", "fifo.json", "large.json"]) {
      assert.throws(() => readArtifact(name, file));
    }
  }
);

// Requires a previously built image. There are no real keys, provider requests,
// or LLM charges: the real pinned CLI receives a synthetic inference response.
test(
  "real CLI uses the isolated filesystem and credential-hiding proxy",
  { skip: !enabled },
  async (t) => {
    const name = container(t);
    const state = JSON.parse(
      docker([
        "exec",
        name,
        "node",
        "-e",
        `console.log(JSON.stringify({uid:process.getuid(),keys:Object.keys(process.env).filter(k=>/TOKEN|API_KEY|SECRET/.test(k)),socket:require('fs').existsSync('/var/run/docker.sock')}))`,
      ])
    );
    assert.deepEqual(state, { uid: 1000, keys: [], socket: false });
    docker([
      "exec",
      name,
      "node",
      "-e",
      `const f=require('fs');f.writeFileSync('/workspace/smoke','ok');f.writeFileSync('/tmp/smoke','ok');let denied=false;try{f.writeFileSync('/opt/updater/smoke','bad')}catch{denied=true}if(!denied)process.exit(1)`,
    ]);
    for (const argv of [
      ["cargo", "--version"],
      ["pnpm", "--version"],
      ["claude", "--version"],
    ])
      assert.ok(docker(["exec", name, ...argv]).trim());
    let calls = 0;
    const proxy = await createInferenceProxy({
      key: "host-only-fixture-key",
      fetchImpl: async (url, options) => {
        calls++;
        assert.equal(
          options.headers.authorization,
          "Bearer host-only-fixture-key"
        );
        assert.ok(!options.body.includes("host-only-fixture-key"));
        if (url.includes("count_tokens"))
          return new Response('{"input_tokens":1}', {
            headers: { "content-type": "application/json" },
          });
        const message = {
          id: "msg_smoke",
          type: "message",
          role: "assistant",
          model: "claude-opus-5-5",
          content: [],
          stop_reason: null,
          stop_sequence: null,
          usage: { input_tokens: 1, output_tokens: 0 },
        };
        const events = [
          ["message_start", { type: "message_start", message }],
          [
            "content_block_start",
            {
              type: "content_block_start",
              index: 0,
              content_block: { type: "text", text: "" },
            },
          ],
          [
            "content_block_delta",
            {
              type: "content_block_delta",
              index: 0,
              delta: { type: "text_delta", text: "SMOKE_OK" },
            },
          ],
          ["content_block_stop", { type: "content_block_stop", index: 0 }],
          [
            "message_delta",
            {
              type: "message_delta",
              delta: { stop_reason: "end_turn", stop_sequence: null },
              usage: { output_tokens: 1 },
            },
          ],
          ["message_stop", { type: "message_stop" }],
        ];
        return new Response(
          events
            .map(
              ([event, data]) =>
                `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`
            )
            .join(""),
          { headers: { "content-type": "text/event-stream" } }
        );
      },
    });
    t.after(() => proxy.close());
    const child = spawn(
      "docker",
      [
        "exec",
        "--env",
        `ANTHROPIC_BASE_URL=http://host.docker.internal:${proxy.port}`,
        "--env",
        `ANTHROPIC_API_KEY=${proxy.token}`,
        "--env",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1",
        name,
        "claude",
        "-p",
        "Reply SMOKE_OK.",
        "--model",
        "claude-opus-5-5",
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        '{"mcpServers":{}}',
        "--max-turns",
        "1",
        "--permission-mode",
        "bypassPermissions",
      ],
      { stdio: ["ignore", "pipe", "pipe"] }
    );
    let output = "";
    for (const stream of [child.stdout, child.stderr])
      stream.on("data", (chunk) => (output += chunk));
    const timer = setTimeout(() => child.kill("SIGKILL"), 45000);
    const code = await new Promise((resolve) => child.once("close", resolve));
    clearTimeout(timer);
    assert.equal(code, 0, output);
    assert.match(output, /SMOKE_OK/);
    assert.ok(calls > 0);
  }
);

test(
  "immutable worker produces a proposal and revalidates identical bytes in a fresh container",
  { skip: !enabled },
  (t) => {
    const setup = (name) => {
      const files = {};
      for (const suffix of ["check", "plan", "security"]) {
        const file = `provider-type-update-${suffix}.mjs`;
        files[`.github/scripts/${file}`] = readFileSync(
          new URL(file, import.meta.url),
          "utf8"
        );
      }
      files["crates/lingua/src/providers/google/adapter.rs"] = "// baseline\n";
      files["pipelines/generate-provider-types.sh"] =
        '#!/usr/bin/env node\nrequire("fs").writeFileSync("crates/lingua/src/providers/google/adapter.rs", "// updated documentation\\n");\n';
      for (const command of ["cargo", "make", "pnpm"]) {
        files[`bin/${command}`] =
          '#!/usr/bin/env node\nconsole.log("test fixture_test ... ok");\n';
      }
      const plan = {
        schema_version: 1,
        provider: "google",
        spec_review: {
          complete: true,
          evidence: ["adapter.rs: documentation only"],
        },
        audit_reports: Object.fromEntries(
          ["spec", "capabilities", "semantics", "coverage"].map((name) => [
            name,
            "Fixture evidence",
          ])
        ),
        changes: [],
        blockers: [],
        non_semantic_changes: ["adapter.rs: documentation only"],
      };
      const report = {
        schema_version: 1,
        provider: "google",
        verdict: "pass",
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
            { status: "pass", evidence: "Fixture evidence" },
          ])
        ),
        findings: [],
      };
      files["bin/claude"] =
        `#!/usr/bin/env node\nconst fs=require('fs');const prompt=fs.readFileSync(0,'utf8');if(prompt.includes('planning-only'))fs.writeFileSync(process.env.PROVIDER_UPDATE_PLAN_PATH,${JSON.stringify(JSON.stringify(plan))});if(prompt.includes('read-only verification'))fs.writeFileSync(process.env.PROVIDER_UPDATE_VERIFICATION_PATH,${JSON.stringify(JSON.stringify(report))});\n`;
      docker([
        "exec",
        name,
        "node",
        "-e",
        `const fs=require('fs'),path=require('path');for(const [name,content]of Object.entries(${JSON.stringify(files)})){fs.mkdirSync(path.dirname(name),{recursive:true});fs.writeFileSync(name,content);if(name.startsWith('bin/')||name.startsWith('pipelines/'))fs.chmodSync(name,0o755)}`,
      ]);
    };
    const execute = (name, mode) =>
      docker([
        "exec",
        "--env",
        "PATH=/workspace/bin:/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin",
        name,
        "node",
        "/opt/updater/provider-type-update-worker.mjs",
        mode,
        "google",
      ]);
    const first = container(t);
    setup(first);
    execute(first, "propose");
    const patch = docker(["exec", first, "cat", "/artifacts/proposal.patch"]);
    const plan = docker(["exec", first, "cat", "/artifacts/plan.json"]);
    const report = JSON.parse(
      docker(["exec", first, "cat", "/artifacts/report.json"])
    );
    assert.match(patch, /updated documentation/);
    assert.equal(report.steps.plan, "pass");
    assert.equal(report.steps.verification, "pass");
    assert.equal(report.steps.live_capture, "not_required");
    const second = container(t);
    setup(second);
    docker(["exec", "-i", second, "tee", "/artifacts/proposal.patch"], patch);
    docker(["exec", "-i", second, "tee", "/artifacts/plan.json"], plan);
    execute(second, "validate");
    const validation = JSON.parse(
      docker(["exec", second, "cat", "/artifacts/report.json"])
    );
    assert.ok(
      Object.values(validation.steps).every((value) => value === "pass"),
      JSON.stringify(validation)
    );
  }
);
