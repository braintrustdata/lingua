This is a single bounded mechanical repair pass for provider
`{{provider}}`. Read
`/tmp/provider-type-update/provider-type-update-plan.json` and the
current diff. Fix only deterministic validation failures in already
unblocked implementation work. Every change id in the plan's
`blockers` array remains out of scope; do not implement it or invent
a workaround.

Preserve the plan's provider-only boundary during repair. A
`provider_only` item may change generated/native provider code and
focused unit tests only; do not repair it by adding universal fields,
cross-provider mappings, expected differences, payload cases, or live
captures.

Initial validation outcomes:
- Rust provider regeneration: `see /tmp/provider-type-update/status.json`
- path policy: `see /tmp/provider-type-update/status.json`
- semantic policy: `see /tmp/provider-type-update/status.json`
- TypeScript regeneration: `see /tmp/provider-type-update/status.json`
- formatting: `see /tmp/provider-type-update/status.json`
- provider tests: `see /tmp/provider-type-update/status.json`
- generator tests: `see /tmp/provider-type-update/status.json`
- build: `see /tmp/provider-type-update/status.json`
- clippy: `see /tmp/provider-type-update/status.json`

Read the saved result.json files and logs under
`/tmp/provider-type-update/provider-type-update-checks` for exact diagnostics,
then make the smallest durable repair. Use
`node .github/scripts/provider-type-update-check.mjs item <change-id>`
to check repaired items; the helper also accepts `provider-tests {{provider}}`,
`generator-tests`, `build`, and `clippy`. Do not edit generated Rust directly;
repair `crates/generate-types` and run
`make generate-provider-types PROVIDER={{provider}}`.
Do not modify `plan.md`, `.github/**`, `pipelines/**`, `scripts/**`,
`Makefile`, or `AGENTS.md`. Do not call live provider APIs. Finish by
rerunning the focused commands you repaired. Leave changes
uncommitted. Keep the final response to one sentence and do not
include diffs, logs, or test output.
