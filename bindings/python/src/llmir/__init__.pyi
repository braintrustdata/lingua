"""Type stubs for LLMIR Python bindings"""

from typing import Any, Dict

# ============================================================================
# Error types
# ============================================================================

class ConversionError(Exception):
    """Error during format conversion"""
    ...

# ============================================================================
# OpenAI Chat Completions API conversions
# ============================================================================

def openai_chat_messages_to_llmir(messages: list) -> list: ...
def llmir_to_openai_chat_messages(messages: list) -> list: ...

# ============================================================================
# OpenAI Responses API conversions
# ============================================================================

def openai_responses_messages_to_llmir(messages: list) -> list: ...
def llmir_to_openai_responses_messages(messages: list) -> list: ...

# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_llmir(messages: list) -> list: ...
def llmir_to_anthropic_messages(messages: list) -> list: ...

# ============================================================================
# OpenAI validation
# ============================================================================

def validate_openai_request(json_str: str) -> Any: ...
def validate_openai_response(json_str: str) -> Any: ...

# ============================================================================
# Anthropic validation
# ============================================================================

def validate_anthropic_request(json_str: str) -> Any: ...
def validate_anthropic_response(json_str: str) -> Any: ...

# ============================================================================
# Exports
# ============================================================================

__all__ = [
    "ConversionError",
    "openai_chat_messages_to_llmir",
    "llmir_to_openai_chat_messages",
    "openai_responses_messages_to_llmir",
    "llmir_to_openai_responses_messages",
    "anthropic_messages_to_llmir",
    "llmir_to_anthropic_messages",
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",
]
