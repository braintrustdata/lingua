"""
Lingua - Universal message format for LLM APIs

This package provides conversion and validation functions for translating between
different LLM provider formats (OpenAI, Anthropic, etc.) and the universal Lingua format.

API matches the TypeScript interface but with Pythonic snake_case naming:
- openai_message_to_lingua (TypeScript: openAIMessageToLingua)
- validate_openai_request (TypeScript: validateOpenAIRequest)

Note: Python uses exceptions while TypeScript uses Zod-style result objects.
"""

from typing import Any

# Import the native conversion functions
from lingua._lingua import (
    chat_completions_messages_to_lingua as _chat_completions_messages_to_lingua,
    lingua_to_chat_completions_messages as _lingua_to_chat_completions_messages,
    responses_messages_to_lingua as _responses_messages_to_lingua,
    lingua_to_responses_messages as _lingua_to_responses_messages,
    anthropic_messages_to_lingua as _anthropic_messages_to_lingua,
    lingua_to_anthropic_messages as _lingua_to_anthropic_messages,
    google_contents_to_lingua as _google_contents_to_lingua,
    lingua_to_google_contents as _lingua_to_google_contents,
    validate_chat_completions_request as _validate_chat_completions_request,
    validate_chat_completions_response as _validate_chat_completions_response,
    validate_responses_request as _validate_responses_request,
    validate_responses_response as _validate_responses_response,
    validate_openai_request as _validate_openai_request,
    validate_openai_response as _validate_openai_response,
    validate_anthropic_request as _validate_anthropic_request,
    validate_anthropic_response as _validate_anthropic_response,
    # Processing functions
    deduplicate_messages as _deduplicate_messages,
    import_messages_from_spans as _import_messages_from_spans,
    import_and_deduplicate_messages as _import_and_deduplicate_messages,
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

def chat_completions_messages_to_lingua(messages: list) -> list:
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
        return _chat_completions_messages_to_lingua(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert chat completions messages to Lingua: {e}"
        ) from e


def lingua_to_chat_completions_messages(messages: list) -> list:
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
        return _lingua_to_chat_completions_messages(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Lingua to chat completions messages: {e}"
        ) from e


# ============================================================================
# Responses API conversions
# ============================================================================

def responses_messages_to_lingua(messages: list) -> list:
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
        return _responses_messages_to_lingua(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert responses messages to Lingua: {e}"
        ) from e


def lingua_to_responses_messages(messages: list) -> list:
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
        return _lingua_to_responses_messages(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Lingua to responses messages: {e}"
        ) from e


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_lingua(messages: list) -> list:
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
        return _anthropic_messages_to_lingua(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Anthropic messages to Lingua: {e}"
        ) from e


def lingua_to_anthropic_messages(messages: list) -> list:
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
        return _lingua_to_anthropic_messages(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Lingua to Anthropic messages: {e}"
        ) from e


# ============================================================================
# Google conversions
# ============================================================================

def google_contents_to_lingua(contents: list) -> list:
    """
    Convert array of Google Content items to Lingua Messages.

    Args:
        contents: List of Google Content items

    Returns:
        List of Lingua Message objects

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _google_contents_to_lingua(contents)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Google contents to Lingua: {e}"
        ) from e


def lingua_to_google_contents(messages: list) -> list:
    """
    Convert array of Lingua Messages to Google Content items.

    Args:
        messages: List of Lingua Message objects

    Returns:
        List of Google Content items

    Raises:
        ConversionError: If conversion fails
    """
    try:
        return _lingua_to_google_contents(messages)
    except Exception as e:
        raise ConversionError(
            f"Failed to convert Lingua to Google contents: {e}"
        ) from e


# ============================================================================
# Chat Completions validation
# ============================================================================

def validate_chat_completions_request(json_str: str) -> Any:
    """
    Validate a JSON string as a Chat Completions request.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Chat Completions request data

    Raises:
        ValueError: If validation fails
    """
    return _validate_chat_completions_request(json_str)


def validate_chat_completions_response(json_str: str) -> Any:
    """
    Validate a JSON string as a Chat Completions response.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Chat Completions response data

    Raises:
        ValueError: If validation fails
    """
    return _validate_chat_completions_response(json_str)


# ============================================================================
# Responses API validation
# ============================================================================

def validate_responses_request(json_str: str) -> Any:
    """
    Validate a JSON string as a Responses API request.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Responses API request data

    Raises:
        ValueError: If validation fails
    """
    return _validate_responses_request(json_str)


def validate_responses_response(json_str: str) -> Any:
    """
    Validate a JSON string as a Responses API response.

    Args:
        json_str: JSON string to validate

    Returns:
        Validated Responses API response data

    Raises:
        ValueError: If validation fails
    """
    return _validate_responses_response(json_str)


# ============================================================================
# OpenAI validation (deprecated)
# ============================================================================

def validate_openai_request(json_str: str) -> Any:
    """
    Validate a JSON string as an OpenAI request.

    .. deprecated::
        Use :func:`validate_chat_completions_request` instead

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

    .. deprecated::
        Use :func:`validate_chat_completions_response` instead

    Args:
        json_str: JSON string to validate

    Returns:
        Validated OpenAI response data

    Raises:
        ValueError: If validation fails
    """
    return _validate_openai_response(json_str)


# ============================================================================
# Processing functions
# ============================================================================

def deduplicate_messages(messages: list) -> list:
    """
    Deduplicate messages based on role and content.

    Removes consecutive duplicate messages that have the same role and content,
    keeping the first occurrence.

    Args:
        messages: List of Lingua Message objects

    Returns:
        List of deduplicated Lingua Message objects

    Raises:
        ConversionError: If processing fails
    """
    try:
        return _deduplicate_messages(messages)
    except Exception as e:
        raise ConversionError(f"Failed to deduplicate messages: {e}") from e


def import_messages_from_spans(spans: list) -> list:
    """
    Import messages from a list of spans.

    Processes spans and extracts messages from their input/output fields,
    attempting to convert them from various provider formats (OpenAI Chat Completions,
    OpenAI Responses API, Anthropic) to the Lingua universal format.

    Each span should be a dict with optional 'input' and 'output' fields.

    Args:
        spans: List of span dicts with optional 'input' and 'output' fields

    Returns:
        List of Lingua Message objects extracted from the spans

    Raises:
        ConversionError: If processing fails
    """
    try:
        return _import_messages_from_spans(spans)
    except Exception as e:
        raise ConversionError(f"Failed to import messages from spans: {e}") from e


def import_and_deduplicate_messages(spans: list) -> list:
    """
    Import and deduplicate messages from spans in a single operation.

    Combines import_messages_from_spans and deduplicate_messages for convenience.

    Args:
        spans: List of span dicts with optional 'input' and 'output' fields

    Returns:
        List of deduplicated Lingua Message objects

    Raises:
        ConversionError: If processing fails
    """
    try:
        return _import_and_deduplicate_messages(spans)
    except Exception as e:
        raise ConversionError(f"Failed to import and deduplicate messages: {e}") from e


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
    "chat_completions_messages_to_lingua",
    "lingua_to_chat_completions_messages",
    # Responses API conversions
    "responses_messages_to_lingua",
    "lingua_to_responses_messages",
    # Anthropic conversions
    "anthropic_messages_to_lingua",
    "lingua_to_anthropic_messages",
    # Google conversions
    "google_contents_to_lingua",
    "lingua_to_google_contents",
    # Processing functions
    "deduplicate_messages",
    "import_messages_from_spans",
    "import_and_deduplicate_messages",
    # Chat Completions validation
    "validate_chat_completions_request",
    "validate_chat_completions_response",
    # Responses API validation
    "validate_responses_request",
    "validate_responses_response",
    # OpenAI validation (deprecated - use Chat Completions or Responses instead)
    "validate_openai_request",
    "validate_openai_response",
    # Anthropic validation
    "validate_anthropic_request",
    "validate_anthropic_response",
]
