"""Type stubs for Lingua Python bindings"""

from typing import Any, Dict, List, Literal, Optional
from typing_extensions import TypedDict


# ============================================================================
# Provider format
# ============================================================================

ProviderFormat = Literal["openai", "anthropic", "google", "mistral", "converse", "responses", "unknown"]


# ============================================================================
# Enums as Literal types
# ============================================================================

SummaryMode = Literal["none", "auto", "detailed"]
ToolChoiceMode = Literal["auto", "none", "required", "tool"]
ResponseFormatType = Literal["text", "json_object", "json_schema"]
ReasoningEffort = Literal["low", "medium", "high"]
ReasoningCanonical = Literal["effort", "budget_tokens"]


# ============================================================================
# Config types
# ============================================================================

class ReasoningConfig(TypedDict, total=False):
    """Configuration for extended thinking / reasoning capabilities."""
    enabled: Optional[bool]
    effort: Optional[ReasoningEffort]
    budget_tokens: Optional[int]
    canonical: Optional[ReasoningCanonical]
    summary: Optional[SummaryMode]


class ToolChoiceConfig(TypedDict, total=False):
    """Tool selection strategy configuration."""
    mode: Optional[ToolChoiceMode]
    tool_name: Optional[str]
    disable_parallel: Optional[bool]


class JsonSchemaConfig(TypedDict, total=False):
    """JSON schema configuration for structured output."""
    name: str
    schema: Dict[str, Any]
    strict: Optional[bool]
    description: Optional[str]


class ResponseFormatConfig(TypedDict, total=False):
    """Response format configuration for structured output."""
    format_type: Optional[ResponseFormatType]
    json_schema: Optional[JsonSchemaConfig]


class UniversalTool(TypedDict, total=False):
    """A tool definition in universal format."""
    name: str
    description: Optional[str]
    parameters: Optional[Dict[str, Any]]
    strict: Optional[bool]
    kind: Literal["function", "builtin"]
    # For builtin tools:
    provider: Optional[str]
    builtin_type: Optional[str]
    config: Optional[Dict[str, Any]]


class UniversalParams(TypedDict, total=False):
    """Common request parameters across providers."""
    temperature: Optional[float]
    top_p: Optional[float]
    top_k: Optional[int]
    seed: Optional[int]
    presence_penalty: Optional[float]
    frequency_penalty: Optional[float]
    max_tokens: Optional[int]
    stop: Optional[List[str]]
    logprobs: Optional[bool]
    top_logprobs: Optional[int]
    tools: Optional[List[UniversalTool]]
    tool_choice: Optional[ToolChoiceConfig]
    parallel_tool_calls: Optional[bool]
    response_format: Optional[ResponseFormatConfig]
    reasoning: Optional[ReasoningConfig]
    metadata: Optional[Dict[str, Any]]
    store: Optional[bool]
    service_tier: Optional[str]
    stream: Optional[bool]


class UniversalRequest(TypedDict, total=False):
    """Universal request envelope for LLM API calls."""
    model: Optional[str]
    messages: List[Any]
    params: UniversalParams


# ============================================================================
# Message types
# ============================================================================

class TextContentPart(TypedDict):
    """Text content part."""
    type: Literal["text"]
    text: str


class Message(TypedDict, total=False):
    """A message in universal format."""
    role: Literal["system", "user", "assistant", "tool"]
    content: Any


# ============================================================================
# Error types
# ============================================================================

class ConversionError(Exception):
    """Error during format conversion"""
    ...


# ============================================================================
# Chat Completions API conversions
# ============================================================================

def chat_completions_messages_to_lingua(messages: List[Any]) -> List[Message]:
    """Convert array of Chat Completions messages to Lingua Messages."""
    ...


def lingua_to_chat_completions_messages(messages: List[Message]) -> List[Any]:
    """Convert array of Lingua Messages to Chat Completions messages."""
    ...


