import { execFileSync } from "node:child_process";
import { checkEnvironment } from "./provider-type-update-security.mjs";

const diff = (paths) =>
  execFileSync(
    "git",
    [
      "diff",
      "HEAD",
      "--no-ext-diff",
      "--no-textconv",
      "--unified=0",
      "--",
      ...paths,
    ],
    {
      cwd: "/workspace",
      env: checkEnvironment(),
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }
  );
try {
  const added = diff(["crates/lingua/src/providers"])
    .split("\n")
    .filter((line) => line.startsWith("+") && !line.startsWith("+++"));
  if (added.some((line) => line.includes("provider_options")))
    throw new Error(
      "Automated provider updates must not add provider_options replay plumbing."
    );
  const google = diff([
    "crates/lingua/src/providers/google/generated.rs",
    "bindings/typescript/src/generated/google",
  ]);
  if (
    google
      .split("\n")
      .some((line) =>
        /^\+[^+].*(pub (struct|enum|type) V1(Main|main)|Option<V1(Main|main)|Vec<V1(Main|main)|from "\.\/V1(Main|main))/.test(
          line
        )
      )
  ) {
    throw new Error(
      "Generated Google public API leaks an internal schema prefix."
    );
  }
  console.log("Provider semantic policy passed.");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
