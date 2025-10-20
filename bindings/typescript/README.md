# @braintrust/lingua

TypeScript bindings for Lingua - a universal message format for LLMs.

## Installation

```bash
pnpm add @braintrust/lingua
# or
npm install @braintrust/lingua
# or
yarn add @braintrust/lingua
```

## Usage

The package provides separate entry points for Node.js and browser environments:

- **`@braintrust/lingua`** or **`@braintrust/lingua/node`** - Node.js with native WASM
- **`@braintrust/lingua/browser`** - Browser with web-targeted WASM (requires initialization)

### Node.js

```typescript
import { Message } from '@braintrust/lingua';

// Create messages in Lingua format
const userMessage: Message = {
  role: 'user',
  content: 'Hello, how are you?'
};

const assistantMessage: Message = {
  role: 'assistant',
  content: 'I'm doing well, thank you!',
  id: 'msg_123'
};
```

### Browser

The browser build requires initialization of the WASM module before use:

```typescript
import { init, importMessagesFromSpans, type Message } from '@braintrust/lingua/browser';

// Option 1: Load from a public URL
await init('/lingua_bg.wasm');

// Option 2: Load from bytes (useful for testing)
import { readFileSync } from 'fs';
import { join } from 'path';

const wasmPath = join(__dirname, 'path/to/lingua_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Now you can use Lingua functions
const messages = importMessagesFromSpans([
  {
    input: { messages: [{ role: 'user', content: 'Hello!' }] },
    output: { choices: [{ message: { role: 'assistant', content: 'Hi there!' } }] }
  }
]);
```

> **Important**:
> - You **must** provide a WASM module, path, or URL to `init()`. The function will throw an error if called without an argument (this prevents webpack static analysis issues).

## Development

This package is part of the Lingua monorepo. The TypeScript types are automatically generated from the Rust source code using ts-rs.

### Generating Types

To regenerate the TypeScript types from Rust:

```bash
pnpm generate
```

### Running Tests

The package includes comprehensive roundtrip tests that validate:

1. Snapshots from the payloads directory are valid according to SDK types
2. Generated Rust types are compatible with SDK types
3. All test data can be parsed and type-checked

Run tests:

```bash
pnpm test
```

### Project Structure

```
bindings/typescript/
├── src/
│   ├── generated/         # Auto-generated types from Rust
│   ├── index.ts           # Node.js entry point
│   └── index.browser.ts   # Browser entry point
├── dist/
│   ├── index.mjs          # Compiled Node.js module
│   ├── index.browser.mjs  # Compiled browser module
│   ├── index.d.mts        # Node.js type definitions
│   └── index.browser.d.mts # Browser type definitions (includes WASM types)
├── wasm/                  # Node.js WASM build
│   └── lingua_bg.wasm
├── wasm-web/              # Browser WASM build
│   └── lingua_bg.wasm
├── tests/
│   └── roundtrip.test.ts  # Validation tests
├── package.json
├── tsconfig.json
└── tsup.config.ts         # Build configuration
```

## Type Compatibility

The generated types are designed to be compatible with popular LLM SDKs:

- OpenAI SDK (`openai`)
- Anthropic SDK (`@anthropic-ai/sdk`)
- AWS Bedrock (via AWS SDK)

## License

MIT
