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
│   ├── generated/    # Auto-generated types from Rust
│   └── index.ts      # Main entry point
├── tests/
│   └── roundtrip.test.ts  # Validation tests
├── package.json
└── tsconfig.json
```

## Type Compatibility

The generated types are designed to be compatible with popular LLM SDKs:

- OpenAI SDK (`openai`)
- Anthropic SDK (`@anthropic-ai/sdk`)
- AWS Bedrock (via AWS SDK)

## License

MIT
