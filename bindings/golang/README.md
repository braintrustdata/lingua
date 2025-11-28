# Lingua Go Bindings

Go bindings for Lingua - a universal message format for LLMs - using CGo and Rust FFI.

## Installation

### Prerequisites

- Go 1.25 or higher
- Rust toolchain (for building the native library)
- C compiler (gcc/clang)

### From source

```bash
# Clone the repository
git clone https://github.com/braintrustdata/lingua.git
cd lingua

# Build using Makefile (recommended)
make golang

# Or build manually
cargo build --release --features golang

# Run tests
make test-golang

# Or test manually
cd bindings/golang && go test -v
```

### Using Makefile (recommended)

The main Makefile includes targets for Go bindings:

```bash
make golang          # Build Rust library with golang feature
make test-golang     # Build and run all Go tests
make fmt             # Format all code including Go
make clean           # Clean all build artifacts
make lint-golang     # Run golangci-lint
```

## API

### Message conversions

```go
package main

import (
    "fmt"
    "github.com/braintrustdata/lingua/bindings/golang"
)

func main() {
    // Convert OpenAI Chat Completions messages to universal format
    chatMsgs := []map[string]interface{}{
        {"role": "user", "content": "Hello"},
    }

    linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(chatMsgs)
    if err != nil {
        panic(err)
    }

    // Convert universal format to Anthropic
    anthropicMsgs, err := lingua.LinguaToAnthropicMessages(linguaMsgs)
    if err != nil {
        panic(err)
    }

    fmt.Printf("Anthropic messages: %+v\n", anthropicMsgs)
}
```

### Available conversion functions

#### Chat Completions API
- `ChatCompletionsMessagesToLingua(messages interface{}) ([]map[string]interface{}, error)`
- `LinguaToChatCompletionsMessages(messages interface{}) ([]map[string]interface{}, error)`

#### Responses API
- `ResponsesMessagesToLingua(messages interface{}) ([]map[string]interface{}, error)`
- `LinguaToResponsesMessages(messages interface{}) ([]map[string]interface{}, error)`

#### Anthropic
- `AnthropicMessagesToLingua(messages interface{}) ([]map[string]interface{}, error)`
- `LinguaToAnthropicMessages(messages interface{}) ([]map[string]interface{}, error)`

### Processing functions

```go
// Deduplicate messages based on role and content
deduplicated, err := lingua.DeduplicateMessages(messages)
if err != nil {
    panic(err)
}
```

### Validation

```go
import "encoding/json"

// Validate Chat Completions request
requestJSON := `{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}`
validated, err := lingua.ValidateChatCompletionsRequest(requestJSON)
if err != nil {
    fmt.Printf("Invalid: %v\n", err)
} else {
    fmt.Printf("Valid: %+v\n", validated)
}
```

### Available validation functions

- `ValidateChatCompletionsRequest(jsonStr string) (map[string]interface{}, error)`
- `ValidateChatCompletionsResponse(jsonStr string) (map[string]interface{}, error)`
- `ValidateResponsesRequest(jsonStr string) (map[string]interface{}, error)`
- `ValidateResponsesResponse(jsonStr string) (map[string]interface{}, error)`
- `ValidateAnthropicRequest(jsonStr string) (map[string]interface{}, error)`
- `ValidateAnthropicResponse(jsonStr string) (map[string]interface{}, error)`

### Error handling

All conversion and validation functions return Go's native error type:

```go
result, err := lingua.ChatCompletionsMessagesToLingua(messages)
if err != nil {
    // Handle error
    if convErr, ok := err.(*lingua.ConversionError); ok {
        fmt.Printf("Conversion failed for %s: %s\n", convErr.Provider, convErr.Message)
    }
}
```

## Examples

Example applications are located in `examples/golang/`:

```bash
# Run the full demo
cd examples/golang
go run full_demo.go

# Run basic conversion example
go run basic_conversion.go
```

## Development

### Running tests

```bash
cd bindings/golang
go test -v
```

### Building for different platforms

The CGo bindings link against the Rust static library. Make sure to build the appropriate library for your target platform:

```bash
# Build for current platform
cargo build --release --features golang

# Cross-compile for Linux
cargo build --release --target x86_64-unknown-linux-gnu --features golang

# Cross-compile for macOS
cargo build --release --target x86_64-apple-darwin --features golang

# Cross-compile for Windows
cargo build --release --target x86_64-pc-windows-gnu --features golang
```

### Linking

The Go package uses CGo to link against the Rust library. The default LDFLAGS assume the library is at `../../target/release/liblingua.a`. You can override this by setting CGO_LDFLAGS:

```bash
export CGO_LDFLAGS="-L/path/to/library -llingua -ldl -lm -lpthread"
go build
```

## How it works

The Go bindings use CGo to call into Rust functions exposed via C FFI:

1. **Rust layer** (`src/golang.rs`): Exposes C-compatible functions using `#[no_mangle]` and `extern "C"`
2. **Go layer** (`lingua.go`): Wraps C functions with idiomatic Go APIs
3. **Memory management**: Rust allocates strings, Go calls `lingua_free_string` to deallocate

All data is passed as JSON strings for simplicity and type safety.

## Requirements

- Go 1.25+
- Rust toolchain (for building from source)
- C compiler (gcc, clang, or MSVC)
