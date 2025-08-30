# LLMIR - LLM Intermediate Representation

A universal message format for large language model APIs that compiles to provider-specific formats with zero runtime overhead.

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
OpenAI │ Anthropic │ Google │ ...
```

## Project Structure

```
llmir/
├── src/
│   ├── lib.rs                 # Main library entry
│   ├── universal/             # Universal LLMIR format definitions
│   │   ├── mod.rs
│   │   ├── message.rs         # Core message types
│   │   ├── tools.rs           # Tool definitions
│   │   └── usage.rs           # Token usage reporting
│   ├── providers/             # Provider-specific types and translators
│   │   ├── mod.rs
│   │   ├── openai/            # OpenAI API types and translator
│   │   ├── anthropic/         # Anthropic API types and translator
│   │   └── google/            # Google Gemini API types and translator
│   ├── capabilities/          # Capability detection system
│   │   ├── mod.rs
│   │   └── detection.rs
│   └── translators/           # Translation logic between formats
│       ├── mod.rs
│       ├── openai.rs
│       ├── anthropic.rs
│       └── google.rs
├── examples/                  # Usage examples
├── tests/                     # Integration tests
└── tools/                     # Code generation tools
    ├── spec_converter/        # OpenAPI/TypeScript → Rust converter
    └── capability_gen/        # Auto-generate capability matrices
```

## Status

🚧 **In Development** - Currently building the foundational types and translator architecture.

## Contributing

This project aims to support the entire ecosystem of LLM providers. Contributions for new providers, capability detection improvements, and format enhancements are welcome.

## License

TBD