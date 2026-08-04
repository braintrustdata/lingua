# Coverage Report

Cross-provider transformation coverage report generator for Lingua.

## Overview

This tool runs transformation tests between all provider formats (OpenAI, Anthropic, Google, Bedrock, Responses) and generates a markdown report showing which transformations succeed, fail, or have known limitations.

## What it tests

1. **Request transformations**: Source provider request → Target provider request
2. **Response transformations**: Source provider response → Target provider response
3. **Streaming transformations**: Source streaming events → Target streaming events
4. **Roundtrip tests**: Provider → Universal → Provider (same provider)

## Architecture and difference handling

### Design philosophy

The coverage report tool follows a clean architecture where **runner.rs is mechanically pure**:

- ✅ **Compares values objectively** - no special cases or equivalence decisions
- ✅ **Reports differences accurately** - doesn't hide or transform failures
- ✅ **Queries configuration** - delegates policy decisions to config files

### Where difference handling lives

All policy decisions about acceptable differences belong in one of two places:

1. **Adapter code** (`lingua/src/providers/*/adapter.rs`)
   - Transformation logic (how to convert between formats)
   - Provider-specific defaults and normalization
   - Example: Google/Bedrock model injection (required by those APIs)

2. **Expected differences configuration** (JSON files in `src/`)
   - `requests_expected_differences.json` - Request transformation limitations
   - `responses_expected_differences.json` - Response transformation limitations
   - `streaming_expected_differences.json` - Streaming transformation limitations

### Configuration structure

**Expected differences files** document known provider limitations using a two-tier structure:

```json
{
  "global": [
    {
      "source": "*",
      "target": "Anthropic",
      "fields": [
        { "pattern": "params.top_k", "reason": "OpenAI doesn't support top_k" }
      ],
      "errors": [
        { "pattern": "does not support logprobs", "reason": "Anthropic lacks logprobs" }
      ]
    }
  ],
  "perTestCase": [
    {
      "testCase": "imageContentParam",
      "source": "*",
      "target": "Anthropic",
      "skip": true,
      "reason": "Anthropic assistant messages don't support image content"
    }
  ]
}
```

**Global rules** apply to all test cases for a source→target pair. **Per-test-case rules** apply only to specific tests.

## Provider-specific metadata handling

Certain fields are intentionally lost during cross-provider transformations because they represent provider-specific metadata with no universal equivalent. These are marked as "limitations" in the coverage report:

### Fields that don't translate across providers

**Message/Response IDs** (`id`, `messages[*].id`):
- Each provider uses different ID schemes that represent different concepts:
  - **OpenAI Chat Completions**: Response-level `id` (e.g., `chatcmpl-ABC123`)
  - **OpenAI Responses API**: Response-level `id` (e.g., `resp-XYZ789`)
  - **Anthropic**: Message-level `id` (e.g., `msg_01AbCdEfG`)
  - **Bedrock**: No IDs at all
- These IDs cannot be meaningfully translated across providers

**Timestamps** (`created`, `created_at`):
- Provider-specific generation timestamps with inconsistent field names:
  - **OpenAI Chat**: Uses `created` (Unix timestamp)
  - **OpenAI Responses**: Uses `created_at` (Unix timestamp)
  - **Anthropic/Bedrock**: Don't include timestamps
- Represents when the response was generated, not part of actual content

**Service tier** (`service_tier`):
- OpenAI-specific billing tier indicating account level (`"default"` or `"scale"`)
- Not present in other providers (Anthropic has different usage tracking structure)
- This is API billing metadata, not universal content

**System fingerprint** (`system_fingerprint`):
- OpenAI-specific system identifier for tracking backend changes
- Not present in other providers

The Universal format is intentionally provider-agnostic and doesn't preserve these provider-specific metadata fields during cross-provider transformations.

### How test results are classified

Each test produces one of four outcomes:

1. **Pass** ✅ - Transformation succeeded with no differences
2. **Fail** ❌ - Transformation failed or produced unexpected differences
3. **Limitation** ⚠️ - Differences match documented provider limitations
4. **Skipped** ⊘ - Test case doesn't exist for this provider

The runner mechanically compares values. The expected differences configuration determines which failures are "expected limitations" vs real bugs.

