import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { applyProposal, patchDigest } from "./provider-type-update-patch.mjs";
import {
  captureCases,
  renderBlockers,
  renderProviderOnly,
  validatePlan,
} from "./provider-type-update-plan.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
try {
  const [provider, proposal, validation, ...extra] = process.argv.slice(2);
  if (
    extra.length ||
    !["openai", "anthropic", "google"].includes(provider) ||
    !proposal ||
    !validation
  )
    throw new Error("Invalid publication arguments.");
  const metadata = JSON.parse(
    readFileSync(join(proposal, "metadata.json"), "utf8")
  );
  const verified = JSON.parse(
    readFileSync(join(validation, "validation.json"), "utf8")
  );
  const patch = readFileSync(join(proposal, "proposal.patch"));
  const revision = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  }).trim();
  if (
    metadata.revision !== revision ||
    metadata.provider !== provider ||
    metadata.sha256 !== patchDigest(patch) ||
    verified.sha256 !== metadata.sha256 ||
    typeof verified.passed !== "boolean"
  )
    throw new Error("Artifact identity mismatch.");
  const result = applyProposal(
    root,
    join(proposal, "proposal.patch"),
    provider
  );
  const lines = [
    `Automated update of ${provider} provider types.`,
    "",
    "This draft requires human review. The agent ran in an isolated container; independent validation ran on a fresh runner without application secrets. Publication did not execute the proposed code.",
    "",
    `- Independent offline validation: ${verified.passed ? "passed" : "incomplete or failed; inspect the sanitized validation artifact"}`,
    `- Patch: ${result.files} files; SHA-256 \`${result.sha256}\``,
    "- Live capture: deferred. Generated payload code has no provider credentials.",
    "",
  ];
  if (existsSync(join(proposal, "plan.json"))) {
    const plan = JSON.parse(readFileSync(join(proposal, "plan.json"), "utf8"));
    if (validatePlan(plan, provider).length === 0) {
      const cases = captureCases(plan);
      if (cases.length)
        lines.push(
          `Required capture cases: ${cases.map((name) => `\`${name}\``).join(", ")}. Complete these in a reviewed capture environment before marking the draft ready.`,
          ""
        );
      lines.push(renderBlockers(plan), "", renderProviderOnly(plan));
    } else
      lines.push(
        "The plan did not validate; reconstruct and review it before continuing."
      );
  } else
    lines.push(
      "The agent did not produce a plan; review the generated changes before continuing."
    );
  writeFileSync(
    join(process.env.RUNNER_TEMP, "provider-type-update-pr-body.md"),
    lines.join("\n")
  );
  writeFileSync(process.env.GITHUB_OUTPUT, "has_changes=true\n", { flag: "a" });
  console.log(
    "Artifact identity and publication path policy passed. A continuation draft can be created."
  );
} catch {
  console.error("Provider update publication rejected the proposal artifacts.");
  process.exitCode = 1;
}
