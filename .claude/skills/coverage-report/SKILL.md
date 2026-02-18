---
name: coverage-report
description: Run transformation coverage tests. Use compact mode when iterating on fixes, full mode when planning or documenting bugs.
---

# Coverage Report Skill

Run cross-provider transformation coverage tests for Lingua.

## When to Use Each Mode

### Compact Mode (`-f compact`) - Use When Iterating

Use compact mode to save tokens (~95% smaller output) when:
- Fixing a specific bug and checking if it's resolved
- Running quick validation after code changes
- Iterating rapidly on implementations
- You already understand the failure patterns

```bash
# Quick check after a code change
cargo run --bin coverage-report -- -f compact

# Filter to specific providers
cargo run --bin coverage-report -- -f compact -p anthropic,responses

# Filter to specific test case
cargo run --bin coverage-report -- -f compact -t seedParam
```

### Full Markdown Mode - Use When Planning/Documenting

Use full mode (default) when:
- Initial investigation of failures (need full error details and diffs)
- Writing bug reports to `.claude/BUGS.md` or `.claude/*_bugs.md`
- Planning implementation approach
- Need to understand the scope of issues

```bash
# Full report for planning
cargo run --bin coverage-report

# Save to bug tracking file
cargo run --bin coverage-report -- -p anthropic > .claude/anthropic_bugs.md
```

## Filtering Options

| Option | Short | Description |
|--------|-------|-------------|
| `--coverage` | | `requests`, `responses`, `streaming`, `roundtrip`, `all` |
| `--test-cases` | `-t` | Test case patterns (glob: `*` any chars, `?` single char) |
| `--providers` | `-p` | Provider filter (both source AND target must match) |
| `--source` | | Filter source providers only |
| `--target` | | Filter target providers only |
| `--format` | `-f` | `markdown` (default), `compact` |

### Common Filter Patterns

```bash
# Only request transformations
cargo run --bin coverage-report -- -f compact --coverage requests

# Test cases matching pattern
cargo run --bin coverage-report -- -f compact -t "reasoning*"
cargo run --bin coverage-report -- -f compact -t "*Param"

# Specific direction
cargo run --bin coverage-report -- -f compact --source anthropic --target responses
```

## Provider Abbreviations (Compact Mode)

In compact output, providers are abbreviated:
- `oai` = ChatCompletions (OpenAI)
- `ant` = Anthropic
- `ggl` = Google
- `bed` = Bedrock
- `rsp` = Responses (OpenAI Responses API)

## Reading Compact Output

```
# Coverage (compact)
Stats: 669/1704 (39.3%) [512+157lim] 1035fail
req:617/836 res:32/424 str:20/444

## Failures (79 patterns, 1035 total)

[P1] L:usage.prompt_cache_creation_tokens (123)
  ant→ggl: cacheControl1hParam (response)...(+44)
```

- `L:` = Lost fields (present in source, missing after transform)
- `A:` = Added fields (not in source, appeared after transform)
- `C:` = Changed fields (value differs after transform)
- `(123)` = Number of test cases with this pattern
- `...(+44)` = 44 more test cases not shown

## Workflow Example

1. **Initial investigation** (full mode):
   ```bash
   cargo run --bin coverage-report -- -p anthropic,responses > .claude/ant_rsp_bugs.md
   ```

2. **Implement fix** and iterate (compact mode):
   ```bash
   cargo run --bin coverage-report -- -f compact -p anthropic,responses
   ```

3. **Verify fix** and update documentation (full mode):
   ```bash
   cargo run --bin coverage-report -- -p anthropic,responses
   # Update .claude/BUGS.md with results
   ```
