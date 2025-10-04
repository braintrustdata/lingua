# Lingua Python Bindings

Python bindings for Lingua - a universal message format for LLMs - using PyO3.

## Installation

### From source

```bash
# Install uv if you haven't already
curl -LsSf https://astral.sh/uv/install.sh | sh

# Navigate to bindings directory
cd bindings/python

# Install dependencies and build in development mode
uv sync --extra dev

# The package is now available in your virtual environment
uv run python
```

```python
>>> import lingua
>>> lingua.chat_completions_messages_to_lingua([{'role': 'user', 'content': 'Hello'}])
```

### Building wheels

```bash
cd bindings/python

# Build release wheel
uv run maturin build --features python --release

# Wheel will be in ../../target/wheels/
```

## API

### Message conversions

```python
from lingua import (
    # Chat Completions API conversions
    chat_completions_messages_to_lingua,
    lingua_to_chat_completions_messages,

    # Responses API conversions
    responses_messages_to_lingua,
    lingua_to_responses_messages,

    # Anthropic conversions
    anthropic_messages_to_lingua,
    lingua_to_anthropic_messages,
)

# Convert OpenAI Chat Completions messages to universal format
lingua_msgs = chat_completions_messages_to_lingua([
    {'role': 'user', 'content': 'Hello'}
])

# Convert universal format to Anthropic
anthropic_msgs = lingua_to_anthropic_messages(lingua_msgs)

# Convert Anthropic messages to universal format
lingua_msgs = anthropic_messages_to_lingua([
    {'role': 'user', 'content': [{'type': 'text', 'text': 'Hello'}]}
])
```

### Validation

```python
from lingua import (
    validate_openai_request,
    validate_openai_response,
    validate_anthropic_request,
    validate_anthropic_response,
)

import json

# Validate OpenAI request
try:
    request_data = validate_openai_request(json.dumps({
        'model': 'gpt-4',
        'messages': [{'role': 'user', 'content': 'Hello'}]
    }))
    print("Valid!")
except ValueError as e:
    print(f"Invalid: {e}")
```

### Error handling

All conversion functions raise `ConversionError` on failure.
All validation functions raise `ValueError` on invalid input.

```python
from lingua import ConversionError

try:
    result = lingua_to_chat_completions_messages(invalid_messages)
except ConversionError as e:
    print(f"Conversion failed: {e}")
```

## Development

### Running tests

```bash
cd bindings/python
uv run pytest tests/ -v
```

### Type checking

The package includes type stubs in `__init__.pyi` for IDE support.

## Requirements

- Python 3.8+
- Rust toolchain (for building from source)
- uv (recommended) or pip with maturin
