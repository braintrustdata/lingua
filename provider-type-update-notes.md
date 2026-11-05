# Provider type update notes

## BLOCKING: openai specification download endpoint returns HTTP 404

**Status:** blocked on an external service. No repository-side defect found.

### What fails

`pipelines/generate-provider-types.sh openai` aborts in step 1, before type
generation runs:

```
SPEC_URL="https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml"
curl -fL -s "$SPEC_URL" -o "$SPEC_FILE"   # HTTP 404 -> curl exit 22 -> set -e aborts
```

The endpoint responds `HTTP/2 404` with body `{"error":"not found"}` from the
origin (Cloudflare/Google headers present), not from a proxy. The same host
still serves `anthropic` successfully (`HTTP 200`, 4.2 MB), so this is a
provider-specific withdrawal rather than a network or DNS problem.

Upstream cause: OpenAI moved its specification source of truth off Stainless to
`github.com/openai/openai-openapi`, and Stainless stopped serving public
`documented` spec URLs (they are now authenticated build outputs).

### Why the logs are empty

Because `set -e` is active and `curl -f` exits non-zero on 404, the script
aborts on the `curl` line itself. The `if [ -f "$SPEC_FILE" ] && [ -s ... ]`
check and its `❌ Failed to download provider spec` message are unreachable on
this failure path, so both `provider-type-generation.log` and
`provider-type-generation.retry.log` end after `Downloading OpenAI OpenAPI
spec...` with no diagnostic.

The failed download does **not** truncate or corrupt `specs/openai/openapi.yml`
— `curl -f` does not write the output file on an HTTP error.

### Repository state is healthy

Verified against the committed synchronized specification (`specs/openai/openapi.yml`,
`info.version: 2.3.0`):

- `cargo run --bin generate-types -- openai` succeeds and reproduces
  `crates/lingua/src/providers/openai/generated.rs` **byte-identically**
  (`git diff` empty).
- `cargo build` succeeds.
- `cargo test -p lingua` passes (1465 + 18 tests, 0 failures).

No union member, discriminator value, required field, or request/response shape
changed, because the synchronized specification did not change.

### Why no spec source was substituted

Switching the download to
`https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml`
is **not** a mechanical fix and is deliberately not applied here. Comparing
`components.schemas` between the committed Stainless spec and that GitHub spec:

| | committed (Stainless) | GitHub `openai-openapi` |
|---|---|---|
| schemas | 1394 | 2162 |
| removed | — | 1 (`BetaServiceTier`) |
| added | — | 769 (entire Agents/Automations surface, etc.) |

Both declare `info.version: 2.3.0` but differ in content and formatting. Swapping
the source replaces the synchronized specification wholesale with an unreviewed
769-schema superset from a different publisher, which is a provenance and
compatibility decision for a maintainer — not a generation repair. Doing it
silently is exactly the change class that can drop specification-referenced union
members without review.

### Required decision

A maintainer must choose the replacement source of truth for the OpenAI
specification and re-run the update so the resulting generated diff can be
audited on its own merits:

1. Point `download_provider_spec` at `openai/openai-openapi` (versioned, may lag
   the live API) and review the full generated diff, or
2. Fetch the Stainless code-samples build output via the `stl` CLI with a
   `STAINLESS_API_KEY` secret, if parity with the previous source is required.

Suggested follow-up regardless of the choice: make the download failure path
diagnosable, since `set -e` currently bypasses the existing error message.
