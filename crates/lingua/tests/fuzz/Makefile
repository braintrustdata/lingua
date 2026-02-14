.PHONY: run stats prune

run:
	cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact

stats:
	cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact

prune:
	cargo test -p lingua --test fuzz openai_roundtrip_prune_snapshots -- --ignored --nocapture --exact
