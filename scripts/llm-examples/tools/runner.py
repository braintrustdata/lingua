#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "requests>=2.31.0",
#     "python-dotenv>=1.0.0",
# ]
# ///

"""
Universal LLM test case runner.

Runs test cases against multiple providers and captures structured responses.
"""

import json
import os
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path
from typing import Dict, Any, List, Optional

import requests
from dotenv import load_dotenv

load_dotenv()

class LLMRunner:
    def __init__(self):
        self.anthropic_key = os.getenv("ANTHROPIC_API_KEY")
        self.openai_key = os.getenv("OPENAI_API_KEY")
        self.google_key = os.getenv("GOOGLE_API_KEY")
        
    def load_case(self, case_path: str) -> Dict[str, Any]:
        """Load a test case from JSON file."""
        with open(case_path, 'r') as f:
            return json.load(f)
    
    def make_anthropic_request(self, case: Dict[str, Any]) -> Dict[str, Any]:
        """Make request to Anthropic API."""
        if not self.anthropic_key:
            return {"error": "ANTHROPIC_API_KEY not found"}
        
        headers = {
            "Content-Type": "application/json",
            "x-api-key": self.anthropic_key,
            "anthropic-version": "2023-06-01"
        }
        
        # Build messages
        messages = []
        prompt = case["prompt"]
        
        if isinstance(prompt, str):
            messages.append({"role": "user", "content": prompt})
        elif isinstance(prompt, list):
            # Handle multimodal content
            content = []
            for item in prompt:
                if item["type"] == "text":
                    content.append({"type": "text", "text": item["text"]})
                elif item["type"] == "image_url":
                    # Convert to Anthropic format
                    url = item["image_url"]["url"]
                    
                    if url.startswith("data:"):
                        # Handle base64 data URLs
                        media_type = url.split(";")[0].replace("data:", "")
                        data = url.split(",")[1]
                        content.append({
                            "type": "image", 
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data
                            }
                        })
                    else:
                        # Handle remote URLs - Anthropic doesn't support remote URLs directly
                        # For now, skip remote images for Anthropic
                        pass
            messages.append({"role": "user", "content": content})
        
        # Build payload
        payload = {
            "model": case["models"]["anthropic"],
            "max_tokens": case["max_tokens"],
            "messages": messages
        }
        
        # Add tools if present
        if case["tools"]:
            tools = []
            for tool in case["tools"]:
                if tool["type"] == "web_search":
                    config = tool["provider_configs"]["anthropic"]
                    tools.append(config)
            payload["tools"] = tools
        
        # Add provider-specific configs
        if "provider_configs" in case and "anthropic" in case["provider_configs"]:
            config = case["provider_configs"]["anthropic"]
            if "thinking" in config:
                payload["thinking"] = config["thinking"]
        
        try:
            response = requests.post(
                "https://api.anthropic.com/v1/messages",
                headers=headers,
                json=payload,
                timeout=120
            )
            return {
                "status_code": response.status_code,
                "response": response.json() if response.status_code == 200 else response.text
            }
        except Exception as e:
            return {"error": str(e)}
    
    def make_openai_request(self, case: Dict[str, Any]) -> Dict[str, Any]:
        """Make request to OpenAI API."""
        if not self.openai_key:
            return {"error": "OPENAI_API_KEY not found"}
        
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.openai_key}"
        }
        
        # Build input
        input_messages = []
        prompt = case["prompt"]
        
        if isinstance(prompt, str):
            input_messages.append({"role": "user", "content": prompt})
        elif isinstance(prompt, list):
            # Handle multimodal content
            content = []
            for item in prompt:
                if item["type"] == "text":
                    content.append({"type": "input_text", "text": item["text"]})
                elif item["type"] == "image_url":
                    content.append({
                        "type": "input_image",
                        "image_url": item["image_url"]["url"]
                    })
            input_messages.append({"role": "user", "content": content})
        
        # Build payload for Responses API
        payload = {
            "model": case["models"]["openai"],
            "input": input_messages,
            "max_output_tokens": case["max_tokens"]
        }
        
        # Add tools if present
        if case["tools"]:
            tools = []
            for tool in case["tools"]:
                if tool["type"] == "web_search":
                    config = tool["provider_configs"]["openai"]
                    tools.append(config)
            payload["tools"] = tools
        
        # Add provider-specific configs
        if "provider_configs" in case and "openai" in case["provider_configs"]:
            config = case["provider_configs"]["openai"]
            if "reasoning" in config:
                payload["reasoning"] = config["reasoning"]
        
        try:
            response = requests.post(
                "https://api.openai.com/v1/responses",
                headers=headers,
                json=payload,
                timeout=120
            )
            return {
                "status_code": response.status_code,
                "response": response.json() if response.status_code == 200 else response.text
            }
        except Exception as e:
            return {"error": str(e)}
    
    def run_case(self, case_path: str) -> Dict[str, Any]:
        """Run a single test case against all providers."""
        case = self.load_case(case_path)
        
        print(f"Running case: {case['name']} - {case['description']}")
        
        results = {
            "case": case,
            "timestamp": datetime.now().isoformat(),
            "results": {}
        }
        
        # Run requests in parallel
        with ThreadPoolExecutor(max_workers=3) as executor:
            futures = {}
            
            if "anthropic" in case["models"]:
                futures[executor.submit(self.make_anthropic_request, case)] = "anthropic"
            
            if "openai" in case["models"]:
                futures[executor.submit(self.make_openai_request, case)] = "openai"
            
            for future in as_completed(futures):
                provider = futures[future]
                try:
                    result = future.result()
                    results["results"][provider] = result
                    print(f"  ✅ {provider}: {'Success' if 'error' not in result else 'Error'}")
                except Exception as e:
                    results["results"][provider] = {"error": str(e)}
                    print(f"  ❌ {provider}: {str(e)}")
        
        return results
    
    def save_snapshot(self, results: Dict[str, Any], case_name: str):
        """Save results as a snapshot and clean up old ones."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"{case_name}_{timestamp}.json"
        snapshots_dir = Path(__file__).parent.parent / "snapshots"
        snapshot_path = snapshots_dir / filename
        
        # Clean up old snapshots for this case
        old_snapshots = list(snapshots_dir.glob(f"{case_name}_*.json"))
        for old_snapshot in old_snapshots:
            try:
                old_snapshot.unlink()
                print(f"🗑️  Removed old snapshot: {old_snapshot.name}")
            except OSError:
                pass  # Ignore if file doesn't exist or can't be deleted
        
        # Save new snapshot
        with open(snapshot_path, 'w') as f:
            json.dump(results, f, indent=2, default=str)
        
        print(f"📸 Snapshot saved: {snapshot_path}")
        return snapshot_path

def main():
    if len(sys.argv) < 2:
        print("Usage: python runner.py <case_file>")
        print("Example: python runner.py ../cases/web_search_simple.json")
        sys.exit(1)
    
    case_path = sys.argv[1]
    if not os.path.exists(case_path):
        print(f"❌ Case file not found: {case_path}")
        sys.exit(1)
    
    runner = LLMRunner()
    results = runner.run_case(case_path)
    
    # Extract case name from path
    case_name = Path(case_path).stem
    runner.save_snapshot(results, case_name)

if __name__ == "__main__":
    main()