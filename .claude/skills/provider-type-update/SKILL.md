---
name: provider-type-update
description: Audit, plan, implement, and verify updates to OpenAI, Anthropic, or Google generated provider types. Use when a provider specification or generated.rs diff may require capability, adapter, universal type, serializer, streaming, payload-case, or test changes.
---

# Provider type update

Treat a provider specification update as a semantic integration task, not a
generated-code review.

## Provider-only boundary

Classify a change as `provider_only` when its semantics depend on
provider-owned execution or provider-defined harness state rather than a
portable message contract. This includes hosted code execution, browser and
computer control, provider-defined toolsets and their result blocks, hosted
search or retrieval, containers and skills, MCP or connector state,
provider-scoped file handles, and encrypted continuation state.

Generic caller-defined function tools remain candidates for portable mapping.
They are not provider-only merely because a provider calls them `tool_use` or
uses a provider-specific wire envelope.

For a provider-only change, the required contract is:

- generated request, response, and streaming types accept the complete native
  wire shape
- native validation and format detection recognize it
- an unmodified same-format request, response, or stream takes the
  byte-preserving passthrough path
- a cross-provider transform returns an explicit unsupported error before any
  field can be dropped or coerced
- `universal_semantics` is `not_applicable`; do not add a universal field,
  provider-options marker, opaque replay carrier, or cross-provider mapping
- a focused unit test covers native passthrough
- a focused offline end-to-end test invokes the public transform entry point
  and verifies cross-provider rejection by stable error category and semantic
  context rather than exact prose
- no payload case, transform snapshot, expected-difference entry, or live
  capture is added

Provider-only status is a completed scope decision, not a human blocker. Ask
for a canonical representation only when a feature is intended to be portable
and its non-lossy mapping is unclear.

## Planning phase

Do not edit tracked files during this phase.

1. Read `AGENTS.md` and inspect the generated public-type diff.
2. Run `provider-spec-auditor` first to reduce the complete raw specification
   diff into semantic change groups. It must collapse repeated schema churn and
   separate documentation-only or mechanically equivalent changes.
3. If semantic groups remain, give only those groups to
   `provider-capability-auditor`, `provider-semantic-auditor`, and
   `provider-coverage-auditor` in parallel. Do not have each agent independently
   re-read the complete raw specification diff.
4. Reconcile their reports yourself. Subagent reports are evidence, not final
   decisions.
5. Write the structured JSON plan requested by the workflow.
6. Cover every semantic change group. Do not create separate plan items for
   repeated occurrences of the same wire-shape change, or for description,
   example, ordering, and formatting-only churn.

Plan only work required by the current specification update. Generator repairs
need evidence of an actual failure, collision, or public-name regression in
this diff. Do not add speculative future guards, unrelated cleanup, or general
hardening. Keep item instructions concise and put detailed audit evidence in
the audit reports instead of repeating it for every surface.

When no semantic groups remain, write an empty `changes` array, record grouped
evidence in `non_semantic_changes`, and mark the other audit reports as skipped
with the reason.

For every semantic change group, classify these surfaces explicitly:

- generated type effect
- model and capability matching
- provider request import
- provider request export
- provider response import
- provider response export
- streaming
- universal semantics
- cross-provider behavior

Use `not_affected` or `not_applicable` only with concrete evidence. Use
`provider_only` for functionality inside the boundary above. Mark an item
`blocked` only when portable mapping is intended but a non-lossy universal
representation is unclear.

## Implementation phase

Start with `plan.md`; the workflow stages a compact implementation checklist
there. Consult the full validated JSON audit at `$PROVIDER_UPDATE_PLAN_PATH`,
when supplied, only to resolve a specific question about the current item. Do
not repeat the specification audit or read whole modules before identifying
the next edit. Implement items in checklist order, completing each behavior
and its tests, or recording an unresolved tool blocker, before starting the
next.

Keep Bash in the repository root. Use Read/Grep with bounded ranges for source
inspection and put scratch scripts in temp (`$PROVIDER_UPDATE_SCRATCH_DIR` in
the workflow), never in the repository.
Leave scratch cleanup to the runner. A command denial does not end the phase.
Allow one evidence-based correction per denied capability to an explicitly
permitted command form or Read/Grep, with at most four recovery attempts across
the phase. Never change permissions or use another interpreter to evade a
denial. Do not retry an unavailable capability on later items. Optional version
checks, inspection helpers, and scratch cleanup must not block useful work.

If recovery fails, immediately record the exact command, denial, affected
change ids, and remaining work or unchecked tests in the implementation notes
path supplied by the workflow. Preserve consistent progress and continue
independent items. If only validation is unavailable, continue scoped code and
test additions, mark the work unverified, and leave execution to the workflow.
Do not claim completion for skipped work or unchecked tests. End early only
when no useful authorized work remains.

1. Add focused unit tests, provider-only offline end-to-end rejection tests,
   and any portable-semantics payload cases before changing adapter behavior.
2. Change generation code or typed adapters. Never edit `generated.rs`
   directly.
3. Preserve typed boundaries and explicit errors.
4. Update every serializer and streaming path identified by the plan.
5. Run the named focused tests with a test-name filter and confirm they actually
   ran and passed, or record the unresolved tool blocker and unverified work,
   before moving to the next item. Do not defer test additions
   until after all production changes. The workflow runs broad validation
   separately; do not run a broad baseline suite or repeat completed checks
   unless needed to diagnose a focused failure.
6. Do not call live provider APIs. The workflow runs planned live captures in
   a separate secret-scoped step.
7. Re-read the plan and inspect `git diff` before finishing. Every affected
   surface must have an implementation or test.

Do not add marker fields, silent coercions, raw JSON semantic inspection, broad
expected-difference exceptions, or fallback behavior.

For `provider_only` items, implement only generated/native wire acceptance,
same-format passthrough coverage, and explicit cross-provider rejection. Test
the rejection offline through the public transform entry point. Do not add
payload cases or live captures for provider-only items. Do not expand the
universal model or teach another provider to emit the feature.

## Verification phase

Do not edit tracked files.

1. Compare the final diff with every plan item.
2. Inspect generated-source provenance, capability predicates, all request and
   response directions, streaming completion behavior, universal semantics,
   and cross-provider effects.
3. Check that focused tests exercise new semantic cases rather than only
   serialization shape.
4. Write the structured verification report requested by the workflow.
5. Fail verification for omissions, unsupported assumptions, or a plan item
   without corresponding implementation and tests.

For every `provider_only` item, verify native wire acceptance, byte-preserving
same-format passthrough, explicit cross-provider rejection through an offline
public-entry end-to-end test, and the absence of universal-model,
expected-difference, payload-case, and live-capture changes.
