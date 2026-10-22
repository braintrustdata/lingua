import { applyProposal } from "./provider-type-update-patch.mjs";

try {
  const [provider, patch, ...extra] = process.argv.slice(2);
  if (extra.length || !patch) throw new Error("Invalid patch arguments.");
  applyProposal(process.cwd(), patch, provider);
  console.log("Publication patch path and file-mode policy passed.");
} catch {
  console.error("Publication rejected the proposed patch.");
  process.exitCode = 1;
}
