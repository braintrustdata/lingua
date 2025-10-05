# Lingua typescript example

This example demonstrates Lingua's core value proposition: define your conversation once (including complex tool calls), then execute it with any LLM provider.

## What this shows

A complete tool calling conversation:
1. **User** asks about weather
2. **Assistant** calls weather tool
3. **Tool** returns result
4. **Assistant** uses result to provide final answer

The same conversation runs on both:
- **OpenAI** (gpt-5-nano)
- **Anthropic** (claude-sonnet-4-20250514)

Features demonstrated:
- **Universal format**: One conversation definition
- **Tool calling**: Complex multi-turn with function calls
- **Bidirectional**: Provider format ↔ Lingua format
- **Type safe**: Full TypeScript support
- **Zero overhead**: Compile-time conversion only

## Setup

### Install dependencies

```bash
pnpm install
```

This installs:
- `@braintrust/lingua` - The universal message format library
- `openai` - OpenAI SDK
- `@anthropic-ai/sdk` - Anthropic SDK

### Configure API keys

The example gracefully handles missing API keys - it only runs providers you've configured.

```bash
# Option 1: Set environment variables directly
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...

# Option 2: Use .env file
cp .env.example .env
# Edit .env with your keys
source .env
```

Get API keys:
- OpenAI: https://platform.openai.com/api-keys
- Anthropic: https://console.anthropic.com/settings/keys

## Run

```bash
# Run with configured providers
pnpm start

# Or with inline keys
OPENAI_API_KEY=sk-... ANTHROPIC_API_KEY=sk-ant-... pnpm start

# Or just one provider
OPENAI_API_KEY=sk-... pnpm start
```

## What you'll see

### 1. Lingua conversation (universal format)
The complete conversation including tool calls, defined once:

```typescript
const conversation: Message[] = [
  {
    role: "user",
    content: "What's the weather like in San Francisco?"
  },
  {
    role: "assistant",
    content: [
      {
        type: "tool_call",
        tool_call_id: "call_123",
        tool_name: "get_weather",
        arguments: { type: "valid", location: "San Francisco, CA" }
      }
    ],
    id: null
  },
  {
    role: "tool",
    content: [
      {
        type: "tool_result",
        tool_call_id: "call_123",
        tool_name: "get_weather",
        output: "72°F, sunny"
      }
    ]
  },
];
```

### 2. OpenAI execution
- Converts to OpenAI format
- Calls GPT-5-nano API
- Shows the final response using tool result

### 3. Anthropic execution
- Converts to Anthropic format
- Calls Claude Sonnet 4 API
- Shows the final response using tool result

### 4. Response conversion
Both provider responses are converted back to universal Lingua format.

## Key concepts

### Define once

```typescript
import { type Message } from "@braintrust/lingua";

const conversation: Message[] = [
  { role: "user", content: "Question here" },
  // ... tool calls, results, etc
];
```

### Convert to any provider

```typescript
// OpenAI
const openaiMessages = linguaToChatCompletionsMessages(conversation);
const openaiResponse = await openai.chat.completions.create({
  model: "gpt-5-nano",
  messages: openaiMessages,
});

// Anthropic
const anthropicMessages = linguaToAnthropicMessages(conversation);
const anthropicResponse = await anthropic.messages.create({
  model: "claude-sonnet-4-20250514",
  messages: anthropicMessages,
});
```

### Convert responses back

```typescript
// From OpenAI
const linguaMessages = chatCompletionsMessagesToLingua([response.choices[0].message]);

// From Anthropic
const linguaMessages = anthropicMessagesToLingua([{
  role: "assistant",
  content: response.content
}]);
```

## Installation options

### Option 1: From this repository (development)

```bash
cd examples/typescript
pnpm install
pnpm start
```

Uses local bindings via `file:../../bindings/typescript`.

### Option 2: Standalone (published version)

```bash
# Create new project
mkdir my-lingua-example && cd my-lingua-example

# Copy example
curl -O https://raw.githubusercontent.com/braintrustdata/lingua/main/examples/typescript/index.ts
curl -O https://raw.githubusercontent.com/braintrustdata/lingua/main/examples/typescript/package.json

# Update package.json to use published version
# Change: "@braintrust/lingua": "file:../../bindings/typescript"
# To:     "@braintrust/lingua": "^0.1.0"

# Install and run
npm install
npm start
```

## Benefits

✨ **Write once, run anywhere** - One conversation definition, any provider

🔒 **Type safe** - Full TypeScript support catches errors at compile time

⚡ **Zero overhead** - Pure compile-time translation, no runtime cost

🎯 **Complete coverage** - Supports 100% of provider features including tool calling

🔄 **Bidirectional** - Convert to and from any provider format

🛡️ **Graceful degradation** - Automatically handles missing API keys
