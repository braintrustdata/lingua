#!/usr/bin/env python3
"""
Lingua Python Examples

Demonstrates the ergonomics of using Lingua's universal types for LLM requests.
"""

from typing import TYPE_CHECKING

# For type checking, we can use the TypedDict definitions
if TYPE_CHECKING:
    from lingua import (
        UniversalParams,
        UniversalRequest,
        UniversalTool,
        ReasoningConfig,
        ToolChoiceConfig,
        ResponseFormatConfig,
        JsonSchemaConfig,
        Message,
    )


def example_typed_params():
    """
    Example: Creating a request with typed parameters.

    This demonstrates how to use TypedDict hints for IDE support
    while passing plain dicts to Lingua functions.
    """
    print("\n📋 Example: Creating typed parameters")

    # Define tools with type hints for IDE support
    tools: list["UniversalTool"] = [
        {
            "name": "get_weather",
            "description": "Get the current weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {"type": "string", "description": "City name"},
                    "units": {"type": "string", "enum": ["celsius", "fahrenheit"]},
                },
                "required": ["location"],
            },
            "kind": "function",
        }
    ]

    # Create params with type hints
    params: "UniversalParams" = {
        "temperature": 0.7,
        "max_tokens": 1000,
        "tools": tools,
        "tool_choice": {"mode": "auto"},
        "response_format": {
            "format_type": "json_schema",
            "json_schema": {
                "name": "weather_response",
                "schema": {
                    "type": "object",
                    "properties": {
                        "temperature": {"type": "number"},
                        "conditions": {"type": "string"},
                    },
                },
                "strict": True,
            },
        },
        "reasoning": {
            "enabled": True,
            "budget_tokens": 2048,
            "summary": "auto",
        },
        "metadata": {"user_id": "example-user"},
    }

    # Create the full request
    request: "UniversalRequest" = {
        "model": "gpt-5-mini",
        "messages": [
            {"role": "user", "content": "What's the weather in San Francisco?"}
        ],
        "params": params,
    }

    print(f"   Model: {request['model']}")
    print(f"   Tools: {len(request['params'].get('tools', []))} tool(s)")
    print(f"   Reasoning enabled: {request['params'].get('reasoning', {}).get('enabled')}")
    print(f"   Response format: {request['params'].get('response_format', {}).get('format_type')}")

    return request


def example_provider_conversion():
    """
    Example: Converting messages between providers.

    This requires the lingua package to be installed with:
        pip install lingua
    """
    print("\n🔄 Example: Provider conversion")

    try:
        from lingua import (
            chat_completions_messages_to_lingua,
            lingua_to_anthropic_messages,
        )

        # OpenAI Chat Completions format
        openai_messages = [
            {"role": "user", "content": "Hello, how are you?"},
            {"role": "assistant", "content": "I'm doing well, thanks!"},
        ]

        # Convert to Lingua format
        lingua_messages = chat_completions_messages_to_lingua(openai_messages)
        print(f"   Converted {len(openai_messages)} messages to Lingua format")

        # Convert to Anthropic format
        anthropic_messages = lingua_to_anthropic_messages(lingua_messages)
        print(f"   Converted to Anthropic format: {len(anthropic_messages)} messages")

    except ImportError:
        print("   ⚠️  Skipping - lingua package not installed")
        print("   Install with: pip install lingua")


def main():
    print("═" * 60)
    print("  🌍 Lingua: Universal Types for LLM Requests")
    print("═" * 60)

    # This example works without the lingua package installed
    # (just demonstrates type hints)
    example_typed_params()

    # This example requires the lingua package
    example_provider_conversion()

    print("\n" + "═" * 60)
    print("  ✨ One format. Any model. Type-safe. ✨")
    print("═" * 60)


if __name__ == "__main__":
    main()
