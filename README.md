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

Not all providers produce openapi specs, and so we'll take advantage of the popularity of TypeScript and strength of its typesystem as a source of truth. The following actions
should be automated:

- [ ] Finding the latest version of a provider's TypeScript library.
- [ ] Using the library to automatically test that LLMIR type for that provider is exactly equivalent to the provider's type.
- [ ] Using an LLM to update the LLMIR type, if needed.
- [ ] Automatically testing that within Rust, the provider's LLMIR type can be losslessly converted to-and-from the universal format.
- [ ] Using an LLM to act on the test outputs and propose updates to the universal format, as needed.
- [ ] Using an LLM to update the compatability matrix, as needed needed.
- [ ] Testing the new capability across all providers to ensure no regressions.

## Status

🚧 **In Development** - Currently building the foundational types and translator architecture.

## Contributing

This project aims to support the entire ecosystem of LLM providers. Contributions for new providers, capability detection improvements, and format enhancements are welcome.

## License

TBD
