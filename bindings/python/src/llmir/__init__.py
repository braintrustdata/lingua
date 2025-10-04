"""
LLMIR - Universal message format for LLM APIs

This package provides conversion and validation functions for translating between
different LLM provider formats (OpenAI, Anthropic, etc.) and the universal LLMIR format.

API matches the TypeScript interface but with Pythonic snake_case naming:
- openai_message_to_llmir (TypeScript: openAIMessageToLLMIR)
- validate_openai_request (TypeScript: validateOpenAIRequest)

Note: Python uses exceptions while TypeScript uses Zod-style result objects.
"""

from typing import Any, Dict

# Import the native conversion functions
from llmir._llmir import (
    openai_chat_messages_to_llmir as _openai_chat_messages_to_llmir,
    llmir_to_openai_chat_messages as _llmir_to_openai_chat_messages,
    openai_responses_messages_to_llmir as _openai_responses_messages_to_llmir,
    llmir_to_openai_responses_messages as _llmir_to_openai_responses_messages,
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
# OpenAI Chat Completions API conversions
# ============================================================================

def openai_chat_messages_to_llmir(messages: list) -> list:
    """
    Convert array of OpenAI Chat Completions messages to LLMIR Messages.

    Args:
        messages: List of OpenAI ChatCompletionRequestMessage objects

    Returns:
        List of LLMIR Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _openai_chat_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert OpenAI chat messages to LLMIR: {e}") from e


def llmir_to_openai_chat_messages(messages: list) -> list:
    """
    Convert array of LLMIR Messages to OpenAI Chat Completions messages.

    Args:
        messages: List of LLMIR Message objects

    Returns:
        List of OpenAI ChatCompletionRequestMessage objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_openai_chat_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert LLMIR to OpenAI chat messages: {e}") from e


# ============================================================================
# OpenAI Responses API conversions
# ============================================================================

def openai_responses_messages_to_llmir(messages: list) -> list:
    """
    Convert array of OpenAI Responses API messages to LLMIR Messages.

    Args:
        messages: List of OpenAI InputItem objects

    Returns:
        List of LLMIR Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _openai_responses_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert OpenAI responses messages to LLMIR: {e}") from e


def llmir_to_openai_responses_messages(messages: list) -> list:
    """
    Convert array of LLMIR Messages to OpenAI Responses API messages.

    Args:
        messages: List of LLMIR Message objects

    Returns:
        List of OpenAI InputItem objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_openai_responses_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert LLMIR to OpenAI responses messages: {e}") from e


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_llmir(messages: list) -> list:
    """
    Convert array of Anthropic messages to LLMIR Messages.

    Args:
        messages: List of Anthropic message objects

    Returns:
        List of LLMIR Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _anthropic_messages_to_llmir(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert Anthropic messages to LLMIR: {e}") from e


def llmir_to_anthropic_messages(messages: list) -> list:
    """
    Convert array of LLMIR Messages to Anthropic messages.

    Args:
        messages: List of LLMIR Message objects

    Returns:
        List of Anthropic message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_anthropic_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to convert LLMIR to Anthropic messages: {e}") from e


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

    # OpenAI Chat Completions API conversions
    "openai_chat_messages_to_llmir",
    "llmir_to_openai_chat_messages",

    # OpenAI Responses API conversions
    "openai_responses_messages_to_llmir",
    "llmir_to_openai_responses_messages",

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
