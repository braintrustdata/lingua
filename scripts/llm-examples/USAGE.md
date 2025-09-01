# Quick Start Guide

## 🚀 Simple 2-Command Workflow

```bash
# 1. See what's available
./list_cases.sh

# 2. Run and analyze any case 
./run_case.sh cases/simple_text.json
```

That's it! Everything else is automatic.

## 📋 All Available Commands

### `./list_cases.sh`
- Lists all available test cases
- Shows descriptions and models
- Provides exact usage commands

### `./run_case.sh cases/<case>.json` 
- Runs test against all configured providers
- Auto-removes old snapshots  
- Saves new results
- **Automatically analyzes and shows structural differences**

### `./summary.sh [case_name]`
- **No argument**: View summary of all test cases and their status
- **With case name**: Show detailed analysis of a specific case
  - Example: `./summary.sh simple_text`
  - Shows full structural differences and response content

### `./cleanup.sh`
- Manual cleanup of old snapshots (rarely needed since auto-cleanup)

## ✅ Ready-to-Use Test Cases

- **simple_text**: Basic text responses
- **web_search_simple**: Web search tool usage
- **thinking_comparison**: Anthropic thinking vs OpenAI basic
- **thinking_enabled**: Both providers with reasoning
- **multimodal_image**: Image processing (ready to test)

## 🎯 Adding New Cases

Create `cases/my_test.json`:
```json
{
  "name": "my_test",
  "description": "What this test does",
  "prompt": "Your prompt here",
  "models": {
    "anthropic": "claude-3-5-haiku-20241022", 
    "openai": "gpt-4o-mini"
  },
  "max_tokens": 500
}
```

Then run: `./run_case.sh cases/my_test.json`

## 🔍 What You Get

Each run shows you:
- **Success/failure** for each provider
- **Structural differences** between responses  
- **Content block types** and patterns
- **Complete response structures** (truncated for readability)

Perfect for understanding how to implement LLMIR universal translation!