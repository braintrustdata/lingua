"""
LLMIR - Universal message format for LLM APIs

This package provides conversion and validation functions for translating between
different LLM provider formats (OpenAI, Anthropic, etc.) and the universal LLMIR format.
"""

from typing import Any, Dict, TypedDict, Union

# Import the native module
from llmir._llmir import (
    # OpenAI conversions
    openai_message_to_llmir,
    llmir_to_openai_message,
    openai_input_items_to_llmir,

    # Anthropic conversions
    anthropic_message_to_llmir,
    llmir_to_anthropic_message,

    # Validation functions
    validate_openai_request,
    validate_openai_response,
    validate_anthropic_request,
    validate_anthropic_response,
)


class ValidationSuccess(TypedDict):
    """Successful validation result"""
    ok: bool  # Always True
    data: Any


class ValidationError(TypedDict):
    """Failed validation result"""
    ok: bool  # Always False
    error: Dict[str, str]


ValidationResult = Union[ValidationSuccess, ValidationError]


def validate_openai_request_safe(json_str: str) -> ValidationResult:
    """
    Validate an OpenAI request JSON string.

    Args:
        json_str: JSON string to validate

    Returns:
        ValidationResult with ok=True and data, or ok=False and error
    """
    try:
        data = validate_openai_request(json_str)
        return {"ok": True, "data": data}
    except Exception as e:
        return {"ok": False, "error": {"message": str(e)}}


def validate_openai_response_safe(json_str: str) -> ValidationResult:
    """
    Validate an OpenAI response JSON string.

    Args:
        json_str: JSON string to validate

    Returns:
        ValidationResult with ok=True and data, or ok=False and error
    """
    try:
        data = validate_openai_response(json_str)
        return {"ok": True, "data": data}
    except Exception as e:
        return {"ok": False, "error": {"message": str(e)}}


def validate_anthropic_request_safe(json_str: str) -> ValidationResult:
    """
    Validate an Anthropic request JSON string.

    Args:
        json_str: JSON string to validate

    Returns:
        ValidationResult with ok=True and data, or ok=False and error
    """
    try:
        data = validate_anthropic_request(json_str)
        return {"ok": True, "data": data}
    except Exception as e:
        return {"ok": False, "error": {"message": str(e)}}


def validate_anthropic_response_safe(json_str: str) -> ValidationResult:
    """
    Validate an Anthropic response JSON string.

    Args:
        json_str: JSON string to validate

    Returns:
        ValidationResult with ok=True and data, or ok=False and error
    """
    try:
        data = validate_anthropic_response(json_str)
        return {"ok": True, "data": data}
    except Exception as e:
        return {"ok": False, "error": {"message": str(e)}}


__all__ = [
    # OpenAI conversions
    "openai_message_to_llmir",
    "llmir_to_openai_message",
    "openai_input_items_to_llmir",

    # Anthropic conversions
    "anthropic_message_to_llmir",
    "llmir_to_anthropic_message",

    # Validation functions (raw)
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",

    # Validation functions (safe Zod-style)
    "validate_openai_request_safe",
    "validate_openai_response_safe",
    "validate_anthropic_request_safe",
    "validate_anthropic_response_safe",

    # Types
    "ValidationResult",
    "ValidationSuccess",
    "ValidationError",
]
