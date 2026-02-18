.PHONY: run stats prune run-openai stats-openai prune-openai run-responses stats-responses prune-responses run-anthropic stats-anthropic prune-anthropic run-two-arm stats-two-arm prune-two-arm run-three-arm stats-three-arm prune-three-arm refresh-snapshots

run: run-openai

stats: stats-openai

prune: prune-openai

run-openai:
	cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact

stats-openai:
	cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact

prune-openai:
	cargo test -p lingua --test fuzz openai_roundtrip_prune_snapshots -- --ignored --nocapture --exact

run-responses:
	cargo test -p lingua --test fuzz responses_roundtrip -- --ignored --nocapture --exact

stats-responses:
	cargo test -p lingua --test fuzz responses_roundtrip_stats -- --ignored --nocapture --exact

prune-responses:
	cargo test -p lingua --test fuzz responses_roundtrip_prune_snapshots -- --ignored --nocapture --exact

run-anthropic:
	cargo test -p lingua --test fuzz anthropic_roundtrip -- --ignored --nocapture --exact

stats-anthropic:
	cargo test -p lingua --test fuzz anthropic_roundtrip_stats -- --ignored --nocapture --exact

prune-anthropic:
	cargo test -p lingua --test fuzz anthropic_roundtrip_prune_snapshots -- --ignored --nocapture --exact

run-two-arm:
	cargo test -p lingua --test fuzz chat_anthropic_two_arm -- --ignored --nocapture --exact

stats-two-arm:
	cargo test -p lingua --test fuzz chat_anthropic_two_arm_stats -- --ignored --nocapture --exact

prune-two-arm:
	cargo test -p lingua --test fuzz chat_anthropic_two_arm_prune_snapshots -- --ignored --nocapture --exact

run-three-arm:
	cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm -- --ignored --nocapture --exact

stats-three-arm:
	cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm_stats -- --ignored --nocapture --exact

prune-three-arm:
	cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm_prune_snapshots -- --ignored --nocapture --exact

# Run ignored fuzz generators (which may fail while still writing snapshots),
# then prune/dedupe snapshot corpus.
refresh-snapshots:
	-cargo test -p lingua --test fuzz openai_roundtrip -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz openai_roundtrip_stats -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz responses_roundtrip -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz responses_roundtrip_stats -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz anthropic_roundtrip -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz anthropic_roundtrip_stats -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz chat_anthropic_two_arm -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz chat_anthropic_two_arm_stats -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm -- --ignored --nocapture --exact
	-cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm_stats -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz openai_roundtrip_prune_snapshots -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz responses_roundtrip_prune_snapshots -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz anthropic_roundtrip_prune_snapshots -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz chat_anthropic_two_arm_prune_snapshots -- --ignored --nocapture --exact
	cargo test -p lingua --test fuzz chat_responses_anthropic_three_arm_prune_snapshots -- --ignored --nocapture --exact
