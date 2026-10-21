This is the planning-only phase for provider `{{provider}}`.
Invoke the `provider-type-update` skill and follow its planning phase.

First invoke only `@provider-spec-auditor`. Have it reduce the raw
specification and generated-code diff into a compact inventory of
wire-shape and public-type semantic changes. It must group repeated
schema churn and separate documentation, ordering, example, and
formatting-only changes from behavior changes.

If that inventory contains semantic changes, invoke
`@provider-capability-auditor`, `@provider-semantic-auditor`, and
`@provider-coverage-auditor` in parallel with only the relevant
semantic groups. They must not independently re-audit the complete
raw specification diff. If the inventory contains no semantic
changes, skip those three auditors.
All auditor calls must run in the foreground and return their
final reports before you synthesize the plan. They may be issued
together as parallel tool calls; do not launch background agents.
Background tasks are disabled for this unattended phase. Do not
return a waiting/status response: your final response ends the
phase. Collect the reports and write the plan before finishing.

Do not modify any tracked file. Read
`.github/scripts/provider-type-update-plan.mjs` for the schema.
Write a candidate JSON plan to
`/tmp/provider-type-update/provider-type-update-plan-candidate.json`, then run:
`node .github/scripts/provider-type-update-check.mjs submit-plan /tmp/provider-type-update/provider-type-update-plan-candidate.json {{provider}}`.
The helper validates the plan and saves the accepted artifact.
Correct any reported errors before finishing.
Use one plan item per semantic change
group rather than one item per repeated schema occurrence. Exclude
documentation-only and mechanically equivalent changes from the
plan, but record the evidence used to classify them.
When there are no semantic changes, use an empty `changes` array,
put the grouped evidence in `non_semantic_changes`, and summarize
why the other three audits were skipped in their `audit_reports`
entries.
Shared automation infrastructure is outside the scope of an automated
provider update. Do not include `.github/**`, `pipelines/**`,
`scripts/**`, `Makefile`, or `AGENTS.md` in implementation targets.
Provider type generation fixes belong in `crates/generate-types`.
Include only work required by this specification update. Generator
repairs need evidence of an actual failure, collision, or public-name
regression in the current diff. Do not plan speculative guards for
future specifications, unrelated cleanup, or general hardening.
Keep per-item behavior and test instructions concise; put detailed
audit evidence in `audit_reports` instead of repeating it per surface.

Classify provider-hosted execution and provider-defined harness protocols
as `provider_only`. This includes hosted code execution, browser or
computer control, provider-defined toolsets and result blocks, hosted
search or retrieval, containers and skills, MCP or connector state,
provider-scoped file handles, and encrypted continuation state.
Generic caller-defined function tools remain candidates for portable mapping;
provider-specific `tool_use` wire syntax alone does not make them
provider-only.

A `provider_only` item is unblocked. Plan generated/native wire
acceptance, format detection, byte-preserving same-format passthrough,
and an explicit unsupported error for cross-provider transformation.
Set `universal_semantics` to `not_applicable` and `cross_provider` to
`affected`. Use a focused unit test for native passthrough. In
`tests.offline_end_to_end`, name a separate offline test that invokes
the public transform entry point and asserts the stable unsupported
error category plus relevant semantic context. Leave payload cases
and live capture cases empty. Do not target universal types or
expected-difference files.

Treat a semantic as blocked only when portable mapping is intended but
its non-lossy universal representation is unclear. Never plan a
`provider_options` marker or other opaque round-trip-only field. Audit
generated public type names for upstream
naming collisions and internal prefixes; do not accept a prefixed
fallback such as `V1Main...` as a compatibility solution.
Separate deterministic generator and public-name compatibility work
from provider-semantic decisions. A generated-name normalization or
collision repair must be its own unblocked change id even when the
provider field using that type needs a human canonical-representation
decision. Do not place both concerns under one blocker.

For every blocked change, fill the complete blocker schema with the
decision question, concrete evidence, a recommended option,
alternatives and their tradeoffs, likely affected files, and exact
validation commands. The workflow renders this information directly
into a draft PR for the human who finishes the update.
Live capture is deferred to human review because generated code must never
receive provider credentials. Any planned payload cases require capture
before this draft can be made ready for review.
Use `live_capture.cases` only for additional capture-only evidence.

Submit the plan before responding. Your final response must be
exactly `Wrote provider update plan.` Do not print the plan, auditor
reports, diffs, or any other analysis in the final response.
