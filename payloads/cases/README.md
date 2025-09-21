# Test Cases Organization

This directory contains all test cases organized by functionality and complexity level.

## Structure

```
cases/
├── types.ts          # Well-defined TypeScript types for all cases
├── models.ts         # Canonical model configuration
├── utils.ts          # Utility functions for working with cases
├── index.ts          # Main export file that combines all cases
├── simple.ts         # Basic functionality tests
├── advanced.ts       # Complex functionality tests
└── README.md         # This file
```

## Adding New Cases

### 1. Create a new case collection file

```typescript
// cases/my-new-cases.ts
import { TestCaseCollection } from "./types";
import { OPENAI_CHAT_COMPLETIONS_MODEL, OPENAI_RESPONSES_MODEL, ANTHROPIC_MODEL } from "./models";

export const myNewCases: TestCaseCollection = {
  myTestCase: {
    "openai-chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Test message" }],
      max_tokens: 100,
    },

    "openai-responses": {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Test message" }],
      max_output_tokens: 100,
    },

    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 100,
      messages: [{ role: "user", content: "Test message" }],
    },
  },
};
```

### 2. Export from index.ts

```typescript
// Add to cases/index.ts
export { myNewCases } from "./my-new-cases";

// Update the merge in allTestCases
import { myNewCases } from "./my-new-cases";
export const allTestCases = mergeCollections(
  simpleCases,
  advancedCases,
  myNewCases
);
```

## Type Safety

All cases are fully typed! No more type assertions needed:

```typescript
// ✅ Fully typed - TypeScript knows this is ChatCompletionCreateParams
const openaiCase = getCaseForProvider(allTestCases, "simpleRequest", "openai-chat-completions");

// ✅ Fully typed - TypeScript knows this is ResponseCreateParams
const responsesCase = getCaseForProvider(allTestCases, "simpleRequest", "openai-responses");

// ✅ Fully typed - TypeScript knows this is MessageCreateParams
const anthropicCase = getCaseForProvider(allTestCases, "simpleRequest", "anthropic");
```

## Benefits

1. **Type Safety**: No more `as` type assertions - everything is properly typed
2. **Organization**: Cases grouped by functionality level (simple, advanced, etc.)
3. **Side-by-side Comparison**: Same test case across different provider APIs
4. **Extensibility**: Easy to add new case collections
5. **Utilities**: Helper functions for common operations
6. **Single Source of Truth**: All cases defined once, used everywhere

## Directory Structure in Snapshots

Cases are organized by case name, then provider type:

```
snapshots/
└── simpleRequest/
    ├── openai-chat-completions/
    │   ├── request.json
    │   ├── response.json
    │   └── ...
    ├── openai-responses/
    │   ├── request.json
    │   ├── response.json
    │   └── ...
    └── anthropic/
        ├── request.json
        ├── response.json
        └── ...
```

This makes it easy to compare how the same logical test case looks across different provider APIs.

## Canonical Models

All test cases use canonical models defined in `cases/models.ts`. This allows you to change the model for all test cases in one place:

```typescript
// cases/models.ts
export const OPENAI_CHAT_COMPLETIONS_MODEL = "gpt-5-nano";
export const OPENAI_RESPONSES_MODEL = "gpt-5-nano";
export const ANTHROPIC_MODEL = "claude-sonnet-4-20250514";
```

**To change models globally:**
1. Update the model name in `cases/models.ts`
2. All test cases will automatically use the new model
3. Cache will be busted and new snapshots will be captured

**Usage in test cases:**
```typescript
// Always import and use constants instead of hardcoded strings
import { OPENAI_CHAT_COMPLETIONS_MODEL } from "./models";
model: OPENAI_CHAT_COMPLETIONS_MODEL  // ✅
model: "gpt-4o-mini"  // ❌ Don't hardcode models
```