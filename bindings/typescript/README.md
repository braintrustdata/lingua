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

// Option 1: Import WASM URL from package (recommended - no fetch needed!)
import wasmUrl from '@braintrust/lingua/browser/lingua_bg.wasm?url';
await init(wasmUrl);

// Option 2: Load from your own public URL
await init('/lingua_bg.wasm');

// Now you can use Lingua functions
const messages = importMessagesFromSpans([
  {
    input: { messages: [{ role: 'user', content: 'Hello!' }] },
    output: { choices: [{ message: { role: 'assistant', content: 'Hi there!' } }] }
  }
]);
```

> **Important**:
> - Use the `?url` suffix when importing the WASM file. This tells the bundler to return the file's URL instead of trying to instantiate it. The bundler will automatically include the file in your build output.
> - You **must** provide a WASM module, path, or URL to `init()`. The function will throw an error if called without an argument (this prevents webpack static analysis issues).

#### Bundler Configuration

**Next.js**: Enable WebAssembly support in `next.config.js`:

```javascript
module.exports = {
  webpack: (config) => {
    config.experiments = {
      asyncWebAssembly: true,
    };
    return config;
  }
};
```

**Vite**: WebAssembly is supported by default. Copy the WASM file to your public directory:

```javascript
// vite.config.js
import { defineConfig } from 'vite';

export default defineConfig({
  // WASM file should be in public/ and will be served at root
});
```

#### WASM File Setup

**Option 1 (Recommended): Let the bundler handle it**

Import the WASM file with `?url` suffix - the bundler will automatically include it:

```typescript
import wasmUrl from '@braintrust/lingua/browser/lingua_bg.wasm?url';
await init(wasmUrl);
```

This works with:
- **Webpack/Next.js**: Treats it as an asset and returns the public URL
- **Vite**: Returns the public URL to the asset
- **Other bundlers**: Most modern bundlers support the `?url` suffix

**Option 2: Manual public directory**

Copy the WASM file from the package to your public/static assets folder:

```bash
cp node_modules/@braintrust/lingua/wasm-web/lingua_bg.wasm public/
```

Then initialize with the public URL:

```typescript
await init('/lingua_bg.wasm');
```

## Development

This package is part of the Lingua monorepo. The TypeScript types are automatically generated from the Rust source code using ts-rs.

### Building

The package uses a two-stage build process:

1. **WASM compilation**: Rust code is compiled to WASM for both Node.js and browser targets
2. **Type bundling**: TypeScript types and entry points are bundled with `tsup`

```bash
pnpm build              # Full build (WASM + types)
pnpm build:wasm         # Build WASM modules only
pnpm build:wasm:node    # Build Node.js WASM target
pnpm build:wasm:web     # Build browser WASM target
pnpm build:types        # Build TypeScript with tsup
```

The build process includes optional `wasm-opt` optimization if available.

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
