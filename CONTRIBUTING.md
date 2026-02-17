# Contributing

## Fuzz Snapshot Workflow

Use the top-level make target to refresh fuzz snapshots:

```bash
make fuzz-snapshots
```

What it does:

1. Runs ignored fuzz tests that generate/update snapshots:
   `openai_roundtrip`, `openai_roundtrip_stats`
   `responses_roundtrip`, `responses_roundtrip_stats`
   `anthropic_roundtrip`, `anthropic_roundtrip_stats`
   `chat_anthropic_two_arm`, `chat_anthropic_two_arm_stats`
   `chat_responses_anthropic_three_arm`, `chat_responses_anthropic_three_arm_stats`
2. Runs snapshot prune/dedupe:
   `openai_roundtrip_prune_snapshots`
   `responses_roundtrip_prune_snapshots`
   `anthropic_roundtrip_prune_snapshots`
   `chat_anthropic_two_arm_prune_snapshots`
   `chat_responses_anthropic_three_arm_prune_snapshots`

Notes:

- The generation tests may fail while still producing useful snapshots; this is expected in this workflow.
- Prune is conservative: it removes malformed/orphan files and dedupes failures by normalized reason.

If you only want to dedupe/prune existing fuzz snapshots:

```bash
make fuzz-snapshots-prune
```