# ============================================================================
# Responses API conversions
# ============================================================================

def responses_messages_to_lingua(messages: List[Any]) -> List[Message]:
    """Convert array of Responses API messages to Lingua Messages."""
    ...


def lingua_to_responses_messages(messages: List[Message]) -> List[Any]:
    """Convert array of Lingua Messages to Responses API messages."""
    ...


# ============================================================================
# Anthropic conversions
# ============================================================================

def anthropic_messages_to_lingua(messages: List[Any]) -> List[Message]:
    """Convert array of Anthropic messages to Lingua Messages."""
    ...


def lingua_to_anthropic_messages(messages: List[Message]) -> List[Any]:
    """Convert array of Lingua Messages to Anthropic messages."""
    ...


# ============================================================================
# Processing functions
# ============================================================================

def deduplicate_messages(messages: List[Message]) -> List[Message]:
    """Deduplicate messages based on role and content."""
    ...


def import_messages_from_spans(spans: List[Any]) -> List[Message]:
    """Import messages from spans."""
    ...


def import_and_deduplicate_messages(spans: List[Any]) -> List[Message]:
    """Import and deduplicate messages from spans in a single operation."""
    ...


# ============================================================================
# Transform functions
# ============================================================================

class TransformPassThroughResult(TypedDict):
    """Result when payload was already valid for target format."""
    pass_through: Literal[True]
    data: Any


class TransformTransformedResult(TypedDict):
    """Result when payload was transformed to target format."""
    transformed: Literal[True]
    data: Any
    source_format: str


TransformResult = TransformPassThroughResult | TransformTransformedResult


def transform_request(
    json: str,
    target_format: ProviderFormat,
    model: Optional[str] = None,
) -> TransformResult:
    """Transform a request payload to the target format.

    Takes a JSON string and target format, auto-detects the source format,
    and transforms to the target format.

    Returns a dict with either:
    - `{ "pass_through": True, "data": ... }` if payload is already valid for target
    - `{ "transformed": True, "data": ..., "source_format": "..." }` if transformed
    """
    ...


def transform_response(json: str, target_format: ProviderFormat) -> TransformResult:
    """Transform a response payload from one format to another.

    Takes a JSON string and target format, auto-detects the source format,
    and transforms to the target format.

    Returns a dict with either:
    - `{ "pass_through": True, "data": ... }` if payload is already valid for target
    - `{ "transformed": True, "data": ..., "source_format": "..." }` if transformed
    """
    ...


def extract_model(json: str) -> Optional[str]:
    """Extract model name from request without full transformation.

    This is a fast path for routing decisions that only need the model name.
    Returns the model string if found, or None if not present.
    """
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
    # Error types
    "ConversionError",
    # Type definitions
    "ProviderFormat",
    "SummaryMode",
    "ToolChoiceMode",
    "ResponseFormatType",
    "ReasoningEffort",
    "ReasoningCanonical",
    "ReasoningConfig",
    "ToolChoiceConfig",
    "JsonSchemaConfig",
    "ResponseFormatConfig",
    "UniversalTool",
    "UniversalParams",
    "UniversalRequest",
    "Message",
    "TextContentPart",
    "TransformResult",
    "TransformPassThroughResult",
    "TransformTransformedResult",
    # Conversion functions
    "chat_completions_messages_to_lingua",
    "lingua_to_chat_completions_messages",
    "responses_messages_to_lingua",
    "lingua_to_responses_messages",
    "anthropic_messages_to_lingua",
    "lingua_to_anthropic_messages",
    # Processing functions
    "deduplicate_messages",
    "import_messages_from_spans",
    "import_and_deduplicate_messages",
    # Transform functions
    "transform_request",
    "transform_response",
    "extract_model",
    # Validation functions
    "validate_openai_request",
    "validate_openai_response",
    "validate_anthropic_request",
    "validate_anthropic_response",
]
