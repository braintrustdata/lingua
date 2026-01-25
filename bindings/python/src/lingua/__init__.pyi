"""Type stubs for Lingua Python bindings"""

from typing import Any


# ============================================================================
# Error types
# ============================================================================

class ConversionError(Exception):
    """Error during format conversion"""
    ...


# ============================================================================
# Chat Completions API conversions
# ============================================================================

def chat_completions_messages_to_lingua(messages: list) -> list:
    ...


def lingua_to_chat_completions_messages(messages: list) -> list:
    ...


# ============================================================================
# Responses API conversions
# ============================================================================

def responses_messages_to_lingua(messages: list) -> list:
    ...


def lingua_to_responses_messages(messages: list) -> list:
    ...


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_lingua(messages: list) -> list:
    ...


def lingua_to_anthropic_messages(messages: list) -> list:
    ...


# ============================================================================
# Google conversions
# ============================================================================

def google_contents_to_lingua(contents: list) -> list:
    ...


def lingua_to_google_contents(messages: list) -> list:
    ...


# ============================================================================
# Processing functions
# ============================================================================

def deduplicate_messages(messages: list) -> list:
    ...


def import_messages_from_spans(spans: list) -> list:
    ...


def import_and_deduplicate_messages(spans: list) -> list:
    ...


# ============================================================================
# OpenAI validation
# ============================================================================

def validate_openai_request(json_str: str) -> Any:
    ...


def validate_openai_response(json_str: str) -> Any:
    ...


# ============================================================================
# Anthropic validation
# ============================================================================

def validate_anthropic_request(json_str: str) -> Any:
    ...


def validate_anthropic_response(json_str: str) -> Any:
    ...


# ============================================================================
# Exports
# ============================================================================

__all__ = [
    "ConversionError",
    "chat_completions_messages_to_lingua",
    "lingua_to_chat_completions_messages",
    "responses_messages_to_lingua",
    "lingua_to_responses_messages",
    "anthropic_messages_to_lingua",
    "lingua_to_anthropic_messages",
    "google_contents_to_lingua",
    "lingua_to_google_contents",
    "deduplicate_messages",
    "import_messages_from_spans",
    "import_and_deduplicate_messages",
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",
]
