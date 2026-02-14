# Contributing

## Fuzz Snapshot Workflow

Use the top-level make target to refresh fuzz snapshots:

```bash
make fuzz-snapshots
```

What it does:

1. Runs ignored fuzz tests that generate/update snapshots:
   `openai_roundtrip`, `openai_roundtrip_stats`
2. Runs snapshot prune/dedupe:
   `openai_roundtrip_prune_snapshots`

Notes:

- The generation tests may fail while still producing useful snapshots; this is expected in this workflow.
- Prune is conservative: it removes malformed/orphan files and dedupes failures by normalized reason.

If you only want to dedupe/prune existing fuzz snapshots:

```bash
make fuzz-snapshots-prune
```
