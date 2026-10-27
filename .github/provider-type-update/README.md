# Provider updater isolation

The scheduled updater separates proposal, validation, and publication across
fresh GitHub-hosted runners. The Codex autofix proposal uses the same isolation.

- The controller and prompts come from the immutable workflow checkout (or
  trusted `main` for review events). They are copied into a root-owned image.
- The container runs as UID 1000 with a read-only root filesystem, no Linux
  capabilities, no privilege escalation, resource limits, and private process
  and network namespaces. Only disposable Docker volumes and tmpfs are writable.
  There are no host bind mounts, Docker sockets, persisted checkout credentials,
  shared build caches, repository tokens, or reusable API keys in the container.
- Source enters through `git archive`. Candidate code, dependency installation,
  hooks, generators, tools, and tests execute inside the container. Tool
  permissions are an error-prevention measure; they are not the sandbox.
- A host-side proxy accepts authenticated POSTs only to `/v1/messages` and
  `/v1/messages/count_tokens` on the fixed Braintrust gateway. It replaces all
  authentication and project headers, rejects redirects, suppresses upstream
  errors, bounds requests, and closes when the phase ends. The agent has a
  temporary inference capability and can spend inference within that session;
  it cannot retrieve the reusable Braintrust key or use other Braintrust APIs.
- Independent validation has no application secrets or inference capability.
  Validation results identify the exact proposal digest. Publication rechecks
  the base revision, digest, paths, and modes from a fresh trusted checkout.
  Publication does not execute proposed code. Git hooks are disabled.
- Publication opens a draft even when offline checks pass. Human review remains
  required. Live provider captures are deferred: moving editable capture code
  into another job with provider keys would preserve the credential exposure.
  Planned capture case names remain visible in the draft.
- Checks receive a small toolchain environment rather than the parent's full
  environment. Logs are redacted before persistence. The controller exports
  only fixed-name JSON records with bounded diagnostics and the proposal patch;
  it never uploads raw logs, agent homes, settings, or transcripts. Patches that
  appear to contain a host credential are rejected rather than rewritten.

Provider specifications, generated code, agent reports, and artifacts remain
untrusted data. Container isolation relies on Docker and the host kernel; use
disposable hosted runners, not a privileged or shared self-hosted runner.

Run helper regressions with:

```bash
node --test .github/scripts/*test.mjs
```

Build the image locally with Docker:

```bash
node .github/scripts/provider-type-update-isolation.mjs build
PROVIDER_UPDATE_DOCKER_TESTS=1 node --test .github/scripts/provider-type-update-docker.test.mjs
```
