# Provider type update notes

## BLOCKING: openai specification download unavailable (2026-11-02)

**Status:** blocked on an external service. No repository defect found.

`./pipelines/generate-provider-types.sh openai` fails in step 1 before any type
generation runs. The upstream spec host returns a Cloudflare 522 (origin
connection timed out):

```
$ curl -sS -D - -o /dev/null https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml
HTTP/2 522
server: cloudflare
```

Scope of the outage:

- `https://app.stainless.com/` (host root) also returns 522, so the origin is
  down rather than the spec path being moved or renamed.
- Three consecutive attempts returned 522 after ~20s each.
- Unrelated egress is healthy (`raw.githubusercontent.com` returns 200), so this
  is not a sandbox or proxy restriction.

The pipeline already stages the download in a temp directory and only moves it
into place on success, so `specs/openai/openapi.yml` was correctly left intact.

**Verified unaffected by the outage** (run against the committed
`specs/openai/openapi.yml`):

- `cargo run --bin generate-types -- openai` succeeds and reproduces the
  checked-in `crates/lingua/src/providers/openai/generated.rs` byte for byte
  (`git diff` is empty).
- `cargo build` succeeds.
- `cargo test -p lingua --lib openai` — 522 passed, 0 failed.

No union member, discriminator, or required field changed, because no new
specification was obtained. No fallback spec source, cached substitute, or
synthetic schema content was introduced.

**Resolution:** re-run the update once `app.stainless.com` is serving again.
