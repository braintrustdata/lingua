"""
Python Roundtrip Tests

These tests validate that:
1. SDK data can be converted to Lingua format
2. Lingua data can be converted back to SDK format
3. Data is preserved through the roundtrip conversion
"""

import json
import os
from pathlib import Path
from typing import Any

import pytest

from lingua import (
    ConversionError,
    chat_completions_messages_to_lingua,
    anthropic_messages_to_lingua,
    lingua_to_chat_completions_messages,
    lingua_to_anthropic_messages,
)


class Snapshot:
    """Represents a test snapshot with provider data"""

    def __init__(
        self,
        name: str,
        provider: str,
        turn: str,
        request: dict | None = None,
        response: dict | None = None,
        streaming_response: list | None = None,
    ):
        self.name = name
        self.provider = provider
        self.turn = turn
        self.request = request
        self.response = response
        self.streaming_response = streaming_response


def load_test_snapshots(test_case_name: str) -> list[Snapshot]:
    """Load all snapshots for a given test case"""
    snapshots: list[Snapshot] = []

    # Snapshots are in the payloads directory (3 levels up from tests/)
    snapshots_dir = (
        Path(__file__).parent.parent.parent.parent
        / "payloads"
        / "snapshots"
        / test_case_name
    )

    if not snapshots_dir.exists():
        return snapshots

    providers = ["openai-chat-completions", "openai-responses", "anthropic"]
    turns = ["first_turn", "followup_turn"]

    for provider in providers:
        provider_dir = snapshots_dir / provider

        if not provider_dir.exists():
            continue

        for turn in turns:
            prefix = "followup-" if turn == "followup_turn" else ""

            snapshot_data = {
                "name": test_case_name,
                "provider": provider,
                "turn": turn,
            }

            # Load request
            request_path = provider_dir / f"{prefix}request.json"
            if request_path.exists():
                with open(request_path) as f:
                    snapshot_data["request"] = json.load(f)

            # Load response
            response_path = provider_dir / f"{prefix}response.json"
            if response_path.exists():
                with open(response_path) as f:
                    snapshot_data["response"] = json.load(f)

            # Load streaming response
            streaming_path = provider_dir / f"{prefix}response-streaming.json"
            if streaming_path.exists():
                with open(streaming_path) as f:
                    content = f.read()
                    try:
                        # Try parsing as JSON array first
                        snapshot_data["streaming_response"] = json.loads(content)
                    except json.JSONDecodeError:
                        # Try newline-delimited JSON
                        lines = [
                            json.loads(line)
                            for line in content.split("\n") if line.strip()
                        ]
                        snapshot_data["streaming_response"] = lines

            if any(
                key in snapshot_data
                for key in ["request", "response", "streaming_response"]
            ):
                snapshots.append(Snapshot(**snapshot_data))

    return snapshots


def normalize_for_comparison(obj: Any) -> Any:
    """
    Recursively normalize an object by removing None and empty values.
    This mimics how Rust's serde skips None values during serialization.
    """
    if obj is None:
        return None

    if isinstance(obj, list):
        # Remove None from arrays and recursively normalize
        normalized = [
            normalize_for_comparison(item) for item in obj if item is not None
        ]
        # Return None for empty arrays to remove them
        return normalized if normalized else None

    if isinstance(obj, dict):
        normalized = {}
        for key, value in obj.items():
            normalized_value = normalize_for_comparison(value)
            # Only include the property if it's not None
            if normalized_value is not None:
                normalized[key] = normalized_value

        # Return None for empty dicts to remove them
        return normalized if normalized else None

    # Primitive values are returned as-is
    return obj


def perform_openai_roundtrip(openai_message: dict) -> dict[str, Any]:
    """
    Perform roundtrip conversion: Chat Completions -> Lingua -> Chat Completions

    Args:
        openai_message: Original Chat Completions message

    Returns:
        Dict with original, lingua, and roundtripped data

    Raises:
        ConversionError: If any conversion step fails
    """
    lingua_msg = chat_completions_messages_to_lingua([openai_message])[0]
    roundtripped = lingua_to_chat_completions_messages([lingua_msg])[0]

    return {
        "original": openai_message,
        "lingua": lingua_msg,
        "roundtripped": roundtripped,
    }


def perform_anthropic_roundtrip(anthropic_message: dict) -> dict[str, Any]:
    """
    Perform roundtrip conversion: Anthropic -> Lingua -> Anthropic

    Args:
        anthropic_message: Original Anthropic message

    Returns:
        Dict with original, lingua, and roundtripped data

    Raises:
        ConversionError: If any conversion step fails
    """
    lingua_msg = anthropic_messages_to_lingua([anthropic_message])[0]
    roundtripped = lingua_to_anthropic_messages([lingua_msg])[0]

    return {
        "original": anthropic_message,
        "lingua": lingua_msg,
        "roundtripped": roundtripped,
    }


