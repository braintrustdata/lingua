# @braintrust/lingua-types

Type-only TypeScript definitions for Lingua.

## Installation

```bash
pnpm add -D @braintrust/lingua-types
```

This package does not depend on `@braintrust/lingua-wasm`. Use it when you only need Lingua message and request types.

## Usage

```typescript
import type { Message, UniversalRequest } from "@braintrust/lingua-types";

const messages: Message[] = [{ role: "user", content: "Hello" }];

const request: UniversalRequest = {
  model: "gpt-5.4",
  messages,
  params: {},
};
```
