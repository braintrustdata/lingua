# Payload Capture

Scripts to capture OpenAI and Anthropic API payloads with TypeScript type safety.

## Table of contents

- [Quick start](#quick-start)
- [Purpose](#purpose)
- [Installation](#installation)
- [Environment variables](#environment-variables)
- [Capture usage](#usage)
- [Validation tool](#validation-tool)
- [Import span fixtures](#import-span-fixtures)
- [Output structure](#output-structure)
- [Example payloads](#example-payloads)
- [Type checking](#type-checking)
- [Extending](#extending)

## Quick start

### 1. Define your case

Add an entry to `cases/simple.ts` (or `advanced.ts`, `params.ts`) with provider definitions:

```typescript
myCase: {
  "chat-completions": {
    model: OPENAI_CHAT_COMPLETIONS_MODEL,
    messages: [{ role: "user", content: "Hello" }],
  },
  responses: {
    model: OPENAI_RESPONSES_MODEL,
    input: [{ role: "user", content: "Hello" }],
  },
  anthropic: {
    model: ANTHROPIC_MODEL,
    max_tokens: 20_000,
    messages: [{ role: "user", content: "Hello" }],
  },
  google: null,   // null = skip this provider
  bedrock: null,
},
```

### 2. Capture and test

```bash
make capture FILTER=myCase        # Captures snapshots + transforms + updates vitest snapshots
make capture-transforms FORCE=1                        # Re-capture all transforms
make capture-transforms PAIR=chat-completions,google   # Re-capture only one pair
make test-payloads                # Runs transform tests + sync check
```

`make capture` runs 3 phases automatically:
1. **Provider snapshots** — calls each SDK (OpenAI, Anthropic, Google, Bedrock) and saves request/response pairs to `snapshots/`
2. **Transform captures** — transforms requests across providers via WASM (e.g. chat-completions → anthropic), calls the target SDK, saves responses to `transforms/`
3. **Vitest snapshot update** — auto-runs `vitest -u` to update transform test snapshots with new data

`make test-payloads` runs:
1. **Transform tests** — for each provider pair, transforms the request via WASM, validates against the target schema, loads the captured response, transforms it back, and snapshots both directions
2. **Sync check** — verifies all test cases have snapshots and all transformable cases have transform captures

`cargo test -p coverage-report` (runs as part of `make test`) validates roundtrip transformations across all providers using the `snapshots/` data

### 3. Auto-regenerate failed transforms

When `make test-payloads` fails due to snapshot mismatches, automatically regenerate only the failed transform captures with real API calls:

```bash
make regenerate-failed-transforms   # Detect failures, recapture failed cases, re-run tests
# or
make test-payloads REGENERATE=1     # Same as above, combined with test run
```

#### How it works

**The problem:** When you change transformation code (e.g., changing `output_format` → `output_config`), your tests fail with snapshot mismatches:

```diff
- Expected (old snapshot): "output_format": { ... }
+ Received (new code):      "output_config": { "format": { ... } }
```

**What `make regenerate-failed-transforms` does:**

1. **Runs tests** and detects which cases failed
2. **Extracts case names** from failed test names (e.g., `"chat-completions → anthropic > textFormatJsonObjectParam"` → `"textFormatJsonObjectParam"`)
3. **Recaptures only those cases** by:
   - Transforming the request via WASM (using your NEW code)
   - Sending the transformed request to the real provider API (Anthropic, OpenAI, etc.)
   - Verifying the provider accepts the new format (or errors if invalid)
   - Saving the actual API response to `transforms/`
4. **Updates vitest snapshots** to match the new format
5. **Re-runs tests** to verify everything passes

**Why recapture instead of just updating snapshots?**

Because we need to verify that your code changes produce requests that the actual provider APIs accept. For example:
- ✅ Anthropic accepts the new `output_config` format → tests pass
- ❌ Anthropic rejects it → you see the error immediately

Updating snapshots without API validation would make tests pass without verifying the new format works with real APIs.

#### When to use each approach

| Scenario | Command | Reason |
|----------|---------|--------|
| **Code changed transform logic** | `make regenerate-failed-transforms` | Verify new format works with real provider APIs |
| **Added new test cases** | `make capture --force` | Capture responses for new cases across all providers |
| **Manual regeneration of specific cases** | `make capture CASES=case1,case2 FORCE=1` | Target specific cases by exact name |

**Note:** The `CASES` variable accepts exact case names (comma-separated), while `FILTER` does substring matching.

### Example: what happens for `simpleRequest`

**`make capture FILTER=simpleRequest`** produces:

```
snapshots/simpleRequest/
├── chat-completions/        # Raw OpenAI chat completions request + response
├── responses/               # Raw OpenAI responses request + response
├── anthropic/               # Raw Anthropic request + response
├── google/                  # Raw Google request + response
└── bedrock/                 # Raw Bedrock request + response

transforms/
├── chat-completions_to_anthropic/simpleRequest.json   # OpenAI request → WASM → Anthropic request → Anthropic SDK → response
├── chat-completions_to_responses/simpleRequest.json   # OpenAI request → WASM → Responses request → Responses SDK → response
├── responses_to_anthropic/simpleRequest.json
├── responses_to_chat-completions/simpleRequest.json
├── anthropic_to_chat-completions/simpleRequest.json
└── anthropic_to_responses/simpleRequest.json
```

**`make test-payloads`** then verifies for each transform pair (e.g. `chat-completions → anthropic`):
1. Transform the chat-completions request → Anthropic format via WASM
2. Validate the transformed request against Lingua's Anthropic schema
3. Load the captured Anthropic response from `transforms/`
4. Transform the response back → chat-completions format via WASM
5. Snapshot both the transformed request and response for regression testing

Note: Lingua's WASM transform passes the model name through unchanged, so both the capture script and the test override it to the target provider's model (e.g. `claude-sonnet-4-5-20250929` for Anthropic targets). This mapping lives in `TARGET_MODELS` in `scripts/transforms/helpers.ts`, sourced from `cases/models.ts`.

## Purpose

This package provides scripts to systematically capture real API requests and responses from OpenAI and Anthropic, using their official TypeScript types to ensure payload validity. This creates a repository of real-world test cases for AI API compatibility testing.

## Installation

```bash
pnpm install
```

## Environment Variables

- `OPENAI_API_KEY`: Required for capturing OpenAI payloads
- `ANTHROPIC_API_KEY`: Required for capturing Anthropic payloads

## Usage

### Unified Capture Script (Recommended)

```bash
# List all available cases and their capture status
pnpm capture --list

# Capture specific providers (--provider or --providers both work)
pnpm capture --providers openai-chat,anthropic
pnpm capture --provider openai-responses

# Capture specific cases across all providers (--case or --cases both work)
pnpm capture --cases simple,toolCall
pnpm capture --case reasoning

# Filter cases by name pattern
pnpm capture --filter reasoning

# Control streaming behavior
pnpm capture --stream true   # Streaming only
pnpm capture --stream false  # Non-streaming only
# (default: both streaming and non-streaming)

# Force re-capture (skip already captured check)
pnpm capture --force

# Combine filters
pnpm capture --providers openai-responses --cases matrix,python --force --stream false
```

**Smart Re-run Detection**: By default, already captured cases are skipped. Use `--force` to re-capture.

**Static Snapshots**: All results save to `snapshots/` directory (no timestamps) for consistent file paths.

### Individual Scripts (Legacy)

You can still use individual provider scripts if needed:

```bash
pnpm capture-openai -- --filter toolCall
pnpm capture-openai-responses -- --filter toolCall
pnpm capture-anthropic
```

### Type Safety

All payloads are defined using the official provider TypeScript types:

```typescript
// OpenAI payloads use OpenAI.ChatCompletionCreateParams
const openaiRequest = {
  model: "gpt-4",
  messages: [{ role: "user", content: "Hello" }],
} satisfies OpenAI.ChatCompletionCreateParams;

// Anthropic payloads use Anthropic.MessageCreateParams
const anthropicRequest = {
  model: "claude-3-5-sonnet-20241022",
  max_tokens: 100,
  messages: [{ role: "user", content: "Hello" }],
} satisfies Anthropic.MessageCreateParams;
```

## Validation tool

Validate any LLM proxy by comparing responses to captured snapshots.

### Basic usage

```bash
# Validate chat-completions format through a proxy
pnpm validate --proxy-url http://localhost:8080

# Use Braintrust API key
pnpm validate --proxy-url http://localhost:8080 --api-key $BRAINTRUST_API_KEY
```

### Testing different models

The `--models` flag lets you test the same format with different provider models:

```bash
# Test with both OpenAI and Anthropic models through chat-completions format
pnpm validate --proxy-url http://localhost:8080 --models openai,anthropic

# Test specific model
pnpm validate --proxy-url http://localhost:8080 --models anthropic
```

### Options

| Flag | Description |
|------|-------------|
| `--proxy-url <url>` | Proxy URL (required) |
| `--api-key <key>` | API key for gateway |
| `--format <formats>` | Formats to test: `chat-completions`, `responses`, `anthropic` |
| `--models <models>` | Model providers: `openai`, `anthropic`, `google`, `bedrock` |
| `--cases <cases>` | Specific cases to run |
| `--all` | Run all cases (including slow ones) |
| `--verbose` | Show full diff details |

### Example output

```
Validating proxy at http://localhost:8080...

chat-completions
  ✓ simpleRequest [gpt-5-nano] (4124ms)
  ✓ simpleRequest [claude-sonnet-4-20250514] (11033ms)
  ✓ toolCallRequest [gpt-5-nano] (7344ms)
  ✓ reasoningRequest [gpt-5-nano] (26859ms)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Summary: 4 passed, 0 failed
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Import span fixtures

For span-import tests (`input`/`output` -> Lingua messages), use:

- `payloads/import-cases/`

Detailed workflow and file format:

- `payloads/import-cases/README.md`

Anonymize content strings in a span fixture (from repo root):

```bash
pnpm --dir payloads anonymize -- import-cases/<name>.spans.json
```

Generate missing assertions from spans (from repo root):

```bash
GENERATE_MISSING=1 cargo test -p lingua --test import_fixtures -- --nocapture
```

## Output structure

Payloads are saved to `snapshots/` directory with the following naming:

**OpenAI Chat Completions API files (per example):**

- `openai-[name]-request.json` - Original request payload
- `openai-[name]-response-non-streaming.json` - Non-streaming response
- `openai-[name]-response-streaming.json` - Streaming response chunks
- `openai-[name]-followup-request.json` - Follow-up conversation request
- `openai-[name]-followup-response-non-streaming.json` - Follow-up non-streaming response
- `openai-[name]-followup-response-streaming.json` - Follow-up streaming response chunks

**OpenAI Responses API files (per example):**

- `openai-responses-[name]-request.json` - Original request payload
- `openai-responses-[name]-response.json` - Non-streaming response with reasoning tokens and output
- `openai-responses-[name]-response-streaming.json` - Streaming response chunks
- `openai-responses-[name]-followup-request.json` - Follow-up conversation request
- `openai-responses-[name]-followup-response.json` - Non-streaming follow-up response
- `openai-responses-[name]-followup-response-streaming.json` - Streaming follow-up response chunks

**Anthropic files:**

- `anthropic-[name]-request.json` - Anthropic request payload
- `anthropic-[name]-response.json` - Anthropic non-streaming response payload
- `anthropic-[name]-response-streaming.json` - Anthropic streaming response chunks

**Error files:**

- `*-error.json` - Error details if API call fails

## Example Payloads

The scripts currently capture these payload types:

### OpenAI (Chat Completions API)

- Simple chat completion
- Reasoning model requests (using gpt-5-nano)
- Function calling with tools
- Matrix transpose bash script (from reasoning guide)
- React component refactoring (from reasoning guide)
- Python app planning (from reasoning guide)
- STEM research about antibiotics (from reasoning guide)
- Capital of France simple example (from reasoning guide)

**For each OpenAI Chat Completions example, the script captures:**

- Original request payload
- Non-streaming response (parallel execution)
- Streaming response (parallel execution as list of chunks)
- Follow-up conversation with "what next?" user message
- Follow-up non-streaming and streaming responses (parallel execution)

### OpenAI (Responses API)

Same examples as above but using OpenAI's Responses API with:

- **Reasoning effort levels**: low/medium/high based on task complexity
- **Reasoning summaries**: Some examples include summary output
- **Enhanced reasoning**: Better suited for reasoning models like gpt-5-nano

**For each OpenAI Responses example, the script captures:**

- Original request payload with reasoning parameters
- Response with reasoning tokens and structured output
- Follow-up conversation with "what next?" user message
- Follow-up response

**Performance optimization:** All examples run in parallel for maximum speed.

### Anthropic

- Basic message creation
- System prompt usage
- Tool calling
- Thinking/reasoning examples

## Type Checking

```bash
pnpm typecheck
```

Validates that all TypeScript code is properly typed using the official SDK types.

## Extending

To add a new test case:

1. Define the case in `cases/simple.ts`, `cases/advanced.ts`, or `cases/params.ts` with provider definitions for each format
2. Run `pnpm capture --filter yourCase` to capture snapshots, transform responses, and update vitest snapshots
3. Run `pnpm test` to verify everything is in sync

The sync test (`scripts/sync.test.ts`) will catch any missing snapshots or transform captures automatically.
