#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "python-dotenv>=1.0.0",
# ]
# ///

"""
Analyze structural differences between provider responses.
"""

import json
import sys
from pathlib import Path
from typing import Dict, Any, List

def extract_structure(obj: Any, max_depth: int = 6, current_depth: int = 0) -> Any:
    """Extract structure of an object, showing full content."""
    if current_depth >= max_depth:
        return obj  # Show full content instead of "..."
    
    if isinstance(obj, dict):
        result = {}
        for key, value in obj.items():
            # Only truncate extremely long encrypted content
            if isinstance(value, str) and key == 'encrypted_content' and len(value) > 500:
                result[key] = f"<{len(value)} chars encrypted>"
            else:
                result[key] = extract_structure(value, max_depth, current_depth + 1)
        return result
    
    elif isinstance(obj, list):
        # Show all items in lists, don't truncate
        return [extract_structure(item, max_depth, current_depth + 1) for item in obj]
    
    else:
        return obj

def analyze_response_structure(response: Dict[str, Any]) -> Dict[str, Any]:
    """Analyze the structure of a provider response."""
    # Handle string responses (error cases)
    if "response" in response and isinstance(response["response"], str):
        return {
            "top_level_keys": ["error_string"],
            "structure": {"error_string": response["response"][:200] + "..." if len(response["response"]) > 200 else response["response"]},
            "is_error": True
        }
    
    analysis = {
        "top_level_keys": list(response.get("response", {}).keys()) if "response" in response else [],
        "structure": extract_structure(response.get("response", {})),
        "is_error": False
    }
    
    # Provider-specific analysis
    if "response" in response and isinstance(response["response"], dict):
        resp = response["response"]
        
        # Anthropic-specific
        if "content" in resp and isinstance(resp["content"], list):
            content_types = [item.get("type", "unknown") for item in resp["content"]]
            analysis["anthropic"] = {
                "content_blocks": len(resp["content"]),
                "content_types": content_types,
                "has_tool_use": "server_tool_use" in content_types,
                "has_tool_results": any("tool_result" in t for t in content_types)
            }
        
        # OpenAI-specific
        if "output" in resp and isinstance(resp["output"], list):
            output_types = [item.get("type", "unknown") for item in resp["output"]]
            analysis["openai"] = {
                "output_blocks": len(resp["output"]),
                "output_types": output_types,
                "has_reasoning": "reasoning" in output_types,
                "has_web_search": "web_search_call" in output_types
            }
    
    return analysis

def compare_structures(results: Dict[str, Any]):
    """Compare structures across providers."""
    print("=" * 80)
    print("🔍 STRUCTURAL ANALYSIS")
    print("=" * 80)
    
    for provider, result in results.items():
        print(f"\n📋 {provider.upper()}")
        print("-" * 40)
        
        if "error" in result:
            print(f"❌ Error: {result['error']}")
            continue
        
        analysis = analyze_response_structure(result)
        
        if analysis.get("is_error", False):
            print("❌ API Error Response")
            print(f"Error content: {analysis['structure']['error_string']}")
            continue
        
        print(f"Top-level keys: {', '.join(analysis['top_level_keys'])}")
        
        if provider == "anthropic" and "anthropic" in analysis:
            a = analysis["anthropic"]
            print(f"Content blocks: {a['content_blocks']}")
            print(f"Content types: {', '.join(a['content_types'])}")
            print(f"Tool use: {a['has_tool_use']}")
            print(f"Tool results: {a['has_tool_results']}")
        
        elif provider == "openai" and "openai" in analysis:
            o = analysis["openai"]
            print(f"Output blocks: {o['output_blocks']}")
            print(f"Output types: {', '.join(o['output_types'])}")
            print(f"Reasoning: {o['has_reasoning']}")
            print(f"Web search: {o['has_web_search']}")
    
    print("\n" + "=" * 80)
    print("🆚 STRUCTURAL DIFFERENCES")
    print("=" * 80)
    
    providers = list(results.keys())
    if len(providers) >= 2:
        for i, provider1 in enumerate(providers):
            for provider2 in providers[i+1:]:
                if "error" in results[provider1] or "error" in results[provider2]:
                    continue
                
                print(f"\n{provider1.upper()} vs {provider2.upper()}:")
                
                a1 = analyze_response_structure(results[provider1])
                a2 = analyze_response_structure(results[provider2])
                
                # Compare top-level structure
                keys1 = set(a1["top_level_keys"])
                keys2 = set(a2["top_level_keys"])
                
                if keys1 != keys2:
                    only_in_1 = keys1 - keys2
                    only_in_2 = keys2 - keys1
                    if only_in_1:
                        print(f"  Only in {provider1}: {', '.join(only_in_1)}")
                    if only_in_2:
                        print(f"  Only in {provider2}: {', '.join(only_in_2)}")
                else:
                    print("  ✅ Same top-level keys")

def print_structure(results: Dict[str, Any]):
    """Print detailed structure for each provider."""
    print("=" * 80)
    print("🏗️  DETAILED STRUCTURES")
    print("=" * 80)
    
    for provider, result in results.items():
        print(f"\n📋 {provider.upper()} STRUCTURE")
        print("-" * 40)
        
        if "error" in result:
            print(f"❌ Error: {result['error']}")
            continue
        
        analysis = analyze_response_structure(result)
        
        if analysis.get("is_error", False):
            print("❌ API Error Response")
            print(f"Error: {analysis['structure']['error_string']}")
            continue
        
        structure = analysis["structure"]
        
        # Pretty print structure
        print(json.dumps(structure, indent=2, default=str))

def main():
    if len(sys.argv) < 2:
        print("Usage: python analyzer.py <snapshot_file>")
        print("Example: python analyzer.py ../snapshots/web_search_simple_20250831_143022.json")
        sys.exit(1)
    
    snapshot_path = sys.argv[1]
    if not Path(snapshot_path).exists():
        print(f"❌ Snapshot file not found: {snapshot_path}")
        sys.exit(1)
    
    with open(snapshot_path, 'r') as f:
        data = json.load(f)
    
    print(f"📸 Analyzing snapshot: {Path(snapshot_path).name}")
    print(f"🎯 Case: {data['case']['name']} - {data['case']['description']}")
    print(f"⏰ Timestamp: {data['timestamp']}")
    
    results = data["results"]
    
    # Run analysis
    compare_structures(results)
    print_structure(results)

if __name__ == "__main__":
    main()