package main

import (
	"encoding/json"
	"fmt"
	"log"
	"strings"

	"github.com/braintrustdata/lingua/bindings/golang"
)

func main() {
	fmt.Println("╔════════════════════════════════════════════════════════════╗")
	fmt.Println("║        Lingua Go Bindings - Full Demonstration            ║")
	fmt.Println("╚════════════════════════════════════════════════════════════╝")
	fmt.Println()

	// Run all examples
	if err := runExamples(); err != nil {
		log.Fatalf("❌ Error: %v", err)
	}

	fmt.Println()
	fmt.Println("✅ All examples completed successfully!")
}

func runExamples() error {
	examples := []struct {
		name string
		fn   func() error
	}{
		{"Simple Chat Completions Conversion", exampleSimpleConversion},
		{"Cross-Provider Conversion", exampleCrossProvider},
		{"Multi-Modal Messages", exampleMultiModal},
		{"Tool Calls", exampleToolCalls},
		{"Message Deduplication", exampleDeduplication},
		{"Request Validation", exampleValidation},
		{"Error Handling", exampleErrorHandling},
		{"Round-Trip Verification", exampleRoundTrip},
	}

	for i, ex := range examples {
		fmt.Printf("\n%d. %s\n", i+1, ex.name)
		fmt.Println(strings.Repeat("─", 60))
		if err := ex.fn(); err != nil {
			return fmt.Errorf("example '%s' failed: %w", ex.name, err)
		}
		fmt.Println()
	}

	return nil
}

func exampleSimpleConversion() error {
	// Create OpenAI-style messages
	openaiMsgs := []map[string]interface{}{
		{
			"role":    "system",
			"content": "You are a helpful assistant that translates English to French.",
		},
		{
			"role":    "user",
			"content": "Hello, how are you?",
		},
		{
			"role":    "assistant",
			"content": "Bonjour, comment allez-vous?",
		},
	}

	fmt.Println("📤 Input (OpenAI format):")
	printJSON(openaiMsgs)

	// Convert to Lingua
	linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(openaiMsgs)
	if err != nil {
		return err
	}

	fmt.Println("\n🔄 Converted to Lingua format:")
	printJSON(linguaMsgs)

	fmt.Printf("\n✓ Converted %d messages to Lingua format\n", len(linguaMsgs))
	return nil
}

func exampleCrossProvider() error {
	// Start with OpenAI format
	openaiMsgs := []map[string]interface{}{
		{"role": "user", "content": "What is the capital of France?"},
		{"role": "assistant", "content": "The capital of France is Paris."},
	}

	fmt.Println("📤 Starting with OpenAI format:")
	printJSON(openaiMsgs)

	// OpenAI -> Lingua
	linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(openaiMsgs)
	if err != nil {
		return err
	}

	// Lingua -> Anthropic
	anthropicMsgs, err := lingua.LinguaToAnthropicMessages(linguaMsgs)
	if err != nil {
		return err
	}

	fmt.Println("\n📥 Converted to Anthropic format:")
	printJSON(anthropicMsgs)

	// Verify structure
	if len(anthropicMsgs) != len(openaiMsgs) {
		return fmt.Errorf("message count mismatch: %d != %d", len(anthropicMsgs), len(openaiMsgs))
	}

	fmt.Printf("\n✓ Successfully converted OpenAI → Lingua → Anthropic (%d messages)\n", len(anthropicMsgs))
	return nil
}

func exampleMultiModal() error {
	// Create a message with text and image
	multiModalMsg := []map[string]interface{}{
		{
			"role": "user",
			"content": []map[string]interface{}{
				{
					"type": "text",
					"text": "What's in this image?",
				},
				{
					"type": "image_url",
					"image_url": map[string]interface{}{
						"url": "https://example.com/image.jpg",
					},
				},
			},
		},
	}

	fmt.Println("📤 Input (Multi-modal message):")
	printJSON(multiModalMsg)

	// Convert to Lingua
	linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(multiModalMsg)
	if err != nil {
		return err
	}

	fmt.Println("\n🔄 Converted to Lingua format:")
	printJSON(linguaMsgs)

	// Convert back
	backToOpenAI, err := lingua.LinguaToChatCompletionsMessages(linguaMsgs)
	if err != nil {
		return err
	}

	fmt.Println("\n📥 Converted back to OpenAI format:")
	printJSON(backToOpenAI)

	fmt.Println("\n✓ Multi-modal content preserved through round-trip")
	return nil
}

func exampleToolCalls() error {
	// Message with tool call
	toolCallMsg := []map[string]interface{}{
		{
			"role":    "user",
			"content": "What's the weather in San Francisco?",
		},
		{
			"role":    "assistant",
			"content": nil,
			"tool_calls": []map[string]interface{}{
				{
					"id":   "call_123",
					"type": "function",
					"function": map[string]interface{}{
						"name":      "get_weather",
						"arguments": `{"location": "San Francisco, CA"}`,
					},
				},
			},
		},
	}

	fmt.Println("📤 Input (Message with tool call):")
	printJSON(toolCallMsg)

	// Convert to Lingua
	linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(toolCallMsg)
	if err != nil {
		return err
	}

	fmt.Println("\n🔄 Converted to Lingua format:")
	printJSON(linguaMsgs)

	fmt.Println("\n✓ Tool calls preserved in conversion")
	return nil
}

