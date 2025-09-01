#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "requests>=2.31.0",
#     "python-dotenv>=1.0.0",
#     "pyyaml>=6.0",
# ]
# ///

"""
Compare thinking/reasoning responses between Anthropic and OpenAI APIs.

This script demonstrates the differences in how each provider handles thinking content:
- Anthropic: Exposes thinking as first-class content blocks
- OpenAI: Only tracks reasoning token usage, no exposed thinking content

Usage:
    export ANTHROPIC_API_KEY=your_key_here
    export OPENAI_API_KEY=your_key_here
    ./scripts/compare_thinking_responses.py
"""

import json
import os
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, Any

import requests
import yaml
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

ANTHROPIC_API_KEY = os.getenv("ANTHROPIC_API_KEY")
OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")

def make_anthropic_request() -> Dict[str, Any]:
    """Make a request to Anthropic Messages API with thinking enabled."""
    if not ANTHROPIC_API_KEY:
        return {"error": "ANTHROPIC_API_KEY not found in environment"}
    
    headers = {
        "Content-Type": "application/json",
        "x-api-key": ANTHROPIC_API_KEY,
        "anthropic-version": "2023-06-01"
    }
    
    payload = {
        "model": "claude-opus-4-1-20250805",
        "max_tokens": 2000,
        "thinking": {
            "type": "enabled",
            "budget_tokens": 1024
        },
        "messages": [
            {
                "role": "user",
                "content": "Think through this step by step: What is 15 × 23? Show your reasoning process."
            }
        ]
    }
    
    try:
        response = requests.post(
            "https://api.anthropic.com/v1/messages",
            headers=headers,
            json=payload,
            timeout=30
        )
        return response.json()
    except Exception as e:
        return {"error": f"Anthropic request failed: {str(e)}"}

def make_openai_request() -> Dict[str, Any]:
    """Make a request to OpenAI Responses API with reasoning model."""
    if not OPENAI_API_KEY:
        return {"error": "OPENAI_API_KEY not found in environment"}
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {OPENAI_API_KEY}"
    }
    
    payload = {
        "model": "gpt-5",  # OpenAI's latest model
        "input": [
            {
                "role": "user",
                "content": "Think through this step by step: What is 15 × 23? Show your reasoning process."
            }
        ],
        "max_output_tokens": 1000
    }
    
    try:
        response = requests.post(
            "https://api.openai.com/v1/responses",
            headers=headers,
            json=payload,
            timeout=30
        )
        if response.status_code == 200:
            return response.json()
        else:
            return {"error": f"HTTP {response.status_code}: {response.text}"}
    except Exception as e:
        return {"error": f"OpenAI request failed: {str(e)}"}

def print_anthropic_response(response: Dict[str, Any]) -> None:
    """Print Anthropic response as YAML."""
    print("=" * 60)
    print("🤖 ANTHROPIC RESPONSE (YAML)")
    print("=" * 60)
    
    if "error" in response:
        print(f"❌ Error: {response['error']}")
        return
    
    print(yaml.dump(response, default_flow_style=False, sort_keys=False, width=120))
    print()

def print_openai_response(response: Dict[str, Any]) -> None:
    """Print OpenAI Responses API response as YAML."""
    print("=" * 60)
    print("🤖 OPENAI RESPONSE (YAML)")
    print("=" * 60)
    
    if not response or (response.get("error") is not None):
        error_msg = response.get("error") if response else "No response"
        print(f"❌ Error: {error_msg}")
        return
    
    print(yaml.dump(response, default_flow_style=False, sort_keys=False, width=120))
    print()

def main():
    """Compare thinking responses from both providers."""
    
    # Check API keys
    missing_keys = []
    if not ANTHROPIC_API_KEY:
        missing_keys.append("ANTHROPIC_API_KEY")
    if not OPENAI_API_KEY:
        missing_keys.append("OPENAI_API_KEY")
    
    if missing_keys:
        print("❌ Missing API keys:")
        for key in missing_keys:
            print(f"   - {key}")
        print("\nSet your API keys as environment variables and try again.")
        sys.exit(1)
    
    # Make requests in parallel
    
    responses = {}
    with ThreadPoolExecutor(max_workers=2) as executor:
        # Submit both requests
        future_to_provider = {
            executor.submit(make_anthropic_request): "anthropic",
            executor.submit(make_openai_request): "openai"
        }
        
        # Collect results as they complete
        for future in as_completed(future_to_provider):
            provider = future_to_provider[future]
            try:
                result = future.result()
                responses[provider] = result
            except Exception as e:
                responses[provider] = {"error": f"{provider} request failed: {str(e)}"}
    
    anthropic_response = responses.get("anthropic", {"error": "No anthropic response"})
    openai_response = responses.get("openai", {"error": "No openai response"})
    
    
    # Print responses
    print_anthropic_response(anthropic_response)
    print_openai_response(openai_response)
    

if __name__ == "__main__":
    main()