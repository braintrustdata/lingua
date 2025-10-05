# Lingua typescript example

This example demonstrates how to use Lingua's universal message format to write provider-agnostic LLM code.

## What does this example show?

- **Universal message format**: Define messages once in Lingua format
- **Multi-provider support**: Convert to OpenAI, Anthropic, or any supported provider
- **Zero runtime overhead**: All conversions happen at compile time
- **Type safety**: Full TypeScript support with autocomplete
- **Tool call handling**: Shows how to work with function calling

## Running the example

### Option 1: From this repository (for development)

```bash
# Install dependencies
pnpm install

# Run the example
pnpm example

# Or run with watch mode
pnpm watch
```

### Option 2: Standalone (for quick start)

```bash
# Create a new directory
mkdir my-lingua-example
cd my-lingua-example

# Copy the example files
curl -O https://raw.githubusercontent.com/braintrustdata/lingua/main/examples/typescript/index.ts
curl -O https://raw.githubusercontent.com/braintrustdata/lingua/main/examples/typescript/package.json

# Update package.json to use published version
# Change: "@braintrust/lingua": "file:../../bindings/typescript"
# To:     "@braintrust/lingua": "^0.1.0"

# Install and run
pnpm install
pnpm example
```

### Option 3: Global installation

```bash
# Install globally
npm install -g @braintrust/lingua

# Or use directly
npx @braintrust/lingua
```

## What you'll see

The example converts the same Lingua messages to multiple provider formats:

1. **Simple messages** - Basic user/assistant conversation
2. **Tool calls** - Function calling with results

Each conversion produces the exact format expected by the target provider's API.

## Key concepts

### Universal message format

```typescript
const messages: Message[] = [
  {
    role: "user",
    content: "What is the capital of France?",
  },
];
```

### Convert to any provider

```typescript
// Convert to OpenAI format
const openaiMessages = linguaToChatCompletionsMessages(messages);

// Convert to Anthropic format
const anthropicMessages = linguaToAnthropicMessages(messages);
```

### Use with actual API calls

```typescript
import OpenAI from "openai";
import Anthropic from "@anthropic-ai/sdk";

// Use with OpenAI
const openai = new OpenAI();
const openaiResponse = await openai.chat.completions.create({
  model: "gpt-4",
  messages: linguaToChatCompletionsMessages(linguaMessages),
});

// Use with Anthropic
const anthropic = new Anthropic();
const anthropicResponse = await anthropic.messages.create({
  model: "claude-3-5-sonnet-20241022",
  max_tokens: 1024,
  messages: linguaToAnthropicMessages(linguaMessages),
});
```

## Next steps

- Check out the [Lingua documentation](https://github.com/braintrustdata/lingua)
- See more examples in the [examples directory](../)
- Learn about [provider-specific features](../../README.md#provider-support)

## Benefits

✨ **Write once, run anywhere** - Define your prompts and conversations in one format

🔒 **Type safe** - Full TypeScript support catches errors at compile time

⚡ **Zero overhead** - Conversions are pure functions with no runtime cost

🎯 **Complete coverage** - Supports 100% of provider-specific features

🔄 **Bidirectional** - Convert to and from any provider format
