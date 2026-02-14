.PHONY: run stats prune refresh-snapshots

run:
	cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact

stats:
	cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact

prune:
	cargo test -p lingua --test fuzz openai_roundtrip_prune_snapshots -- --ignored --nocapture --exact

# Run ignored fuzz generators (which may fail while still writing snapshots),
# then prune/dedupe snapshot corpus.
refresh-snapshots:
	-cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz openai_roundtrip_prune_snapshots -- --ignored --nocapture --exact
