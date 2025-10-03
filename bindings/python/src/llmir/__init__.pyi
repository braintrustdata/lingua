"""Type stubs for LLMIR Python bindings"""

from typing import Any, Dict, TypedDict, Union

class ValidationSuccess(TypedDict):
    ok: bool
    data: Any

class ValidationError(TypedDict):
    ok: bool
    error: Dict[str, str]

ValidationResult = Union[ValidationSuccess, ValidationError]

# OpenAI conversions
def openai_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]: ...
def llmir_to_openai_message(message: Dict[str, Any]) -> Dict[str, Any]: ...
def openai_input_items_to_llmir(items: list[Dict[str, Any]]) -> list[Dict[str, Any]]: ...

# Anthropic conversions
def anthropic_message_to_llmir(message: Dict[str, Any]) -> Dict[str, Any]: ...
def llmir_to_anthropic_message(message: Dict[str, Any]) -> Dict[str, Any]: ...

# Validation functions (raw - may raise exceptions)
def validate_openai_request(json_str: str) -> Any: ...
def validate_openai_response(json_str: str) -> Any: ...
def validate_anthropic_request(json_str: str) -> Any: ...
def validate_anthropic_response(json_str: str) -> Any: ...

# Validation functions (safe - return ValidationResult)
def validate_openai_request_safe(json_str: str) -> ValidationResult: ...
def validate_openai_response_safe(json_str: str) -> ValidationResult: ...
def validate_anthropic_request_safe(json_str: str) -> ValidationResult: ...
def validate_anthropic_response_safe(json_str: str) -> ValidationResult: ...

__all__ = [
    "openai_message_to_llmir",
    "llmir_to_openai_message",
    "openai_input_items_to_llmir",
    "anthropic_message_to_llmir",
    "llmir_to_anthropic_message",
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",
    "validate_openai_request_safe",
    "validate_openai_response_safe",
    "validate_anthropic_request_safe",
    "validate_anthropic_response_safe",
    "ValidationResult",
    "ValidationSuccess",
    "ValidationError",
]