## Usage

```bash
# Run all tests (default)
cargo run --bin coverage-report

# Filter by coverage type
cargo run --bin coverage-report -- --coverage requests
cargo run --bin coverage-report -- --coverage requests,responses
cargo run --bin coverage-report -- --coverage roundtrip

# Filter by test case name
cargo run --bin coverage-report -- --test-cases seedParam
cargo run --bin coverage-report -- -t seedParam,toolCallRequest

# Filter with glob patterns
cargo run --bin coverage-report -- -t "reasoning*"      # All reasoning tests
cargo run --bin coverage-report -- -t "*Param"          # All param tests
cargo run --bin coverage-report -- -t "tool*"           # All tool tests

# Filter by provider
cargo run --bin coverage-report -- --providers responses,anthropic
cargo run --bin coverage-report -- -p anthropic,google

# Filter by source/target direction
cargo run --bin coverage-report -- --source responses --target anthropic
cargo run --bin coverage-report -- --source anthropic

# Combine filters
cargo run --bin coverage-report -- \
  -t seedParam \
  -p responses,anthropic \
  --coverage requests

# Token-optimized compact output (~95% smaller)
cargo run --bin coverage-report -- --format compact
cargo run --bin coverage-report -- -f c
```

## Options

| Option | Short | Description |
|--------|-------|-------------|
| `--coverage` | | Coverage types: `requests`, `responses`, `streaming`, `roundtrip`, `all` |
| `--test-cases` | `-t` | Test case patterns (supports glob: `*` any chars, `?` single char) |
| `--providers` | `-p` | Provider filter (both source AND target must match) |
| `--source` | | Filter source providers only |
| `--target` | | Filter target providers only |
| `--format` | `-f` | Output format: `markdown` (default), `compact` |

## Provider names

| Name | Aliases |
|------|---------|
| `responses` | `response`, `openai-responses` |
| `openai` | `chat-completions`, `chatcompletions`, `completions` |
| `anthropic` | |
| `google` | `gemini` |
| `bedrock` | `converse` |

## Test cases

Test cases are discovered from `payloads/snapshots/`. Each test case directory contains provider-specific request/response JSON files:

```
payloads/snapshots/
├── seedParam/
│   ├── anthropic/
│   │   ├── request.json
│   │   ├── response.json
│   │   └── response-streaming.json
│   ├── responses/
│   ├── chat-completions/
│   ├── google/
│   └── bedrock/
├── toolCallRequest/
├── reasoningRequest/
└── ...
```

## Output formats

### Markdown (default)

The default format outputs detailed markdown with:

- Summary statistics (pass/fail/limitation counts)
- Cross-provider transformation matrix
- Roundtrip test results per provider
- Detailed failure information with diffs
- Collapsible sections for easy navigation

### Compact (token-optimized)

The compact format (`-f compact`) produces ~95% smaller output optimized for LLM consumption:

```
# Coverage (compact)
Stats: 669/1704 (39.3%) [512+157lim] 1035fail
req:617/836 res:32/424 str:20/444

## Failures (79 patterns, 1035 total)

[P1] L:usage.prompt_cache_creation_tokens (123)
  ant→ggl: cacheControl1hParam (response)...(+44)
  ant→oai: cacheControl5mParam (response)...(+41)
```

Key optimizations:
- **Provider abbreviations**: `oai`, `ant`, `ggl`, `bed`, `rsp`
- **Error deduplication**: Groups failures by pattern with counts
- **Test case compression**: `seedParam...(+27)` instead of listing all 28
- **No HTML/markdown overhead**: Plain text with minimal structure

## Examples

### Quick check on a specific test case

```bash
cargo run --bin coverage-report -- -t seedParam --coverage requests
```

### Test Responses→Anthropic transformations

```bash
cargo run --bin coverage-report -- --source responses --target anthropic
```

### Test all reasoning-related features

```bash
cargo run --bin coverage-report -- -t "reasoning*"
```

### Full coverage report (CI)

```bash
cargo run --bin coverage-report > coverage.md
```

### Token-optimized report for LLM analysis

```bash
cargo run --bin coverage-report -- -f compact > coverage.txt
```
