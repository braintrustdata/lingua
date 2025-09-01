# LLM Examples Framework

A structured system for testing prompts + tools across multiple LLM providers and capturing response structures.

## Directory Structure

```
llm-examples/
├── cases/           # Test case definitions (JSON)
├── snapshots/       # Captured responses (JSON)
├── tools/           # Runner and analysis scripts
└── README.md        # This file
```

## Usage

### 1. List Available Cases

```bash
./list_cases.sh
```

### 2. Run and Analyze (One Command!)

```bash
# Run any test case - automatically runs and analyzes
./run_case.sh cases/simple_text.json
./run_case.sh cases/web_search_simple.json
./run_case.sh cases/thinking_comparison.json
```

This single command will:
- ✅ Run the test against all configured providers
- 🗑️ Clean up old snapshots  
- 📸 Save new snapshot
- 📊 Automatically analyze structural differences

### 3. Define Custom Test Cases

Create JSON files in `cases/` with this structure:

```json
{
  "name": "test_name",
  "description": "What this test does", 
  "prompt": "Your prompt here",
  "tools": [
    {
      "type": "web_search",
      "provider_configs": {
        "anthropic": {"type": "web_search_20250305", "name": "web_search"},
        "openai": {"type": "web_search_preview"}
      }
    }
  ],
  "models": {
    "anthropic": "claude-3-5-haiku-20241022",
    "openai": "gpt-4o-mini"
  },
  "max_tokens": 500,
  "provider_configs": {
    "anthropic": {"thinking": {"type": "enabled", "budget_tokens": 1024}},
    "openai": {"reasoning": {"effort": "medium", "summary": "auto"}}
  }
}
```

## Test Case Features

### Prompt Types
- **String**: Simple text prompt
- **Array**: Multimodal content (text + images)

### Tool Configurations
- Provider-specific tool configs
- Automatic translation between provider formats

### Provider-Specific Settings
- Anthropic: `thinking`, `context_caching`
- OpenAI: `reasoning_effort`, `verbosity`

### Models
- Specify different models per provider
- Automatic API endpoint selection

## Example Cases

- `web_search_simple.json` - Basic web search functionality
- `thinking_enabled.json` - Reasoning/thinking capabilities  
- `multimodal_image.json` - Image processing (future)

## Analysis Output

The analyzer provides:

1. **Structural Analysis**: Top-level keys, content block types
2. **Provider Differences**: What's unique to each provider
3. **Detailed Structure**: Full response structure (truncated)

## Snapshot Management

- **Auto-cleanup**: Each run automatically removes old snapshots for that test case
- **One snapshot per case**: Only the latest run is kept to save space
- **Manual cleanup**: Run `python3 tools/cleanup.py` to clean up all old snapshots

## Adding New Providers

1. Add API key to environment
2. Extend `LLMRunner` class in `runner.py`
3. Add provider configs to test cases
4. Update analyzer for provider-specific patterns

## Environment Setup

Required environment variables:
```bash
export ANTHROPIC_API_KEY=your_key
export OPENAI_API_KEY=your_key
export GOOGLE_API_KEY=your_key  # Future
```