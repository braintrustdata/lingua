"""
Lingua - Universal message format for LLM APIs

This package provides conversion and validation functions for translating between
different LLM provider formats (OpenAI, Anthropic, etc.) and the universal Lingua format.

API matches the TypeScript interface but with Pythonic snake_case naming:
- openai_message_to_lingua (TypeScript: openAIMessageToLingua)
- validate_openai_request (TypeScript: validateOpenAIRequest)

Note: Python uses exceptions while TypeScript uses Zod-style result objects.
"""

from typing import Any, Dict

# Import the native conversion functions
from lingua._lingua import (
    chat_completions_messages_to_llmir as _chat_completions_messages_to_llmir,
    llmir_to_chat_completions_messages as _llmir_to_chat_completions_messages,
    responses_messages_to_llmir as _responses_messages_to_llmir,
    llmir_to_responses_messages as _llmir_to_responses_messages,
    anthropic_messages_to_llmir as _anthropic_messages_to_llmir,
    llmir_to_anthropic_messages as _llmir_to_anthropic_messages,
    validate_openai_request as _validate_openai_request,
    validate_openai_response as _validate_openai_response,
    validate_anthropic_request as _validate_anthropic_request,
    validate_anthropic_response as _validate_anthropic_response,
)


# ============================================================================
# Error types
# ============================================================================

class ConversionError(Exception):
    """Error during format conversion"""
    pass


# ============================================================================
# Chat Completions API conversions
# ============================================================================

def chat_completions_messages_to_llmir(messages: list) -> list:
    """
    Convert array of Chat Completions messages to Lingua Messages.

    Args:
        messages: List of ChatCompletionRequestMessage objects

    Returns:
        List of Lingua Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _chat_completions_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert chat completions messages to Lingua: {e}") from e


def llmir_to_chat_completions_messages(messages: list) -> list:
    """
    Convert array of Lingua Messages to Chat Completions messages.

    Args:
        messages: List of Lingua Message objects

    Returns:
        List of ChatCompletionRequestMessage objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_chat_completions_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert Lingua to chat completions messages: {e}") from e


# ============================================================================
# Responses API conversions
# ============================================================================

def responses_messages_to_llmir(messages: list) -> list:
    """
    Convert array of Responses API messages to Lingua Messages.

    Args:
        messages: List of InputItem objects

    Returns:
        List of Lingua Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _responses_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert responses messages to Lingua: {e}") from e


def llmir_to_responses_messages(messages: list) -> list:
    """
    Convert array of Lingua Messages to Responses API messages.

    Args:
        messages: List of Lingua Message objects

    Returns:
        List of InputItem objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_responses_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert Lingua to responses messages: {e}") from e


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_llmir(messages: list) -> list:
    """
    Convert array of Anthropic messages to Lingua Messages.

    Args:
        messages: List of Anthropic message objects

    Returns:
        List of Lingua Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _anthropic_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert Anthropic messages to Lingua: {e}") from e


def llmir_to_anthropic_messages(messages: list) -> list:
    """
    Convert array of Lingua Messages to Anthropic messages.

    Args:
        messages: List of Lingua Message objects

    Returns:
        List of Anthropic message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_anthropic_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert Lingua to Anthropic messages: {e}") from e


# ============================================================================
# OpenAI validation
# ============================================================================

def validate_openai_request(json_str: str) -> Any:
    """
    Validate a JSON string as an OpenAI request.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated OpenAI request data

    Raises:
        ValueError: If validation fails
    """
    return _validate_openai_request(json_str)


def validate_openai_response(json_str: str) -> Any:
    """
    Validate a JSON string as an OpenAI response.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated OpenAI response data

    Raises:
        ValueError: If validation fails
    """
    return _validate_openai_response(json_str)


# ============================================================================
# Anthropic validation
# ============================================================================

def validate_anthropic_request(json_str: str) -> Any:
    """
    Validate a JSON string as an Anthropic request.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Anthropic request data

    Raises:
        ValueError: If validation fails
    """
    return _validate_anthropic_request(json_str)


def validate_anthropic_response(json_str: str) -> Any:
    """
    Validate a JSON string as an Anthropic response.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Anthropic response data

    Raises:
        ValueError: If validation fails
    """
    return _validate_anthropic_response(json_str)


# ============================================================================
# Exports
# ============================================================================

__all__ = [
    # Error handling
    "ConversionError",

    # Chat Completions API conversions
    "chat_completions_messages_to_llmir",
    "llmir_to_chat_completions_messages",

    # Responses API conversions
    "responses_messages_to_llmir",
    "llmir_to_responses_messages",

    # Anthropic conversions
    "anthropic_messages_to_llmir",
    "llmir_to_anthropic_messages",

    # OpenAI validation
    "validate_openai_request",
    "validate_openai_response",

    # Anthropic validation
    "validate_anthropic_request",
    "validate_anthropic_response",
]
