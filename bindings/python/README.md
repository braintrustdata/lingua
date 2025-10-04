# LLMIR Python Bindings

Python bindings for LLMIR (LLM Intermediate Representation) using PyO3.

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
>>> import llmir
>>> llmir.openai_message_to_llmir({'role': 'user', 'content': 'Hello'})
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
from llmir import (
    # OpenAI conversions
    openai_message_to_llmir,
    llmir_to_openai_message,
    openai_input_items_to_llmir,
    llmir_to_openai_input_items,

    # Anthropic conversions
    anthropic_message_to_llmir,
    llmir_to_anthropic_message,
    llmir_to_anthropic_messages,
)

# Convert OpenAI message to universal format
llmir_msg = openai_message_to_llmir({
    'role': 'user',
    'content': 'Hello'
})

# Convert universal format to Anthropic
anthropic_msg = llmir_to_anthropic_message(llmir_msg)

# Convert lists
openai_items = [...]
llmir_msgs = openai_input_items_to_llmir(openai_items)
anthropic_msgs = llmir_to_anthropic_messages(llmir_msgs)
```

### Validation

```python
from llmir import (
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
from llmir import ConversionError

try:
    result = llmir_to_openai_message(invalid_message)
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
