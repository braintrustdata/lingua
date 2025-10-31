package lingua

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestChatCompletionsConversion(t *testing.T) {
	// Test converting Chat Completions format to Lingua
	chatMsgs := []map[string]any{
		{"role": "user", "content": "Hello, how are you?"},
	}

	linguaMsgs, err := ChatCompletionsMessagesToLingua(chatMsgs)
	require.NoError(t, err)
	require.Len(t, linguaMsgs, 1)
	assert.Equal(t, "user", linguaMsgs[0]["role"])

	// Test converting back to Chat Completions format
	backToChat, err := LinguaToChatCompletionsMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, backToChat, 1)
	assert.Equal(t, "user", backToChat[0]["role"])
}

func TestAnthropicConversion(t *testing.T) {
	// Test converting Anthropic format to Lingua
	anthropicMsgs := []map[string]any{
		{
			"role": "user",
			"content": []map[string]any{
				{"type": "text", "text": "Hello"},
			},
		},
	}

	linguaMsgs, err := AnthropicMessagesToLingua(anthropicMsgs)
	require.NoError(t, err)
	require.Len(t, linguaMsgs, 1)
	assert.Equal(t, "user", linguaMsgs[0]["role"])

	// Test converting back to Anthropic format
	backToAnthropic, err := LinguaToAnthropicMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, backToAnthropic, 1)
	assert.Equal(t, "user", backToAnthropic[0]["role"])
}

func TestCrossProviderConversion(t *testing.T) {
	// Test converting from Chat Completions to Anthropic via Lingua
	chatMsgs := []map[string]any{
		{"role": "user", "content": "What is the weather?"},
		{"role": "assistant", "content": "I don't have access to real-time weather data."},
	}

	// Chat Completions -> Lingua
	linguaMsgs, err := ChatCompletionsMessagesToLingua(chatMsgs)
	require.NoError(t, err)
	require.Len(t, linguaMsgs, 2)

	// Lingua -> Anthropic
	anthropicMsgs, err := LinguaToAnthropicMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, anthropicMsgs, 2)
	assert.Equal(t, "user", anthropicMsgs[0]["role"])
	assert.Equal(t, "assistant", anthropicMsgs[1]["role"])
}

func TestDeduplicateMessages(t *testing.T) {
	// Test deduplication with duplicate messages
	messages := []map[string]any{
		{"role": "user", "content": "Hello"},
		{"role": "user", "content": "Hello"}, // Duplicate
		{"role": "assistant", "content": "Hi there!"},
	}

	deduplicated, err := DeduplicateMessages(messages)
	require.NoError(t, err)
	assert.Len(t, deduplicated, 2, "Should remove duplicate message")
	assert.Equal(t, "user", deduplicated[0]["role"])
	assert.Equal(t, "assistant", deduplicated[1]["role"])
}

func TestImportMessagesFromSpans(t *testing.T) {
	spans := []map[string]any{
		{
			"input": []map[string]any{
				{"role": "user", "content": "Hello"},
			},
			"output": []map[string]any{
				{"role": "assistant", "content": "Hi there"},
			},
		},
		{
			"output": []map[string]any{
				{"role": "assistant", "content": "Hi there"},
			},
		},
	}

	messages, err := ImportMessagesFromSpans(spans)
	require.NoError(t, err)
	require.Len(t, messages, 3)
	assert.Equal(t, "user", messages[0]["role"])
	assert.Equal(t, "assistant", messages[1]["role"])

	deduplicated, err := ImportAndDeduplicateMessages(spans)
	require.NoError(t, err)
	require.Len(t, deduplicated, 2)
	assert.Equal(t, "user", deduplicated[0]["role"])
	assert.Equal(t, "assistant", deduplicated[1]["role"])
}

func TestValidateChatCompletionsRequest(t *testing.T) {
	validRequest := `{
		"model": "gpt-4",
		"messages": [
			{"role": "user", "content": "Hello"}
		]
	}`

	result, err := ValidateChatCompletionsRequest(validRequest)
	require.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "gpt-4", result["model"])
}

func TestValidateChatCompletionsRequestInvalid(t *testing.T) {
	invalidRequest := `{
		"messages": [
			{"role": "invalid_role", "content": "Hello"}
		]
	}`

	_, err := ValidateChatCompletionsRequest(invalidRequest)
	assert.Error(t, err, "Should fail validation for invalid request")
}

func TestValidateAnthropicRequest(t *testing.T) {
	validRequest := `{
		"model": "claude-3-5-sonnet-20241022",
		"max_tokens": 1024,
		"messages": [
			{
				"role": "user",
				"content": [
					{"type": "text", "text": "Hello"}
				]
			}
		]
	}`

	result, err := ValidateAnthropicRequest(validRequest)
	require.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "claude-3-5-sonnet-20241022", result["model"])
}

func TestConversionError(t *testing.T) {
	// Test with invalid JSON
	invalidMsgs := "not valid json"

	_, err := ChatCompletionsMessagesToLingua(invalidMsgs)
	require.Error(t, err)

	var convErr *ConversionError
	require.True(t, errors.As(err, &convErr), "Error should be ConversionError type")
	assert.Equal(t, "Chat Completions", convErr.Provider)
	// The error could be from marshaling or parsing
	assert.True(t,
		strings.Contains(convErr.Message, "failed to marshal input") ||
			strings.Contains(convErr.Message, "Failed to parse input JSON"),
		"Error message should mention marshaling or parsing failure")
}

func TestComplexMessageContent(t *testing.T) {
	// Test message with array content
	chatMsgs := []map[string]any{
		{
			"role": "user",
			"content": []map[string]any{
				{"type": "text", "text": "What's in this image?"},
				{
					"type": "image_url",
					"image_url": map[string]any{
						"url": "https://example.com/image.jpg",
					},
				},
			},
		},
	}

	linguaMsgs, err := ChatCompletionsMessagesToLingua(chatMsgs)
	require.NoError(t, err)
	require.Len(t, linguaMsgs, 1)

	// Verify content structure is preserved
	content := linguaMsgs[0]["content"]
	assert.NotNil(t, content)
}

func TestRoundTripPreservesData(t *testing.T) {
	// Test that round-trip conversion preserves message data
	original := []map[string]any{
		{
			"role":    "user",
			"content": "Test message",
		},
		{
			"role":    "assistant",
			"content": "Test response",
		},
	}

	// Chat -> Lingua -> Chat
	lingua1, err := ChatCompletionsMessagesToLingua(original)
	require.NoError(t, err)

	result1, err := LinguaToChatCompletionsMessages(lingua1)
	require.NoError(t, err)

	// Compare as JSON to handle type differences
	originalJSON, err := json.Marshal(original)
	require.NoError(t, err)
	resultJSON, err := json.Marshal(result1)
	require.NoError(t, err)

	var originalParsed, resultParsed any
	require.NoError(t, json.Unmarshal(originalJSON, &originalParsed))
	require.NoError(t, json.Unmarshal(resultJSON, &resultParsed))

	assert.Equal(t, originalParsed, resultParsed, "Round-trip should preserve data")
}
