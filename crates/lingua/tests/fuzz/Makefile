.PHONY: run stats

run:
	cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact

stats:
	cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact
