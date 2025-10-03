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
    openai_message_to_llmir as _openai_message_to_llmir,
    llmir_to_openai_message as _llmir_to_openai_message,
    openai_input_items_to_llmir as _openai_input_items_to_llmir,
    anthropic_message_to_llmir as _anthropic_message_to_llmir,
    llmir_to_anthropic_message as _llmir_to_anthropic_message,
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
# OpenAI conversions
# ============================================================================

def openai_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert OpenAI ChatCompletionRequestMessage to LLMIR Message.

    Args:
        message: OpenAI message object

    Returns:
        LLMIR Message object

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _openai_message_to_llmir(message)
    except Exception as e:
        raise ConversionError(f"Failed to convert OpenAI message to LLMIR: {e}") from e


def llmir_to_openai_message(message: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert LLMIR Message to OpenAI ChatCompletionRequestMessage.

    Args:
        message: LLMIR Message object

    Returns:
        OpenAI message object

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_openai_message(message)
    except Exception as e:
        raise ConversionError(f"Failed to convert LLMIR to OpenAI message: {e}") from e


def openai_input_items_to_llmir(items: list) -> list:
    """
    Convert array of OpenAI InputItems to LLMIR Messages.

    Args:
        items: List of OpenAI InputItem objects

    Returns:
        List of LLMIR Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _openai_input_items_to_llmir(items)
    except Exception as e:
        raise ConversionError(f"Failed to convert OpenAI input items to LLMIR: {e}") from e


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert Anthropic InputMessage to LLMIR Message.

    Args:
        message: Anthropic message object

    Returns:
        LLMIR Message object

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _anthropic_message_to_llmir(message)
    except Exception as e:
        raise ConversionError(f"Failed to convert Anthropic message to LLMIR: {e}") from e


def llmir_to_anthropic_message(message: Dict[str, Any]) -> Dict[str, Any]:
    """
    Convert LLMIR Message to Anthropic InputMessage.

    Args:
        message: LLMIR Message object

    Returns:
        Anthropic message object

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _llmir_to_anthropic_message(message)
    except Exception as e:
        raise ConversionError(f"Failed to convert LLMIR to Anthropic message: {e}") from e


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

    # OpenAI conversions
    "openai_message_to_llmir",
    "llmir_to_openai_message",
    "openai_input_items_to_llmir",

    # Anthropic conversions
    "anthropic_message_to_llmir",
    "llmir_to_anthropic_message",

    # OpenAI validation
    "validate_openai_request",
    "validate_openai_response",

    # Anthropic validation
    "validate_anthropic_request",
    "validate_anthropic_response",
]
