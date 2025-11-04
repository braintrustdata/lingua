package lingua

import (
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
	assertJSONEqual(t, chatMsgs, linguaMsgs, "Conversion to Lingua should preserve Chat Completions message")

	// Test converting back to Chat Completions format
	backToChat, err := LinguaToChatCompletionsMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, backToChat, 1)
	assertJSONEqual(t, chatMsgs, backToChat, "Round-trip conversion should preserve Chat Completions message")
}

func TestAnthropicConversion(t *testing.T) {
	// Test converting Anthropic format to Lingua
	anthropicMsgs := []map[string]any{
		{
			"role": "user",
			"content": []map[string]any{
				{"type": "text", "text": "Hello"},
				{"type": "text", "text": "World"},
			},
		},
	}

	linguaMsgs, err := AnthropicMessagesToLingua(anthropicMsgs)
	require.NoError(t, err)
	require.Len(t, linguaMsgs, 1)
	expectedLingua := []map[string]any{
		{
			"role": "user",
			"content": []map[string]any{
				{"type": "text", "text": "Hello"},
				{"type": "text", "text": "World"},
			},
		},
	}
	assertJSONEqual(t, expectedLingua, linguaMsgs, "Conversion to Lingua should preserve message data")

	// Test converting back to Anthropic format
	backToAnthropic, err := LinguaToAnthropicMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, backToAnthropic, 1)

	assertJSONEqual(t, anthropicMsgs, backToAnthropic, "Round-trip preserves original Anthropic message")
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
	expectedLingua := []map[string]any{
		{"role": "user", "content": "What is the weather?"},
		{"role": "assistant", "content": "I don't have access to real-time weather data.", "id": nil},
	}

	assertJSONEqual(t, expectedLingua, linguaMsgs, "Chat Completions -> Lingua conversion should preserve message data")

	// Lingua -> Anthropic
	anthropicMsgs, err := LinguaToAnthropicMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, anthropicMsgs, 2)

	expectedAnthropic := []map[string]any{
		{
			"role":    "user",
			"content": "What is the weather?",
		},
		{
			"role": "assistant",
			"content": []map[string]any{
				{"type": "text", "text": "I don't have access to real-time weather data."},
			},
		},
	}

	assertJSONEqual(t, expectedAnthropic, anthropicMsgs, "Lingua -> Anthropic conversion should preserve message data")
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

	expectedDeduplicated := []map[string]any{
		{"role": "user", "content": "Hello"},
		{"role": "assistant", "content": "Hi there!", "id": nil},
	}

	assertJSONEqual(t, expectedDeduplicated, deduplicated, "DeduplicateMessages removes duplicate content")
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

	expectedMessages := []map[string]any{
		{"role": "user", "content": "Hello"},
		{"role": "assistant", "content": "Hi there", "id": nil},
		{"role": "assistant", "content": "Hi there", "id": nil},
	}

	assertJSONEqual(t, expectedMessages, messages, "ImportMessagesFromSpans should import all messages in order")

	deduplicated, err := ImportAndDeduplicateMessages(spans)
	require.NoError(t, err)
	require.Len(t, deduplicated, 2)

	expectedDeduplicated := []map[string]any{
		{"role": "user", "content": "Hello"},
		{"role": "assistant", "content": "Hi there", "id": nil},
	}

	assertJSONEqual(t, expectedDeduplicated, deduplicated, "ImportAndDeduplicateMessages should dedupe imported messages")
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

	expectedLingua := []map[string]any{
		{
			"role": "user",
			"content": []map[string]any{
				{"type": "text", "text": "What's in this image?"},
				{
					"type":             "image",
					"image":            "https://example.com/image.jpg",
					"media_type":       "image/url",
					"provider_options": nil,
				},
			},
		},
	}

	assertJSONEqual(t, expectedLingua, linguaMsgs, "Conversion to Lingua should preserve multimodal content")

	backToChat, err := LinguaToChatCompletionsMessages(linguaMsgs)
	require.NoError(t, err)
	require.Len(t, backToChat, 1)
	assertJSONEqual(t, chatMsgs, backToChat, "Round-trip conversion should preserve multimodal Chat Completions message")
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

	assertJSONEqual(t, original, result1, "Round-trip should preserve data")
}
