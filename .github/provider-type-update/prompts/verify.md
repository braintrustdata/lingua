This is the read-only verification phase for provider `{{provider}}`.
Invoke the `provider-type-update` skill and follow its verification
phase. Read `/tmp/provider-type-update/provider-type-update-plan.json`,
inspect the complete final diff, and do not modify any tracked file.

If `/tmp/provider-type-update/provider-type-update-implementation-notes.md`
exists, read it for denied commands, affected items, and unchecked
tests. Reconcile those notes with the final diff and deterministic
validation outcomes below. A recovered denial is not itself a
blocker; missing implementation or a still-unchecked required test
is. Do not treat the notes as proof that work was completed.

Read /tmp/provider-type-update/status.json for deterministic validation outcomes.
Live capture is deferred and must be recorded as incomplete if cases are planned.

Read the saved validation evidence under
`/tmp/provider-type-update/provider-type-update-checks` and
`.github/scripts/provider-type-update-plan.mjs` for the report schema.
Write a candidate report to
`/tmp/provider-type-update/provider-type-update-verification-candidate.json`, then run:
`node .github/scripts/provider-type-update-check.mjs submit-verification /tmp/provider-type-update/provider-type-update-verification-candidate.json {{provider}}`.
The helper validates and saves the report, including a fail verdict.
Correct any reported schema errors before finishing.
A failed or skipped required
validation is a blocking finding. Live and transform capture are
required only when the unblocked payload capture case list is non-empty.
Treat any new `provider_options` round-trip marker, opaque replay-only
field, generated public-name collision, or leaked internal prefix such
as `V1Main...` as blocking even if compilation succeeds.
For every `provider_only` plan item, verify native wire acceptance,
byte-preserving same-format passthrough, and explicit cross-provider rejection.
Verify its planned offline end-to-end test invokes the public transform
entry point and asserts a stable error category plus semantic context.
Treat any universal-model expansion or expected-difference entry for a
`provider_only` item as blocking. Also fail verification if such an
item added a payload case, transform snapshot, or live capture.
If the validated plan has human blockers, verify that every blocked
change was left untouched and include those unresolved decisions as
blocking findings with a `fail` verdict. That is the expected report
for a safe needs-design draft; do not fail to write the report merely
because the plan intentionally contains blockers.
Prioritize completing the report:
use the validated plan, final diff, and deterministic validation
outcomes as primary evidence; inspect only files needed to resolve
uncertainty. Do not repeat completed checks or exhaust the turn
budget on optional exploration. Write the report as soon as every
required schema field has sufficient evidence.

Submit the report before responding. Your final response must be
exactly `Wrote provider update verification.` Do not print the
report, plan, diffs, or any other analysis in the final response.