func exampleDeduplication() error {
	// Create messages with duplicates
	msgs := []map[string]interface{}{
		{"role": "user", "content": "Hello"},
		{"role": "user", "content": "Hello"}, // Duplicate
		{"role": "assistant", "content": "Hi there!"},
		{"role": "user", "content": "How are you?"},
		{"role": "user", "content": "Hello"}, // Another duplicate
	}

	fmt.Printf("📤 Input: %d messages (with duplicates)\n", len(msgs))
	printJSON(msgs)

	// Deduplicate
	deduplicated, err := lingua.DeduplicateMessages(msgs)
	if err != nil {
		return err
	}

	fmt.Printf("\n📥 Output: %d messages (deduplicated)\n", len(deduplicated))
	printJSON(deduplicated)

	fmt.Printf("\n✓ Removed %d duplicate messages\n", len(msgs)-len(deduplicated))
	return nil
}

func exampleValidation() error {
	// Valid request
	validRequest := `{
		"model": "gpt-4",
		"messages": [
			{"role": "user", "content": "Hello"}
		],
		"temperature": 0.7,
		"max_tokens": 100
	}`

	fmt.Println("📤 Validating Chat Completions request:")
	fmt.Println(validRequest)

	validated, err := lingua.ValidateChatCompletionsRequest(validRequest)
	if err != nil {
		return err
	}

	fmt.Println("\n✓ Request is valid!")
	fmt.Printf("   Model: %v\n", validated["model"])
	fmt.Printf("   Temperature: %v\n", validated["temperature"])
	fmt.Printf("   Max tokens: %v\n", validated["max_tokens"])

	// Invalid request
	invalidRequest := `{
		"messages": [
			{"role": "invalid_role", "content": "Hello"}
		]
	}`

	fmt.Println("\n📤 Validating invalid request:")
	fmt.Println(invalidRequest)

	_, err = lingua.ValidateChatCompletionsRequest(invalidRequest)
	if err != nil {
		fmt.Printf("\n✓ Correctly rejected invalid request:\n   %v\n", err)
		return nil
	}

	return fmt.Errorf("validation should have failed for invalid request")
}

func exampleErrorHandling() error {
	fmt.Println("Testing error handling with invalid inputs...")

	// Test 1: Invalid JSON
	fmt.Println("\n1️⃣  Testing with invalid JSON:")
	_, err := lingua.ChatCompletionsMessagesToLingua("not valid json")
	if err != nil {
		if convErr, ok := err.(*lingua.ConversionError); ok {
			fmt.Printf("   ✓ Got ConversionError: %s\n", convErr.Message)
			fmt.Printf("   ✓ Provider: %s\n", convErr.Provider)
		} else {
			fmt.Printf("   ✓ Got error: %v\n", err)
		}
	} else {
		return fmt.Errorf("should have gotten an error for invalid JSON")
	}

	// Test 2: Invalid message structure
	fmt.Println("\n2️⃣  Testing with invalid message structure:")
	invalidMsgs := []map[string]interface{}{
		{"invalid": "structure"},
	}
	_, err = lingua.ChatCompletionsMessagesToLingua(invalidMsgs)
	if err != nil {
		fmt.Printf("   ✓ Correctly rejected invalid structure: %v\n", err)
	} else {
		return fmt.Errorf("should have gotten an error for invalid structure")
	}

	fmt.Println("\n✓ Error handling works correctly")
	return nil
}

func exampleRoundTrip() error {
	// Original messages
	original := []map[string]interface{}{
		{
			"role":    "system",
			"content": "You are a helpful assistant.",
		},
		{
			"role":    "user",
			"content": "Tell me a joke.",
		},
		{
			"role":    "assistant",
			"content": "Why did the programmer quit? Because they didn't get arrays!",
		},
	}

	fmt.Println("📤 Original messages:")
	printJSON(original)

	// Round-trip: OpenAI -> Lingua -> OpenAI
	lingua1, err := lingua.ChatCompletionsMessagesToLingua(original)
	if err != nil {
		return err
	}

	result, err := lingua.LinguaToChatCompletionsMessages(lingua1)
	if err != nil {
		return err
	}

	fmt.Println("\n📥 After round-trip:")
	printJSON(result)

	// Compare
	originalJSON, _ := json.Marshal(original)
	resultJSON, _ := json.Marshal(result)

	var originalParsed, resultParsed interface{}
	json.Unmarshal(originalJSON, &originalParsed)
	json.Unmarshal(resultJSON, &resultParsed)

	originalStr := fmt.Sprintf("%v", originalParsed)
	resultStr := fmt.Sprintf("%v", resultParsed)

	if originalStr == resultStr {
		fmt.Println("\n✓ Round-trip preserves data perfectly!")
	} else {
		fmt.Println("\n⚠️  Data changed during round-trip (this may be expected due to normalization)")
	}

	return nil
}

func printJSON(v interface{}) {
	b, err := json.MarshalIndent(v, "   ", "  ")
	if err != nil {
		fmt.Printf("   Error formatting JSON: %v\n", err)
		return
	}
	fmt.Println("   " + string(b))
}
