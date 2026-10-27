import { spawn, execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";
import { applyProposal, patchDigest } from "./provider-type-update-patch.mjs";
import { createInferenceProxy } from "./provider-type-update-proxy.mjs";
import {
  checkEnvironment,
  credentialRedactor,
  redactor,
  redactValue,
} from "./provider-type-update-security.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const image = "lingua-provider-updater:local";
const docker = (args, options = {}) =>
  execFileSync("docker", args, {
    cwd: root,
    env: checkEnvironment(),
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["pipe", "pipe", "pipe"],
    ...options,
  });

export function readArtifact(name, file, optional = false) {
  // Read through an immutable interpreter inside the container. Distinguish
  // absence from read/size/parse failures and reject symlinks or devices.
  const script = `
    const fs = require('node:fs');
    const path = process.argv[1];
    let fd;
    try { fd = fs.openSync(path, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (fd === undefined) console.log(JSON.stringify({ missing: true }));
    else {
      try {
        const stat = fs.fstatSync(fd);
        const limit = (path.endsWith('.patch') ? 64 : 4) * 1024 * 1024;
        if (!stat.isFile() || stat.size > limit) process.exit(1);
        const buffer = Buffer.alloc(limit + 1);
        let length = 0;
        while (length <= limit) {
          const count = fs.readSync(fd, buffer, length, limit + 1 - length, null);
          if (!count) break;
          length += count;
        }
        if (length > limit) process.exit(1);
        console.log(JSON.stringify({ content: buffer.subarray(0, length).toString('utf8') }));
      } finally { fs.closeSync(fd); }
    }
  `;
  const record = JSON.parse(
    docker(["exec", name, "node", "-e", script, `/artifacts/${file}`])
  );
  if (record.missing && optional) return null;
  if (typeof record.content !== "string")
    throw new Error("Missing required updater artifact.");
  return record.content;
}

export function validationPassed(report) {
  const required = [
    "git-init",
    "git-add",
    "git-baseline",
    "dependencies",
    "rust-types",
    "ts-types",
    "format",
    "semantic-policy",
    "provider-tests",
    "generator-tests",
    "build",
    "clippy",
    "payload-sync",
    "payload-tests",
    "typed-boundaries",
    "cross-provider",
    "stage-validated",
    "validated-patch",
    "unchanged-patch",
  ];
  return (
    required.every((name) => report.steps?.[name] === "pass") &&
    Object.values(report.steps).every((value) => value === "pass")
  );
}

export function containerArguments(name) {
  return [
    "create",
    "--name",
    name,
    "--read-only",
    "--user",
    "1000:1000",
    "--cap-drop",
    "ALL",
    "--security-opt",
    "no-new-privileges",
    "--pids-limit",
    "512",
    "--memory",
    "12g",
    "--cpus",
    "3",
    "--network",
    "bridge",
    "--add-host",
    "host.docker.internal:host-gateway",
    "--mount",
    `type=volume,source=${name}-workspace,target=/workspace`,
    "--mount",
    `type=volume,source=${name}-artifacts,target=/artifacts`,
    "--tmpfs",
    "/tmp:rw,nosuid,nodev,uid=1000,gid=1000,mode=1777,size=4g",
    "--tmpfs",
    "/home/node:rw,nosuid,nodev,uid=1000,gid=1000,mode=700,size=4g",
    image,
  ];
}

async function build() {
  const context = mkdtempSync(join(tmpdir(), "provider-updater-image-"));
  try {
    mkdirSync(join(context, "scripts"));
    for (const suffix of [
      "worker",
      "patch",
      "security",
      "plan",
      "check",
      "semantic-policy",
    ]) {
      const name = `provider-type-update-${suffix}.mjs`;
      copyFileSync(
        join(root, ".github/scripts", name),
        join(context, "scripts", name)
      );
    }
    for (const name of ["Dockerfile"]) {
      copyFileSync(
        join(root, ".github/provider-type-update", name),
        join(context, name)
      );
    }
    mkdirSync(join(context, "prompts"));
    for (const phase of [
      "repair",
      "plan",
      "implement",
      "repair-validation",
      "verify",
      "autofix",
    ]) {
      copyFileSync(
        join(root, ".github/provider-type-update/prompts", `${phase}.md`),
        join(context, "prompts", `${phase}.md`)
      );
    }
    // No checkout, .git directory, credentials, or host environment is sent
    // as Docker build context. Image building happens before secret injection.
    const child = spawn("docker", ["build", "--tag", image, context], {
      env: checkEnvironment(),
      stdio: "inherit",
    });
    child.on("error", () => {});
    if ((await new Promise((resolve) => child.once("close", resolve))) !== 0)
      throw new Error("Updater image build failed.");
  } finally {
    rmSync(context, { recursive: true, force: true });
  }
}

async function archiveInto(name, sourceRoot = root) {
  // Do not expose the host checkout or its .git directory as a writable mount.
  const archive = spawn("git", ["archive", "--format=tar", "HEAD"], {
    cwd: sourceRoot,
    env: checkEnvironment(),
    stdio: ["ignore", "pipe", "pipe"],
  });
  const extract = spawn(
    "docker",
    ["exec", "-i", name, "tar", "-x", "--no-same-owner", "-C", "/workspace"],
    { env: checkEnvironment(), stdio: ["pipe", "ignore", "pipe"] }
  );
  for (const child of [archive, extract]) {
    child.on("error", () => {});
    child.stderr.resume();
  }
  const completed = [archive, extract].map(
    (child) => new Promise((resolve) => child.once("close", resolve))
  );
  await pipeline(archive.stdout, extract.stdin);
  if ((await Promise.all(completed)).some((code) => code !== 0))
    throw new Error("Unable to copy trusted source archive.");
}

async function isolate(mode, provider, output, proposal, source) {
  if (!["openai", "anthropic", "google"].includes(provider) || !output)
    throw new Error("Invalid provider or output directory.");
  const sourceRoot = mode === "autofix" && source ? resolve(source) : root;
  if (mode === "validate") {
    if (!proposal) throw new Error("Validation requires a proposal directory.");
    const metadata = JSON.parse(
      readFileSync(join(proposal, "metadata.json"), "utf8")
    );
    const revision = execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim();
    if (metadata.revision !== revision || metadata.provider !== provider)
      throw new Error("Proposal base/provider mismatch.");
    const applied = applyProposal(
      root,
      join(proposal, "proposal.patch"),
      provider
    );
    if (applied.sha256 !== metadata.sha256)
      throw new Error("Proposal digest mismatch.");
  }
  const name = `lingua-updater-${randomBytes(12).toString("hex")}`;
  const redact = redactor();
  const redactPatch = credentialRedactor();
  let proxy;
  const cleanup = () => {
    try {
      docker(["rm", "--force", name]);
    } catch {
      console.error(
        "Unable to remove updater container; the disposable runner must clean it up."
      );
      process.exitCode = 1;
    }
    for (const suffix of ["workspace", "artifacts"]) {
      try {
        docker(["volume", "rm", `${name}-${suffix}`]);
      } catch {
        console.error(
          "Unable to remove an updater volume; the disposable runner must clean it up."
        );
        process.exitCode = 1;
      }
    }
  };
  const onSignal = () => {
    cleanup();
    process.exit(1);
  };
  process.once("SIGINT", onSignal);
  process.once("SIGTERM", onSignal);
  try {
    docker(containerArguments(name));
    docker(["start", name]);
    await archiveInto(name, sourceRoot);
    if (mode === "autofix") {
      if (!proposal || !source)
        throw new Error("Autofix requires review data and a source checkout.");
      docker(["exec", "-i", name, "tee", "/artifacts/review.json"], {
        input: readFileSync(proposal),
      });
    }
    if (mode === "validate") {
      for (const file of ["proposal.patch", "plan.json"]) {
        if (existsSync(join(proposal, file)))
          docker(["exec", "-i", name, "tee", `/artifacts/${file}`], {
            input: readFileSync(join(proposal, file)),
          });
      }
    }
    const args = ["exec"];
    if (mode !== "validate") {
      proxy = await createInferenceProxy({
        key: process.env.BRAINTRUST_API_KEY,
      });
      args.push(
        "--env",
        `ANTHROPIC_BASE_URL=http://host.docker.internal:${proxy.port}`,
        "--env",
        `ANTHROPIC_API_KEY=${proxy.token}`
      );
    }
    args.push(
      name,
      "node",
      "/opt/updater/provider-type-update-worker.mjs",
      mode,
      provider
    );
    // Worker output is untrusted and can contain forged workflow commands.
    // Never stream it into the Actions console or its environment files.
    const child = spawn("docker", args, {
      env: checkEnvironment(),
      stdio: ["ignore", "ignore", "ignore"],
    });
    child.on("error", () => {});
    const code = await new Promise((resolve) => child.once("close", resolve));
    await proxy?.close();
    proxy = undefined;
    if (code !== 0)
      throw new Error("Isolated updater failed; no proposal was published.");
    mkdirSync(output, { recursive: true });
    // Copy only fixed-name, bounded text records, never directories, tarballs,
    // symlinks, raw stdout/stderr, or the agent's home/configuration files.
    const report = JSON.parse(readArtifact(name, "report.json"));
    writeFileSync(
      join(output, "report.json"),
      JSON.stringify(redactValue(report, redact), null, 2)
    );
    if (mode !== "validate") {
      const patch = readArtifact(name, "proposal.patch", true);
      if (patch === null) {
        console.log("Provider specifications are unchanged.");
        return;
      }
      if (redactPatch(patch) !== patch)
        throw new Error(
          "Proposal appears to contain credentials; refusing to export it."
        );
      writeFileSync(join(output, "proposal.patch"), patch);
      const revision = execFileSync("git", ["rev-parse", "HEAD"], {
        cwd: sourceRoot,
        encoding: "utf8",
      }).trim();
      writeFileSync(
        join(output, "metadata.json"),
        JSON.stringify({ provider, revision, sha256: patchDigest(patch) })
      );
      for (const file of ["plan.json", "verification.json"]) {
        const content = readArtifact(name, file, true);
        if (content === null) continue;
        const value = JSON.parse(content);
        writeFileSync(
          join(output, file),
          JSON.stringify(redactValue(value, redact), null, 2)
        );
      }
      if (mode === "autofix")
        applyProposal(sourceRoot, join(output, "proposal.patch"), provider);
      if (process.env.GITHUB_OUTPUT)
        writeFileSync(process.env.GITHUB_OUTPUT, "has_changes=true\n", {
          flag: "a",
        });
    } else {
      const metadata = JSON.parse(
        readFileSync(join(proposal, "metadata.json"), "utf8")
      );
      writeFileSync(
        join(output, "validation.json"),
        JSON.stringify({
          sha256: metadata.sha256,
          passed: validationPassed(report),
        })
      );
    }
    console.log("Isolated updater completed; sanitized records exported.");
  } finally {
    await proxy?.close();
    cleanup();
    process.removeListener("SIGINT", onSignal);
    process.removeListener("SIGTERM", onSignal);
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    const [mode, provider, output, proposal, source] = process.argv.slice(2);
    if (mode === "build") await build();
    else if (["propose", "validate", "autofix"].includes(mode))
      await isolate(mode, provider, output, proposal, source);
    else
      throw new Error(
        "Use build, propose <provider> <output>, validate <provider> <output> <proposal>, or autofix <provider> <output> <review> <source>."
      );
  } catch {
    // Subprocess exceptions include command output. Do not dump them on the
    // host, which owns the reusable inference credential and Actions token.
    console.error(
      "Provider updater failed at the isolation/artifact boundary."
    );
    process.exitCode = 1;
  }
}
