# AI SDK Capture System Guide

## Overview

The Lingua capture system is designed to execute and capture API calls across multiple LLM providers (OpenAI, Anthropic, AI SDK) for testing and comparison purposes. This document explains how the system works and how AI SDK has been integrated.

## Architecture

### Core Components

1. **Test Cases** (`payloads/cases/`)
   - Unified test case definitions that work across all providers
   - Each test case defines the same logical test for multiple provider APIs
   - Located in `simple.ts`, `advanced.ts`, etc.

2. **Provider Executors** (`payloads/scripts/providers/`)
   - Provider-specific implementations that handle API execution
   - Each executor knows how to call its provider's API and capture responses
   - Files: `openai.ts`, `anthropic.ts`, `openai-responses.ts`, `ai-sdk.ts`

3. **Capture Script** (`payloads/scripts/capture.ts`)
   - Orchestrates parallel execution of all providers
   - Manages caching and regeneration logic
   - Saves snapshots to disk

4. **Snapshots** (`payloads/snapshots/`)
   - Stored request/response data organized by case name and provider
   - Used for regression testing and comparison

## How Capture Works

### 1. Test Case Definition

Test cases are defined in a unified structure where each case has variants for different providers:

```typescript
export const simpleCases: TestCaseCollection = {
  simpleRequest: {
    "chat-completions": { /* OpenAI params */ },
    "responses": { /* OpenAI Responses params */ },
    "anthropic": { /* Anthropic params */ },
    "ai-sdk": { /* AI SDK params */ }
  }
}
```

### 2. Provider Execution Flow

Each provider executor follows this pattern:

```typescript
1. Extract test cases for the provider
2. Execute API calls (both streaming and non-streaming)
3. Capture initial response
4. Generate follow-up conversation
5. Execute follow-up calls
6. Return all captured data
```

### 3. Parallel Execution

The capture script runs all providers in parallel:
- Each provider runs independently
- Within each provider, streaming and non-streaming calls run in parallel
- Follow-up calls also run in parallel
- This maximizes throughput and reduces total execution time

### 4. Snapshot Storage

Results are saved in this structure:
```
snapshots/
└── [caseName]/
    └── [provider]/
        ├── request.json
        ├── response.json
        ├── streamingResponse.json
        ├── followupRequest.json
        ├── followupResponse.json
        └── followupStreamingResponse.json
```

## AI SDK Integration

### Test Case Structure

AI SDK test cases use the Vercel AI SDK format:

```typescript
"ai-sdk": {
  model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
  messages: [
    {
      role: "user",
      content: "What is the capital of France?",
    },
  ],
  maxOutputTokens: 100,  // Note: uses maxOutputTokens, not max_tokens
  tools: {
    get_weather: tool({
      description: "Get the current weather",
      parameters: z.object({
        location: z.string().describe("City and state"),
      }),
    }),
  },
  toolChoice: "auto",
}
```

### Key Differences from Other Providers

1. **Model Definition**: Uses `openai(modelName)` from `@ai-sdk/openai`
2. **Tool Definition**: Uses `tool()` helper with Zod schemas
3. **Parameter Names**: Uses camelCase (e.g., `maxOutputTokens` vs `max_tokens`)
4. **Streaming**: Uses `textStream` iterator instead of event-based streaming

### AI SDK Provider Executor

The `ai-sdk.ts` provider executor handles:

1. **Non-streaming calls**: Uses `generateText()`
2. **Streaming calls**: Uses `streamText()` and iterates over `textStream`
3. **Tool handling**: Captures tool calls and results in follow-ups
4. **Message format**: Converts between AI SDK's `CoreMessage` format

## Running Captures

### Basic Usage

```bash
# Capture all cases for all providers
pnpm capture

# List available cases without running
pnpm capture --list

# Force regeneration (ignore cache)
pnpm capture --force

# Filter by case name
pnpm capture --filter simpleRequest

# Run specific providers only
pnpm capture --providers ai-sdk,anthropic

# Run specific cases only
pnpm capture --cases simpleRequest,toolCallRequest

# Control streaming behavior
pnpm capture --stream true   # Streaming only
pnpm capture --stream false  # Non-streaming only
```

