import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { checkEnvironment } from "./provider-type-update-security.mjs";

export function allowedPath(path, provider) {
  if (!new Set(["openai", "anthropic", "google"]).has(provider)) return false;
  if (
    path.includes("\\") ||
    path
      .split("/")
      .some(
        (part) =>
          !part ||
          part === "." ||
          part === ".." ||
          part === "AGENTS.md" ||
          part.startsWith(".")
      )
  ) {
    return path === "specs/anthropic/.stats.yml" && provider === "anthropic";
  }
  if (/[^A-Za-z0-9_./-]/.test(path)) return false;
  return (
    /^crates\/lingua\/(src|tests)\/.+\.(rs|json|snap)$/.test(path) ||
    /^crates\/generate-types\/src\/.+\.rs$/.test(path) ||
    /^crates\/coverage-report\/src\/(requests|responses|streaming)_expected_differences\.json$/.test(
      path
    ) ||
    /^bindings\/typescript\/src\/generated\/.+\.ts$/.test(path) ||
    /^payloads\/cases\/[A-Za-z0-9_-]+\.ts$/.test(path) ||
    /^payloads\/(snapshots|responses|streams|transforms)\/.+\.(json|snap)$/.test(
      path
    ) ||
    new RegExp(`^specs/${provider}/.+\\.(json|yml|yaml|proto)$`).test(path)
  );
}

export function patchDigest(patch) {
  return createHash("sha256").update(patch).digest("hex");
}

// Run only from an immutable, credential-free checkout on a fresh runner.
// Inspect the staged index before allowing any proposed file into the worktree.
// Publication never builds, installs, imports, or executes candidate files.
export function applyProposal(root, path, provider) {
  const patch = readFileSync(path);
  if (!patch.length || patch.length > 64 * 1024 * 1024)
    throw new Error("Invalid proposal size.");
  const git = (args) =>
    execFileSync(
      "git",
      [
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.attributesFile=/dev/null",
        ...args,
      ],
      {
        cwd: root,
        env: checkEnvironment(),
        encoding: "utf8",
        maxBuffer: 128 * 1024 * 1024,
        stdio: ["pipe", "pipe", "pipe"],
      }
    );
  if (git(["status", "--porcelain"]).trim())
    throw new Error("Proposal requires a clean checkout.");
  git(["apply", "--cached", "--whitespace=nowarn", "--", path]);
  try {
    const paths = git([
      "diff",
      "--cached",
      "--name-only",
      "--no-renames",
      "-z",
      "HEAD",
    ])
      .split("\0")
      .filter(Boolean);
    if (!paths.length || paths.some((name) => !allowedPath(name, provider))) {
      throw new Error("Proposal changes a forbidden path.");
    }
    const modes = git([
      "diff",
      "--cached",
      "--raw",
      "--no-renames",
      "-z",
      "HEAD",
    ]).split("\0");
    for (let i = 0; i < modes.length - 1; i += 2) {
      const [oldMode, newMode] = modes[i].slice(1).split(" ");
      if (
        ![oldMode, newMode].every((mode) => ["000000", "100644"].includes(mode))
      ) {
        throw new Error(
          "Proposal must contain only regular, non-executable files."
        );
      }
    }
    git(["reset", "--mixed", "--quiet", "HEAD"]);
    git(["apply", "--index", "--whitespace=nowarn", "--", path]);
    return { files: paths.length, sha256: patchDigest(patch) };
  } catch (error) {
    git(["reset", "--mixed", "--quiet", "HEAD"]);
    throw error;
  }
}
