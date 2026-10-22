This is the implementation phase for provider `{{provider}}`.
Invoke the `provider-type-update` skill and follow its implementation
phase. Start with the compact implementation checklist in `plan.md`.
The full validated audit is at
`/tmp/provider-type-update/provider-type-update-plan.json`; consult only
the relevant item if the checklist leaves a specific question.
Do not re-audit the specification, inventory unrelated generator
risks, or read entire modules before identifying the next edit.

Use the validation helper after each item:
`node .github/scripts/provider-type-update-check.mjs item <change-id>`.
It runs the plan's focused Rust tests, rejects zero-test matches,
and returns test names, diagnostics, and saved log paths.
Keep scratch files under `/tmp/provider-type-update/provider-type-update-scratch`.
If a required operation is unavailable, record the failure, affected
change ids, and unverified work in
`/tmp/provider-type-update/provider-type-update-implementation-notes.md`,
then continue independent items.

Implement every unblocked plan item, including focused tests and any
payload cases required by the plan. Provider-only items use focused
unit tests and have no payload cases. Treat every change id listed in
the checklist's blocked changes section as out of scope: do not
implement it, add a fixture for it, or invent a workaround.
Do not edit `plan.md`, `AGENTS.md`, or any
`generated.rs` file directly,
`.github/**`, `pipelines/**`, `scripts/**`, or `Makefile`, and do not
call a live provider API. Never add a `provider_options` marker or
another opaque field solely to replay a provider semantic. If the
universal model cannot represent a field non-lossily, stop without
implementing that plan item. Resolve generated public-name collisions
in `crates/generate-types`; do not accept internal prefixes such as
`V1Main...` in exported Rust or TypeScript names.
Public-name normalization and collision repairs are deterministic
generator work and must be completed through their separate unblocked
plan items without implementing any blocked semantic mapping.

Work in checklist order. For each item, add its named tests and
payload cases, implement the behavior, and use the helper to check
it before moving on. Broad validation runs separately in CI.

For every `provider_only` item:
- preserve complete native wire acceptance in generated types and validation;
- add focused tests for byte-preserving same-format passthrough;
- reject cross-provider transformation with an explicit unsupported error;
- add the planned offline end-to-end test through the public transform entry point,
  asserting a stable error category and semantic context rather than exact error prose.
Do not add universal fields, cross-provider mappings, expected-difference entries,
payload cases, or live captures for `provider_only` items.

If you change `crates/generate-types`, run
`make generate-provider-types PROVIDER={{provider}}` so the
generated Rust reflects the generator fix. Changes to `generated.rs`
are allowed only when produced by that command. Before finishing,
inspect `git diff` against the checklist and check remaining items
with the helper (`generator-tests` runs the generator suite).
Do not report completion while a permitted deterministic validation
is failing. Leave the completed
changes uncommitted. Keep the final response to one sentence and do
not include diffs, logs, or test output.
