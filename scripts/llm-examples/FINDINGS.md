# Key Structural Differences Between Providers

Based on actual API testing, here are the confirmed structural differences:

## Response Structure Patterns

### Anthropic
- **Root Structure**: Single message with `content` array
- **Content Blocks**: `List[ContentBlock]` - flat structure
- **Tool Pattern**: `server_tool_use` → `web_search_tool_result`
- **Keys**: `id, type, role, model, content, stop_reason, usage` (8 keys)

### OpenAI  
- **Root Structure**: Response object with `output` array
- **Content Blocks**: `List[OutputBlock]` - nested with sources
- **Tool Pattern**: `web_search_call` with embedded sources
- **Keys**: `id, object, created_at, status, model, output, usage, ...` (20+ keys)

## Specific Examples

### Simple Text Response
```
Anthropic: content[0].type = "text"
OpenAI:    output[0].type = "message"
```

### Web Search Response
```
Anthropic: 
- content[0].type = "server_tool_use" 
- content[1].type = "web_search_tool_result"
- content[2].type = "text" (final response)

OpenAI:
- output[0].type = "web_search_call"
- output[1].type = "message" (may be missing)
```

### Thinking/Reasoning
```
Anthropic: content[0].type = "thinking" (when enabled)
OpenAI:    output[0].type = "reasoning" + reasoning.tokens_used
```

## Key Insights for LLMIR

1. **Content vs Output**: Anthropic uses `content`, OpenAI uses `output`
2. **Flat vs Nested**: Anthropic has flatter content blocks, OpenAI has nested structures
3. **Tool Results**: Anthropic exposes full tool results, OpenAI may omit final synthesis
4. **Metadata**: OpenAI provides much more response metadata
5. **Error Handling**: Both return structured errors but in different formats

## Test Cases Summary

- ✅ **simple_text**: Both providers work identically for basic text
- ✅ **web_search_simple**: Both work but very different structures  
- ⚠️ **thinking_enabled**: OpenAI works, Anthropic has config issues
- ❌ **multimodal_image**: Not yet tested

This framework captures exact input/output shapes for implementing universal translations.