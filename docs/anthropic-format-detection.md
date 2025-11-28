# Anthropic Format Detection

Detects if a payload is already in Anthropic format (vs OpenAI format that needs translation).

## Usage

```rust
use lingua::providers::anthropic::is_anthropic_format;

if is_anthropic_format(payload_str)? {
    // Send directly to Anthropic - no translation needed
} else {
    // Translate from OpenAI format first
}
```

## Key Differences: Anthropic vs OpenAI

| Feature | Anthropic | OpenAI |
|---------|-----------|--------|
| `max_tokens` | Required | Optional |
| Message roles | `user`, `assistant` only | `system`, `user`, `assistant`, `tool` |
| System prompt | Top-level `system` field | `system` role message |
| Tool results | `tool_result` block in user message | Separate `tool` role message |
| Images | `source: {type: "base64", data, media_type}` | `image_url: {url}` |
| Content blocks | `tool_use`, `tool_result`, `thinking` | Different structure |

## Detection Logic

1. Try deserializing as Anthropic `CreateMessageParams`
2. Verify `max_tokens` is present (required)
3. Check roles are only `user`/`assistant`
4. Look for Anthropic-specific content types (`tool_use`, `tool_result`, `thinking`)

