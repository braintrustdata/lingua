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

This automatically installs `@braintrust/lingua-wasm` as a dependency (you don't need to install it separately).

## Usage

The package provides separate entry points for Node.js and browser environments:

- **`@braintrust/lingua`** or **`@braintrust/lingua/node`** - Node.js with native WASM (auto-initialized)
- **`@braintrust/lingua/browser`** - Browser with web-targeted WASM (requires explicit initialization)

### Node.js

In Node.js, the WASM module is automatically loaded - just import and use:

```typescript
import {
  type Message,
  linguaToChatCompletionsMessages,
  chatCompletionsMessagesToLingua,
} from "@braintrust/lingua";

// Create messages in Lingua format
const messages: Message[] = [
  { role: "user", content: "Hello, how are you?" },
];

// Convert to OpenAI format
const openaiMessages = linguaToChatCompletionsMessages(messages);

// Convert OpenAI response back to Lingua
const linguaMessages = chatCompletionsMessagesToLingua([response.choices[0].message]);
```

### Browser

The browser build requires explicit initialization before use:

```typescript
import init, {
  linguaToChatCompletionsMessages,
  type Message,
} from "@braintrust/lingua/browser";

// Initialize with WASM URL (must be called before using any functions)
await init("/wasm/lingua.wasm");

// Now you can use Lingua functions
const messages: Message[] = [{ role: "user", content: "Hello!" }];
const openaiMessages = linguaToChatCompletionsMessages(messages);
```

**Initialization options:**

```typescript
// Option 1: URL string (fetches the WASM file)
await init("/wasm/lingua.wasm");

// Option 2: ArrayBuffer/Uint8Array (useful for testing or custom loading)
const wasmBuffer = await fetch("/wasm/lingua.wasm").then((r) => r.arrayBuffer());
await init(wasmBuffer);
```

> **Important**: You **must** provide a WASM source to `init()`. The function will throw an error if called without an argument.

### Next.js

For Next.js applications, you need to:

1. **Copy the WASM file to your public directory** during build
2. **Exclude the WASM package from webpack processing**
3. **Initialize on the client side**

#### Step 1: Configure `next.config.mjs`

```javascript
import CopyWebpackPlugin from "copy-webpack-plugin";

/** @type {import('next').NextConfig} */
const nextConfig = {
  webpack: (config, { isServer }) => {
    // Copy WASM file to public directory
    config.plugins.push(
      new CopyWebpackPlugin({
        patterns: [
          {
            from: "node_modules/@braintrust/lingua-wasm/web/lingua_bg.wasm",
            to: "../public/wasm/lingua.wasm",
          },
        ],
      })
    );

    // Exclude lingua-wasm from webpack's WASM processing
    config.module.rules.push({
      test: /\.wasm$/,
      exclude: [/[\\/]lingua-wasm[\\/]/],
      type: "webassembly/async",
    });

    return config;
  },
  // Mark the WASM package as external for server-side
  serverExternalPackages: ["@braintrust/lingua-wasm"],
};

export default nextConfig;
```

#### Step 2: Create a client-side wrapper

```typescript
// lib/lingua.tsx
"use client";

import * as linguaModule from "@braintrust/lingua/browser";
import { useEffect } from "react";

const WASM_URL = "/wasm/lingua.wasm";

let ready: Promise<void> | null = null;

export async function initLingua(): Promise<void> {
  if (!ready) {
    ready = linguaModule.init(WASM_URL).catch((error) => {
      console.error("[Lingua] Initialization failed:", error);
      ready = null;
      throw error;
    });
  }
  return ready;
}

// Optional: React provider to initialize on app load
export function LinguaProvider({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    initLingua();
  }, []);

  return <>{children}</>;
}

// Re-export everything from the browser module
export * from "@braintrust/lingua/browser";
```

#### Step 3: Use in your components

```typescript
"use client";

import { initLingua, linguaToChatCompletionsMessages } from "@/lib/lingua";

async function convertMessages() {
  await initLingua(); // Ensures WASM is loaded

  const messages = [{ role: "user" as const, content: "Hello!" }];
  return linguaToChatCompletionsMessages(messages);
}
```

## Package Architecture

This package uses a two-package architecture:

```
@braintrust/lingua          # TypeScript wrapper (this package)
  └── @braintrust/lingua-wasm  # Raw WASM bindings (auto-installed dependency)
        ├── nodejs/            # Node.js WASM build
        └── web/               # Browser WASM build
```

- **`@braintrust/lingua`** - Pure TypeScript that imports from `@braintrust/lingua-wasm`
- **`@braintrust/lingua-wasm`** - Raw `wasm-pack` output, separate package for clean bundling

This separation ensures webpack/bundlers can properly handle the WASM files without complex configuration.

## Development

This package is part of the Lingua monorepo. The TypeScript types are automatically generated from the Rust source code using ts-rs.

### Building

```bash
# Build WASM first (from repo root)
make lingua-wasm

# Build TypeScript
cd bindings/typescript
pnpm build
```

### Generating Types

To regenerate the TypeScript types from Rust:

```bash
pnpm generate
```

### Running Tests

```bash
pnpm test
```

### Project Structure

```
bindings/
├── lingua-wasm/           # @braintrust/lingua-wasm package
│   ├── nodejs/            # wasm-pack --target nodejs output
│   ├── web/               # wasm-pack --target web output
│   └── package.json
└── typescript/            # @braintrust/lingua package (this directory)
    ├── src/
    │   ├── generated/         # Auto-generated types from Rust
    │   ├── index.ts           # Node.js entry point
    │   ├── index.browser.ts   # Browser entry point
    │   └── wasm-runtime.ts    # WASM initialization logic
    ├── dist/
    │   ├── index.mjs          # Compiled Node.js module
    │   ├── index.browser.mjs  # Compiled browser module
    │   └── *.d.mts            # Type definitions
    ├── tests/
    └── package.json
```

## API Reference

### Conversion Functions

```typescript
// OpenAI Chat Completions ↔ Lingua
linguaToChatCompletionsMessages(messages: Message[]): ChatCompletionMessageParam[]
chatCompletionsMessagesToLingua(messages: ChatCompletionMessage[]): Message[]

// Anthropic ↔ Lingua
linguaToAnthropicMessages(messages: Message[]): MessageParam[]
anthropicMessagesToLingua(messages: AnthropicMessage[]): Message[]

// Validation
validateChatCompletionsRequest(request: unknown): ValidationResult
validateChatCompletionsResponse(response: unknown): ValidationResult
validateAnthropicRequest(request: unknown): ValidationResult
validateAnthropicResponse(response: unknown): ValidationResult
```

### Types

```typescript
interface Message {
  role: "user" | "assistant" | "system" | "tool";
  content: string | ContentPart[];
  id?: string | null;
}
```

## Type Compatibility

The generated types are designed to be compatible with popular LLM SDKs:

- OpenAI SDK (`openai`)
- Anthropic SDK (`@anthropic-ai/sdk`)

## License

MIT
