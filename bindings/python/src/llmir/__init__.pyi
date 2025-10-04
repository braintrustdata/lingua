"""Type stubs for LLMIR Python bindings"""

from typing import Any, Dict

# ============================================================================
# Error types
# ============================================================================

class ConversionError(Exception):
    """Error during format conversion"""
    ...

# ============================================================================
# OpenAI conversions
# ============================================================================

def openai_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]: ...
def llmir_to_openai_message(message: Dict[str, Any]) -> Dict[str, Any]: ...
def openai_input_items_to_llmir(items: list) -> list: ...
def llmir_to_openai_input_items(messages: list) -> list: ...

# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]: ...
def llmir_to_anthropic_message(message: Dict[str, Any]) -> Dict[str, Any]: ...
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
    "openai_message_to_llmir",
    "llmir_to_openai_message",
    "openai_input_items_to_llmir",
    "llmir_to_openai_input_items",
    "anthropic_message_to_llmir",
    "llmir_to_anthropic_message",
    "llmir_to_anthropic_messages",
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",
]
