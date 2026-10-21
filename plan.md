# Provider updater security

## Root cause

Untrusted plan strings enter Bash source. Agent-editable code executes on runners with reusable credentials and a write token, and raw diagnostics inherit those credentials. Command allowlists and post-execution path checks do not isolate code.

## Target files and behavior

- Validate capture identifiers and keep plan values out of shell source.
- Run the agent and all generated code inside a non-root container with no host mounts, Docker socket, repository token, or reusable API credentials. A host-side proxy exposes only the fixed inference endpoint and supplies upstream authentication.
- Validate proposals on a fresh runner without application secrets. Publish only validated-path patches on another fresh runner, executing no proposal code and opening draft PRs for review. Apply the same isolation to review autofix and recheck paths and modes in its publisher.
- Defer live capture; never give agent-editable payload code provider credentials.
- Use a minimal check environment and redact diagnostics before persistence and artifact upload.
- Keep immutable controller code separate from candidate files; do not share writable caches across trust boundaries.

## Validation

Add executable regression tests for malicious identifiers, environment exclusion, redaction, proxy endpoint/header restrictions, patch paths/modes, and workflow job boundaries. Run all updater Node tests, format checks, workflow YAML/actionlint, and an isolated Docker smoke test when available. No provider converter behavior or expected differences change.

## Completed checks

- Helper regressions pass on Node 20 and Node 24.
- Docker image builds; the pinned CLI completes a request through the proxy using dummy credentials and a synthetic inference response.
- Real container tests verify UID/filesystem restrictions, reject unsafe artifact files, and run a proposal through independent validation in a fresh container.
- Workflow linting, formatting, diff checks, and both typed-boundary checks pass.
- CI runs Docker smoke tests when updater code or workflows change. Live provider capture and a complete scheduled run remain deferred.
