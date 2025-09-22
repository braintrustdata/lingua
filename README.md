# LLMIR - A library for creating provider-agnostic LLM inputs & outputs

LLMIR is a library and specification for defining a universal message format for large language model APIs. It enables developers to write messages, model parameters, and tool definitions in a single format that can be translated to and from any model provider's API client-side with zero runtime overhead.

## Goals

- You should be able to write messages, model parameters, and tool definitions in this format, and use them with any version of any model provider.
- The spec describes how message data is represented, and the implementation converts to-and-from model provider APIs and popular frameworks.
- Zero runtime overhead, because there is no execution logic. The sole purpose of this project is to define a universal message format that can be translated across different model providers.

## Anti-goals

- **Framework.** This project is explicitly _not_ providing any higher-level abstractions, guidance on how to structure your app, or model execution support.
- **Proxy.** This format could be used as the foundation for a proxy implementation, but has no concept of actually running prompts or handling authentication.
- **Optimization.** Messages written in this format will execute _exactly_ what you would expect from the model provider. 3rd party optimizers can be built on the format, and those optimizers will naturally work across providers.

## Principles

- Supports 100% of model-provider specific quirks (eg cache breakpoints, stateful responses).
- Messages you write in this format should be safe to store and survive many years of changes in model behavior and API versions.
- Zero dependencies and support for many languages including Typescript, Python, Java, Golang. Ideally can cross-compile or trivial for AI to generate support in language N+1
- Has a precise definition of usage (token) reporting that can be used to compute cost from a standard price table across providers.

## Architecture

```
LLMIR Universal Format
         ↓
    Capability Detection
         ↓
   Provider Translators
         ↓
OpenAI │ Anthropic │ Google │ Bedrock │ ...
```

## Capabilities

[ ... list the known capabilities ... ]

## Compatability matrix

[ .. for each provider, list which capabilities are supported ... ]

## Project structure

```
llmir/
├── src/
│   ├── universal/             # Universal LLMIR format definitions
│   ├── providers/             # Provider-specific API types
│   ├── translators/           # Translation logic between formats
│   ├── capabilities/          # Capability detection system
│   └── lib.rs                 # Main library entry
├── bindings/typescript/       # Auto-generated TypeScript types
├── examples/                  # Usage examples
└── tests/typescript/          # TypeScript compatibility tests
```

## Update pipeline

It is crucial that this library stay up-to-date with the latest model provider APIs, and therefore the pipeline from new spec to implementation to testing should be as automated
as possible. This repo is designed specifically for LLMs to perform every step, with an opportunity for human reviewers to test and contribute to the _taste_ of the data format.

Each provider has _some kind_ of pipeline, runnable through `./pipelines/generate-provider-types.sh <provider>` that generates valid Rust types for its API. OpenAI and Anthropic
both use OpenAPI, while Google uses protobuf. The pipeline works as follows:

- Run `./generate-provider-types.sh <provider>` to download the latest spec and generate Rust types
  - [ ] Fix Anthropic to fetch stats from https://github.com/anthropics/anthropic-sdk-typescript/blob/main/.stats.yml first
- [ ] Automatically testing that within Rust, the provider's type can be losslessly converted to-and-from the universal format.
- [ ] Using an LLM to act on the test outputs and propose updates to the universal format, as needed.
- [ ] Using an LLM to update the compatability matrix, as needed needed.
- [ ] Testing the new capability across all providers to ensure no regressions.

## Testing Strategy

LLMIR employs a comprehensive testing strategy to ensure accurate and lossless conversion between provider-specific formats and the universal format.

### Roundtrip Testing

The core testing approach uses **roundtrip conversion tests** to verify that data can be converted from provider format → universal format → provider format without loss:

```
Provider Payload → Universal ModelMessage → Provider Payload
     (input)            (conversion)          (output)
```

**Key test scenarios:**

1. **Request Roundtrips**:
   - `openai_request → universal → openai_request` (should be identical)
   - `anthropic_request → universal → anthropic_request` (should be identical)

2. **Response Roundtrips**:
   - `openai_response → universal → openai_response` (should be identical)
   - `anthropic_response → universal → anthropic_response` (should be identical)

3. **Cross-Provider Compatibility**:
   - `openai_request → universal → anthropic_request` (should be equivalent)
   - `anthropic_response → universal → openai_response` (should be equivalent)

### Payload-Based Testing

Tests use **real API payloads** captured from actual provider interactions:

- **Payload Snapshots**: Located in `paylods/snapshots/` directory with real request/response examples
- **Comprehensive Coverage**: Tests cover simple messages, tool calls, streaming responses, multi-modal content
- **Version Tracking**: Payloads are version-controlled to detect breaking changes in provider APIs

### Testing Levels

1. **Unit Tests**: Individual conversion functions with synthetic data
2. **Integration Tests**: Full roundtrip tests using real payload snapshots
3. **Compatibility Tests**: Cross-provider conversion validation
4. **Regression Tests**: Ensure updates don't break existing functionality

This strategy ensures LLMIR maintains 100% fidelity when converting between provider formats while providing confidence that the universal format can represent any provider-specific capability.

### Automated Updates

Provider types can be automatically updated using GitHub Actions:

1. **Manual trigger**: Go to Actions → "Update Provider Types" → Run workflow
2. **Choose providers**: Select `all`, or specific providers like `openai,anthropic`
3. **Automatic PR**: If changes are detected, a PR will be created automatically

The automation downloads the latest specifications, regenerates types, applies formatting, and creates a pull request for review.


## Tests / interesting cases

- [ ] Show token accounting across providers. Ideally we give users a way to access the provider's native usage + a unified format.
- [ ] How does structured outputs + Anthropic work? Translate to tool, and parse the response? Does that require carrying some state across request/response? Maybe we can generate an object when performing the forward translation that can be used in the reverse translation.

## Feature Flags

LLMIR supports optional provider dependencies through feature flags to minimize build time and binary size:

### Available Features

- **`openai`** - OpenAI API types and translators
- **`anthropic`** - Anthropic API types and translators  
- **`google`** - Google Gemini API types and translators
- **`bedrock`** - Amazon Bedrock API types and translators (pulls in AWS SDK)

### Usage

**Default (all providers):**
```toml
[dependencies]
llmir = "0.1.0"
```

**Minimal (only OpenAI):**
```toml
[dependencies]
llmir = { version = "0.1.0", default-features = false, features = ["openai"] }
```

**Without AWS dependencies:**
```toml
[dependencies]
llmir = { version = "0.1.0", default-features = false, features = ["openai", "anthropic", "google"] }
```

**Only Bedrock:**
```toml
[dependencies]
llmir = { version = "0.1.0", default-features = false, features = ["bedrock"] }
```

### Conditional Compilation

The translators and types are only available when their respective features are enabled:

```rust
#[cfg(feature = "openai")]
use llmir::translators::to_openai_format;

#[cfg(feature = "bedrock")]
use llmir::translators::to_bedrock_format_with_model;
```

## Status

🚧 **In Development** - Currently building the foundational types and translator architecture.

- [ ] Support parsing streaming responses and combining streaming messages into a single response.

## Contributing

This project aims to support the entire ecosystem of LLM providers. Contributions for new providers, capability detection improvements, and format enhancements are welcome.

## License

TBD