### Adding New Test Cases

1. Add the case to the appropriate file in `payloads/cases/`:

```typescript
export const myCases: TestCaseCollection = {
  myNewCase: {
    "ai-sdk": {
      model: openai(OPENAI_CHAT_COMPLETIONS_MODEL),
      messages: [{ role: "user", content: "Test" }],
    },
    // Add other provider variants...
  }
}
```

2. Export from `payloads/cases/index.ts`:

```typescript
import { myCases } from "./my-cases";
export const allTestCases = mergeCollections(
  simpleCases,
  advancedCases,
  myCases
);
```

3. Run capture:

```bash
pnpm capture --cases myNewCase
```

## Follow-up Conversations

The capture system automatically generates follow-up conversations to test multi-turn interactions:

### For Regular Messages
- Adds assistant's response to conversation
- Adds user follow-up: "What should I do next?"
- Executes another API call with the extended conversation

### For Tool Calls
- Adds assistant's response with tool calls
- Adds dummy tool responses (e.g., "71 degrees" for weather)
- Executes another API call with tool results

This tests:
- Context retention
- Tool result handling
- Multi-turn conversation flow
- Streaming consistency

## Caching System

The capture system uses smart caching to avoid redundant API calls:

1. **Cache Key**: Generated from provider + case name + payload hash
2. **Cache Check**: Before running, checks if files exist and match cache
3. **Cache Update**: After successful capture, updates cache manifest
4. **Force Regeneration**: Use `--force` flag to bypass cache

Cache benefits:
- Saves API costs
- Speeds up iterative development
- Ensures consistency in testing

## Troubleshooting

### Common Issues

1. **Missing API Keys**
   - Set `OPENAI_API_KEY` for AI SDK/OpenAI
   - Set `ANTHROPIC_API_KEY` for Anthropic

2. **Type Errors**
   - AI SDK uses different parameter names (camelCase)
   - Tools use Zod schemas, not JSON schemas
   - Model uses wrapped instance, not string

3. **Streaming Issues**
   - AI SDK streaming uses async iterators
   - Check for proper await handling
   - Ensure stream is fully consumed

### Debugging Tips

1. **Check Individual Provider**:
```bash
pnpm capture --providers ai-sdk --cases simpleRequest
```

2. **Examine Snapshots**:
```bash
cat snapshots/simpleRequest/ai-sdk/request.json | jq
```

3. **Add Console Logging**:
Add temporary logging in `providers/ai-sdk.ts`:
```typescript
console.log("Request:", JSON.stringify(payload, null, 2));
console.log("Response:", response);
```

## Next Steps

### Complete AI SDK Integration

1. ✅ Created `providers/ai-sdk.ts` executor
2. ✅ Added to capture script
3. ✅ Fixed test case formats in `simple.ts`
4. ⏳ Add remaining test cases in `advanced.ts`
5. ⏳ Run full capture to generate snapshots
6. ⏳ Validate snapshot format and content

### Testing Checklist

- [ ] Verify all test cases have ai-sdk variants
- [ ] Run capture for all ai-sdk cases
- [ ] Compare snapshots with other providers
- [ ] Test streaming consistency
- [ ] Validate follow-up conversations
- [ ] Check tool call handling

### Future Enhancements

1. **Model Providers**: Add support for other AI SDK providers (Anthropic, Cohere, etc.)
2. **Advanced Features**: Test AI SDK-specific features like `experimental_telemetry`
3. **Error Cases**: Add test cases for error handling
4. **Performance**: Benchmark response times across providers
5. **Validation**: Add automated snapshot validation

## Summary

The AI SDK integration into Lingua's capture system enables:
- Side-by-side comparison with other LLM providers
- Comprehensive testing of AI SDK features
- Snapshot-based regression testing
- Parallel execution for efficiency

The system is designed to be extensible, allowing easy addition of new test cases and providers while maintaining consistency across the entire test suite.