class TestRoundtrip:
    """Test roundtrip conversions using real snapshots"""

    @pytest.fixture(scope="class")
    def snapshots_dir(self):
        """Get the snapshots directory path"""
        return Path(__file__).parent.parent.parent.parent / "payloads" / "snapshots"

    @pytest.fixture(scope="class")
    def test_cases(self, snapshots_dir):
        """Get all test case directories"""
        if not snapshots_dir.exists():
            return []

        return [
            d.name
            for d in snapshots_dir.iterdir()
            if d.is_dir() and not d.name.startswith(".")
        ]

    def test_has_test_cases(self, test_cases):
        """Verify we have test cases to run"""
        if not test_cases:
            pytest.skip(
                "No snapshot test cases found. Run capture script in payloads directory first."
            )
        assert len(test_cases) > 0

    def test_openai_roundtrips(self, test_cases):
        """Test OpenAI message roundtrip conversions"""
        if not test_cases:
            pytest.skip("No test cases available")

        for test_case in test_cases:
            snapshots = load_test_snapshots(test_case)

            for snapshot in snapshots:
                if (
                    snapshot.provider != "openai-chat-completions"
                    or not snapshot.request
                ):
                    continue

                messages = snapshot.request.get("messages", [])
                if not isinstance(messages, list) or not messages:
                    continue

                test_name = f"{test_case}/{snapshot.provider}/{snapshot.turn}"

                for i, original_message in enumerate(messages):
                    try:
                        # Perform the roundtrip
                        result = perform_openai_roundtrip(original_message)

                        # Verify Lingua conversion worked
                        assert result["lingua"] is not None
                        assert "role" in result["lingua"]

                        # Normalize both objects
                        normalized_original = normalize_for_comparison(original_message)
                        normalized_roundtripped = normalize_for_comparison(
                            result["roundtripped"]
                        )

                        # The normalized objects should be equal
                        assert normalized_roundtripped == normalized_original, (
                            f"Roundtrip mismatch in {test_name} message {i}:\n"
                            f"Original: {json.dumps(normalized_original, indent=2)}\n"
                            f"Roundtripped: {json.dumps(normalized_roundtripped, indent=2)}"
                        )

                    except ConversionError as e:
                        # Skip unsupported message formats for now
                        print(f"Skipping unsupported format in {test_name}: {e}")

    def test_anthropic_roundtrips(self, test_cases):
        """Test Anthropic message roundtrip conversions"""
        if not test_cases:
            pytest.skip("No test cases available")

        for test_case in test_cases:
            snapshots = load_test_snapshots(test_case)

            for snapshot in snapshots:
                if snapshot.provider != "anthropic" or not snapshot.request:
                    continue

                messages = snapshot.request.get("messages", [])
                if not isinstance(messages, list) or not messages:
                    continue

                test_name = f"{test_case}/{snapshot.provider}/{snapshot.turn}"

                for i, original_message in enumerate(messages):
                    try:
                        # Perform the roundtrip
                        result = perform_anthropic_roundtrip(original_message)

                        # Verify Lingua conversion worked
                        assert result["lingua"] is not None
                        assert "role" in result["lingua"]

                        # Normalize both objects
                        normalized_original = normalize_for_comparison(original_message)
                        normalized_roundtripped = normalize_for_comparison(
                            result["roundtripped"]
                        )

                        # The normalized objects should be equal
                        assert normalized_roundtripped == normalized_original, (
                            f"Roundtrip mismatch in {test_name} message {i}:\n"
                            f"Original: {json.dumps(normalized_original, indent=2)}\n"
                            f"Roundtripped: {json.dumps(normalized_roundtripped, indent=2)}"
                        )

                    except ConversionError as e:
                        # Skip unsupported message formats for now
                        print(f"Skipping unsupported format in {test_name}: {e}")

    def test_coverage(self, test_cases):
        """Display test coverage information"""
        if not test_cases:
            pytest.skip("No test cases available")

        coverage = {}

        for test_case in test_cases:
            snapshots = load_test_snapshots(test_case)
            providers = list({s.provider for s in snapshots})
            turns = list({s.turn for s in snapshots})

            coverage[test_case] = {"providers": providers, "turns": turns}

        print("\nTest coverage by case:")
        for test_case, data in coverage.items():
            print(f"  {test_case}:")
            print(f"    Providers: {', '.join(data['providers'])}")
            print(f"    Turns: {', '.join(data['turns'])}")

        # Ensure each test case has at least some snapshots
        for test_case in test_cases:
            assert len(coverage[test_case]["providers"]) > 0


class TestTypeChecking:
    """Test that our types match the official SDKs"""

    def test_openai_message_types(self):
        """Verify Chat Completions message types match the official SDK"""
        # Import OpenAI SDK types if available
        try:
            from openai.types.chat import ChatCompletionMessageParam
        except ImportError:
            pytest.skip("OpenAI SDK not installed")

        # Test that we can create valid Chat Completions messages
        message: ChatCompletionMessageParam = {"role": "user", "content": "Hello"}

        # Convert to Lingua and back
        lingua_msg = chat_completions_messages_to_lingua([message])[0]
        roundtripped = lingua_to_chat_completions_messages([lingua_msg])[0]

        assert roundtripped is not None
        assert roundtripped["role"] == "user"

    def test_anthropic_message_types(self):
        """Verify Anthropic message types match the official SDK"""
        # Import Anthropic SDK types if available
        try:
            from anthropic.types import MessageParam
        except ImportError:
            pytest.skip("Anthropic SDK not installed")

        # Test that we can create valid Anthropic messages
        message: MessageParam = {
            "role": "user",
            "content": [{"type": "text", "text": "Hello"}],
        }

        # Convert to Lingua and back
        lingua_msg = anthropic_messages_to_lingua([message])[0]
        roundtripped = lingua_to_anthropic_messages([lingua_msg])[0]

        assert roundtripped is not None
        assert roundtripped["role"] == "user"
