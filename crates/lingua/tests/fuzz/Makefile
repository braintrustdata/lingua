.PHONY: run-openai run-anthropic run-anthropic-openai stats-openai stats-anthropic stats-anthropic-openai

run-openai:
	cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact

run-anthropic:
	cargo test -p lingua --test fuzz anthropic_roundtrip -- --ignored --nocapture --exact

run-anthropic-openai:
	cargo test -p lingua --test fuzz anthropic_openai_anthropic -- --ignored --nocapture --exact

stats-openai:
	cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact

stats-anthropic:
	cargo test -p lingua --test fuzz anthropic_roundtrip_stats -- --ignored --nocapture --exact

stats-anthropic-openai:
	cargo test -p lingua --test fuzz anthropic_openai_anthropic_stats -- --ignored --nocapture --exact
