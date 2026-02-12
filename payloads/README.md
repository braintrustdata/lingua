# Payload Capture

Scripts to capture OpenAI and Anthropic API payloads with TypeScript type safety.

## Table of contents

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

Quick command (from `payloads/`):

```bash
pbpaste | pnpm new-import-case --name my-case-name
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

To add new payload types:

1. Add the payload definition to the appropriate script using the provider's TypeScript types
2. Use `satisfies` to ensure type safety
3. Run the capture script to generate the new payloads

This approach ensures all captured payloads are valid according to the official API specifications.
