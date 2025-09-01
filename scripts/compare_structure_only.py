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
Compare just the response structures between Anthropic and OpenAI APIs.
Shows only the structure without full content.
"""

import json
import os
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, Any

import requests
import yaml
from dotenv import load_dotenv

load_dotenv()

ANTHROPIC_API_KEY = os.getenv("ANTHROPIC_API_KEY")
OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")

def make_anthropic_request() -> Dict[str, Any]:
    """Make a simple Anthropic request to see structure."""
    if not ANTHROPIC_API_KEY:
        return {"error": "ANTHROPIC_API_KEY not found in environment"}
    
    headers = {
        "Content-Type": "application/json",
        "x-api-key": ANTHROPIC_API_KEY,
        "anthropic-version": "2023-06-01"
    }
    
    payload = {
        "model": "claude-3-5-haiku-20241022",
        "max_tokens": 100,
        "tools": [{"type": "web_search_20250305", "name": "web_search"}],
        "messages": [{"role": "user", "content": "Search for one fact about AI and tell me."}]
    }
    
    try:
        response = requests.post("https://api.anthropic.com/v1/messages", headers=headers, json=payload, timeout=30)
        return response.json() if response.status_code == 200 else {"error": f"HTTP {response.status_code}"}
    except Exception as e:
        return {"error": str(e)}

def make_openai_request() -> Dict[str, Any]:
    """Make a simple OpenAI request to see structure.""" 
    if not OPENAI_API_KEY:
        return {"error": "OPENAI_API_KEY not found in environment"}
    
    headers = {"Content-Type": "application/json", "Authorization": f"Bearer {OPENAI_API_KEY}"}
    
    payload = {
        "model": "gpt-4o-mini",
        "input": [{"role": "user", "content": "Search for one fact about AI and tell me."}],
        "max_output_tokens": 100,
        "tools": [{"type": "web_search_preview"}]
    }
    
    try:
        response = requests.post("https://api.openai.com/v1/responses", headers=headers, json=payload, timeout=30)
        return response.json() if response.status_code == 200 else {"error": f"HTTP {response.status_code}"}
    except Exception as e:
        return {"error": str(e)}

def extract_structure(obj, max_depth=3, current_depth=0):
    """Extract just the structure of a response, not the content."""
    if current_depth >= max_depth:
        return "..."
    
    if isinstance(obj, dict):
        result = {}
        for key, value in obj.items():
            if key in ['encrypted_content', 'content'] and isinstance(value, str) and len(value) > 100:
                result[key] = f"<{type(value).__name__}: {len(value)} chars>"
            else:
                result[key] = extract_structure(value, max_depth, current_depth + 1)
        return result
    elif isinstance(obj, list):
        if len(obj) == 0:
            return []
        elif len(obj) == 1:
            return [extract_structure(obj[0], max_depth, current_depth + 1)]
        else:
            return [
                extract_structure(obj[0], max_depth, current_depth + 1),
                f"... {len(obj)-1} more items"
            ]
    else:
        return obj

def main():
    if not ANTHROPIC_API_KEY or not OPENAI_API_KEY:
        print("❌ Missing API keys")
        return
    
    # Make requests in parallel
    with ThreadPoolExecutor(max_workers=2) as executor:
        anthropic_future = executor.submit(make_anthropic_request)
        openai_future = executor.submit(make_openai_request)
        
        anthropic_response = anthropic_future.result()
        openai_response = openai_future.result()
    
    print("=" * 80)
    print("🤖 ANTHROPIC RESPONSE STRUCTURE")
    print("=" * 80)
    print(yaml.dump(extract_structure(anthropic_response), default_flow_style=False, width=120))
    
    print("=" * 80)
    print("🤖 OPENAI RESPONSE STRUCTURE") 
    print("=" * 80)
    print(yaml.dump(extract_structure(openai_response), default_flow_style=False, width=120))

if __name__ == "__main__":
    main()